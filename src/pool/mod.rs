mod task;
mod work_stealing;
mod worker;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use super::queue::TaskQueue;
use super::stealer::WorkStealingQueues;
use task::{spawn_task, BoxedTask, TaskHandle};
use worker::{worker_loop, WorkerHandle};

/// A configurable thread pool that can operate with a simple global queue or in work-stealing mode.
pub struct ThreadPool {
    running: Arc<AtomicBool>,
    workers: Vec<WorkerHandle>,
    fetch_task: Arc<dyn Fn(usize) -> Option<BoxedTask> + Send + Sync>,
    submit_task: Arc<dyn Fn(BoxedTask) + Send + Sync>,
    work_stealing: bool,
}

impl ThreadPool {
    pub fn new(num_threads: usize) -> Self {
        ThreadPoolBuilder::new().num_threads(num_threads).build()
    }

    pub fn spawn<F, T>(&self, f: F) -> TaskHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        let (task, handle) = spawn_task(f);
        (self.submit_task)(task);
        handle
    }

    pub fn shutdown(mut self) {
        self.running.store(false, Ordering::Release);
        for worker in &mut self.workers {
            worker.join();
        }
    }
}

pub struct ThreadPoolBuilder {
    num_threads: usize,
    work_stealing: bool,
}

impl ThreadPoolBuilder {
    pub fn new() -> Self {
        Self {
            num_threads: 4,
            work_stealing: false,
        }
    }

    pub fn num_threads(mut self, n: usize) -> Self {
        self.num_threads = n;
        self
    }

    pub fn work_stealing(mut self, enable: bool) -> Self {
        self.work_stealing = enable;
        self
    }

    pub fn build(self) -> ThreadPool {
        let running = Arc::new(AtomicBool::new(true));

        if self.work_stealing {
            // Setup work-stealing structures
            // Adjust your WorkStealingQueues::new to return these three things:
            // (Arc<Injector<T>>, Vec<Stealer<T>>, Vec<Worker<T>>)
            let (injector, stealers, mut workers_local) =
                WorkStealingQueues::new_vectors(self.num_threads);

            let running_clone = Arc::clone(&running);

            let submit_task = {
                let injector = Arc::clone(&injector);
                Arc::new(move |task: BoxedTask| {
                    injector.push(task);
                }) as Arc<dyn Fn(BoxedTask) + Send + Sync>
            };

            // We'll store stealers in an Arc so each thread can reference them
            let stealers = Arc::new(stealers);

            let mut workers = Vec::with_capacity(self.num_threads);

            for i in 0..self.num_threads {
                let r = Arc::clone(&running_clone);
                // Take a worker for this thread
                let worker = workers_local.remove(0);
                let injector = Arc::clone(&injector);
                let stealers_for_thread = Arc::clone(&stealers);

                // Per-thread fetch_task closure
                // This closure doesn't need to be Send+Sync because it will be moved into
                // the thread that uses it. It only needs to be 'static because the thread
                // may outlive the stack frame.
                let fetch_task = move || {
                    // First try to pop from this thread's worker
                    if let Some(task) = worker.pop() {
                        return Some(task);
                    }

                    // Try stealing a batch of tasks from the injector
                    match injector.steal_batch(&worker) {
                        crossbeam::deque::Steal::Success(t) => {
                            if let Some(task) = worker.pop() {
                                return Some(task);
                            }
                        }
                        crossbeam::deque::Steal::Empty | crossbeam::deque::Steal::Retry => {}
                    }

                    // Try stealing from other workers using their stealers
                    for st in stealers_for_thread.iter() {
                        match st.steal() {
                            crossbeam::deque::Steal::Success(t) => return Some(t),
                            _ => {}
                        }
                    }

                    None
                };

                let handle = std::thread::spawn(move || {
                    worker_loop(r, fetch_task);
                });
                workers.push(WorkerHandle::new(i, handle));
            }

            ThreadPool {
                running,
                workers,
                // fetch_task is no longer stored per pool in this mode
                // since each thread has its own closure.
                // Just provide a dummy closure or modify ThreadPool struct to make it optional.
                fetch_task: Arc::new(|_| None),
                submit_task,
                work_stealing: true,
            }
        } else {
            // Normal queue mode
            let queue = TaskQueue::new();
            let running_clone = Arc::clone(&running);

            let fetch_task = {
                let queue_clone = queue.clone_inner();
                Arc::new(move |_id: usize| queue_clone.pop())
                    as Arc<dyn Fn(usize) -> Option<BoxedTask> + Send + Sync>
            };

            let submit_task = {
                let queue_clone = queue.clone_inner();
                Arc::new(move |task: BoxedTask| {
                    queue_clone.push(task);
                }) as Arc<dyn Fn(BoxedTask) + Send + Sync>
            };

            let mut workers = Vec::with_capacity(self.num_threads);
            for i in 0..self.num_threads {
                let r = Arc::clone(&running_clone);
                let ft = Arc::clone(&fetch_task);
                let handle = std::thread::spawn(move || {
                    // Normal mode still uses a shared fetch_task that is Sync
                    // and can take an ID.
                    while r.load(Ordering::Acquire) {
                        if let Some(task) = ft(i) {
                            task();
                        } else {
                            std::thread::yield_now();
                        }
                    }
                });
                workers.push(WorkerHandle::new(i, handle));
            }

            ThreadPool {
                running,
                workers,
                fetch_task,
                submit_task,
                work_stealing: false,
            }
        }
    }
}
