mod task;
mod work_stealing;
mod worker;

use super::queue::TaskQueue;
use super::stealer::WorkStealingQueues;
use task::{spawn_task, BoxedTask, TaskHandle};
use worker::{worker_loop, WorkerHandle};

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub struct ThreadPool {
    running: Arc<AtomicBool>,
    workers: Vec<WorkerHandle>,
    fetch_task: Arc<dyn Fn(usize) -> Option<BoxedTask> + Send + Sync>,
    work_stealing: bool,
}

impl ThreadPool {
    /// Creates a new thread pool with a given number of threads, using a simple queue.
    pub fn new(num_threads: usize) -> Self {
        ThreadPoolBuilder::new().num_threads(num_threads).build()
    }

    /// Spawn a task into the pool.
    pub fn spawn<F, T>(&self, f: F) -> TaskHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        let (task, handle) = spawn_task(f);
        self.submit_task(task);
        handle
    }

    fn submit_task(&self, task: BoxedTask) {
        // The logic for submitting a task depends on whether we have work_stealing enabled.
        // We'll rely on the closure environment to do the right thing.
        // If not using work stealing, we had a shared queue. If using work stealing,
        // we push to the global injector.
        // This would require that `fetch_task` closure has references to the actual queue structures.
        // We'll rely on a pattern established in the builder implementation.
        if self.work_stealing {
            // We can't directly push here; rely on a closure or store a reference
            // In a real scenario, we'd store references in ThreadPool.
            panic!("submit_task should be overridden by builder if using work-stealing");
        } else {
            // no-op here because we rely on the fetch_task closure
            panic!("submit_task should be overridden by builder in normal mode");
        }
    }

    /// Gracefully shut down the pool
    pub fn shutdown(mut self) {
        self.running.store(false, Ordering::Release);
        for worker in &mut self.workers {
            worker.join();
        }
    }
}

/// A builder pattern for creating a thread pool with configuration.
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
            let ws_queues = WorkStealingQueues::new(self.num_threads);
            let running_clone = Arc::clone(&running);

            // fetch_task closure:
            let fetch_task = Arc::new(move |id: usize| {
                if let Some(task) = ws_queues.pop(id) {
                    return Some(task);
                }
                // try stealing:
                ws_queues.steal_work(id)
            });

            let mut workers = Vec::with_capacity(self.num_threads);
            for i in 0..self.num_threads {
                let r = Arc::clone(&running_clone);
                let ft = Arc::clone(&fetch_task);
                let handle = std::thread::spawn(move || {
                    worker_loop(i, r, ft);
                });
                workers.push(WorkerHandle::new(i, handle));
            }

            // We'll implement submit_task by capturing ws_queues
            let pool = ThreadPool {
                running,
                workers,
                fetch_task,
                work_stealing: true,
            };

            // override submit_task for this instance:
            let submit_fn = {
                let ws_queues = ws_queues.injector.clone();
                move |task: BoxedTask| {
                    ws_queues.push(task);
                }
            };

            // We can't easily override methods, but we can store the submission logic
            // in a closure. Let’s store it in a trait object in ThreadPool:
            // However, we have a design issue: ThreadPool::submit_task currently
            // can’t be easily overridden. Let’s refactor ThreadPool to store a submission closure.

            // Updated approach: we will store the submission closure directly in ThreadPool.
            // Let's modify the code slightly:

            // We'll need a second build method or adjust the design.
            // For simplicity, let's define a separate struct. But that complicates the demonstration.
            // Instead, let's define ThreadPool fully here.

            drop(pool); // Drop the incomplete pool and redefine properly below.

            struct ThreadPoolInner {
                running: Arc<AtomicBool>,
                workers: Vec<WorkerHandle>,
                fetch_task: Arc<dyn Fn(usize) -> Option<BoxedTask> + Send + Sync>,
                submit_task: Box<dyn Fn(BoxedTask) + Send + Sync>,
                work_stealing: bool,
            }

            let inner = Arc::new(ThreadPoolInner {
                running: running_clone,
                workers: workers,
                fetch_task,
                submit_task: Box::new(submit_fn),
                work_stealing: true,
            });

            // Create a user-facing ThreadPool that delegates to inner
            ThreadPoolWrapper(inner)
        } else {
            let queue = TaskQueue::new();
            let running_clone = Arc::clone(&running);

            let fetch_task = Arc::new(move |_id: usize| queue.pop());

            let mut workers = Vec::with_capacity(self.num_threads);
            for i in 0..self.num_threads {
                let r = Arc::clone(&running_clone);
                let ft = Arc::clone(&fetch_task);
                let handle = std::thread::spawn(move || {
                    worker_loop(i, r, ft);
                });
                workers.push(WorkerHandle::new(i, handle));
            }

            let submit_fn = {
                let queue = queue.inner.clone();
                move |task: BoxedTask| {
                    queue.push(task);
                }
            };

            struct ThreadPoolInner {
                running: Arc<AtomicBool>,
                workers: Vec<WorkerHandle>,
                fetch_task: Arc<dyn Fn(usize) -> Option<BoxedTask> + Send + Sync>,
                submit_task: Box<dyn Fn(BoxedTask) + Send + Sync>,
                work_stealing: bool,
            }

            let inner = Arc::new(ThreadPoolInner {
                running: running_clone,
                workers,
                fetch_task,
                submit_task: Box::new(submit_fn),
                work_stealing: false,
            });

            ThreadPoolWrapper(inner)
        }
    }
}

// Due to complexity, let's define a wrapper that implements ThreadPool methods.
use std::ops::Deref;

struct ThreadPoolWrapper(Arc<ThreadPoolInner>);

impl Deref for ThreadPoolWrapper {
    type Target = ThreadPoolInner;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ThreadPool for ThreadPoolWrapper {
    fn spawn<F, T>(&self, f: F) -> TaskHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        let (task, handle) = spawn_task(f);
        (self.submit_task)(task);
        handle
    }

    fn shutdown(mut self) {
        self.running.store(false, Ordering::Release);
        for worker in &mut self.workers.iter_mut() {
            worker.join();
        }
    }
}

pub trait ThreadPool: Sized {
    fn spawn<F, T>(&self, f: F) -> TaskHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static;

    fn shutdown(self);
}
