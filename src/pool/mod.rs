pub mod modes;
pub mod task;
mod worker;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crate::{
    metrics::MetricsCollector, priority_stealer::prioritized_work_stealing_queues,
    queue::PriorityQueue,
};

use super::queue::TaskQueue;
use super::stealer::work_stealing_queues;
use modes::{
    GlobalQueueMode, NonPriorityTaskFetchMode, PriorityGlobalQueueMode, PriorityTaskFetchMode,
    PriorityWorkStealingMode, TaskFetchMode, WorkStealingMode,
};
use task::{spawn_task, BoxedTask, Priority, PriorityTask, TaskHandle};
use worker::{priority_worker_loop, worker_loop, WorkerHandle};

pub enum SubmitTaskType {
    Function(Arc<dyn Fn(BoxedTask) + Send + Sync>), // For dynamic dispatch
    Struct(Arc<dyn Fn(PriorityTask) + Send + Sync>), // For a specific struct
}

pub enum FetchTaskType {
    Function(Arc<dyn Fn(usize) -> Option<BoxedTask> + Send + Sync>),
    Struct(Arc<dyn Fn(usize) -> Option<PriorityTask> + Send + Sync>),
}

/// A configurable thread pool that can operate with a simple global queue or in work-stealing mode.
pub struct ThreadPool<M: TaskFetchMode> {
    running: Arc<AtomicBool>,
    workers: Vec<WorkerHandle>,
    _fetch_task: FetchTaskType,
    submit_task: SubmitTaskType,
    _work_stealing: bool,
    metrics_collector: Option<Arc<dyn MetricsCollector>>,
    mode: M,
}

impl<M: NonPriorityTaskFetchMode> ThreadPool<M> {
    pub fn spawn<F, T>(&self, f: F) -> TaskHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        let (task, handle) = spawn_task(f);
        if let SubmitTaskType::Function(submit) = &self.submit_task {
            (submit)(task);

            self.metrics_collector
                .as_ref()
                .map(|m| m.on_task_submitted());
        }
        handle
    }
}

impl<M: PriorityTaskFetchMode> ThreadPool<M> {
    pub fn spawn_with_priority<F, T>(&self, f: F, priority: usize) -> TaskHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        let (task, handle) = spawn_task(f);

        if let SubmitTaskType::Struct(submit) = &self.submit_task {
            let task = PriorityTask::new(task, Priority(priority));
            (submit)(task);

            self.metrics_collector
                .as_ref()
                .map(|m| m.on_task_submitted());
        }
        handle
    }
}

impl<M: TaskFetchMode> ThreadPool<M> {
    pub fn shutdown(mut self) {
        self.running.store(false, Ordering::Release);
        for worker in &mut self.workers {
            worker.join();
        }
    }

    pub fn mode(&self) -> &str {
        self.mode.mode()
    }
}
/// States for the ThreadPoolBuilder
pub struct DefaultModeState; // no features selected
pub struct WorkStealingState; // work_stealing = true, priority = false
pub struct PriorityState; // priority = true, work_stealing = false
pub struct PriorityWorkStealingState; // priority = true, work_stealing = true

/// Typed-state Builder Pattern
pub struct ThreadPoolBuilder<S = GlobalQueueMode> {
    num_threads: usize,
    metrics_collector: Option<Arc<dyn MetricsCollector>>,
    _state: std::marker::PhantomData<S>,
}

impl<S> ThreadPoolBuilder<S> {
    pub fn with_metrics_collector(mut self, collector: Arc<dyn MetricsCollector>) -> Self {
        self.metrics_collector = Some(collector);
        self
    }
}

impl ThreadPoolBuilder<DefaultModeState> {
    pub fn new() -> Self {
        Self {
            num_threads: 4,
            metrics_collector: None,
            _state: std::marker::PhantomData,
        }
    }

    pub fn num_threads(mut self, n: usize) -> Self {
        self.num_threads = n;
        self
    }

    pub fn set_work_stealing(self) -> ThreadPoolBuilder<WorkStealingState> {
        ThreadPoolBuilder {
            num_threads: self.num_threads,
            metrics_collector: self.metrics_collector,
            _state: std::marker::PhantomData,
        }
    }

    pub fn enable_priority(self) -> ThreadPoolBuilder<PriorityState> {
        ThreadPoolBuilder {
            num_threads: self.num_threads,
            metrics_collector: self.metrics_collector,
            _state: std::marker::PhantomData,
        }
    }

    pub fn build(self) -> ThreadPool<GlobalQueueMode> {
        let running = Arc::new(AtomicBool::new(true));

        // Non-priority global queue mode
        let queue = TaskQueue::new();

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
        let running_clone = Arc::clone(&running);
        let metrics_collector = self.metrics_collector.clone();

        for i in 0..self.num_threads {
            let r = Arc::clone(&running_clone);
            let metrics_collector_clone = self.metrics_collector.clone();
            let ft = Arc::clone(&fetch_task);
            let handle = std::thread::spawn(move || {
                // Normal mode uses a shared fetch_task that is Sync
                // and can take an ID.
                while r.load(Ordering::Acquire) {
                    if let Some(task) = ft(i) {
                        metrics_collector_clone
                            .as_ref()
                            .map(|m| m.on_task_started());

                        task();

                        metrics_collector_clone
                            .as_ref()
                            .map(|m| m.on_task_completed());
                    } else {
                        std::thread::yield_now();
                    }
                }

                metrics_collector_clone
                    .as_ref()
                    .map(|m| m.on_worker_stopped());
            });
            workers.push(WorkerHandle::new(i, handle));
            metrics_collector.as_ref().map(|m| m.on_worker_started());
        }

        ThreadPool::<GlobalQueueMode> {
            running,
            workers,
            _fetch_task: FetchTaskType::Function(fetch_task),
            submit_task: SubmitTaskType::Function(submit_task),
            _work_stealing: false,
            metrics_collector: self.metrics_collector,
            mode: GlobalQueueMode,
        }
    }
}

impl ThreadPoolBuilder<WorkStealingState> {
    pub fn enable_priority(self) -> ThreadPoolBuilder<PriorityWorkStealingState> {
        ThreadPoolBuilder {
            num_threads: self.num_threads,
            metrics_collector: self.metrics_collector,
            _state: std::marker::PhantomData,
        }
    }

    pub fn build(self) -> ThreadPool<WorkStealingMode> {
        let running = Arc::new(AtomicBool::new(true));

        // Non-priority work-stealing mode
        let metrics_collector_clone = self.metrics_collector.clone();
        let (injector, stealers, mut workers_local) =
            work_stealing_queues(self.num_threads, metrics_collector_clone);

        // ! Checkout this pattern
        let submit_task = {
            let injector = Arc::clone(&injector);
            Arc::new(move |task: BoxedTask| {
                injector.push(task);
            }) as Arc<dyn Fn(BoxedTask) + Send + Sync>
        };

        let stealers = Arc::new(stealers);
        let mut workers = Vec::with_capacity(self.num_threads);
        let running_clone = Arc::clone(&running);

        for i in 0..self.num_threads {
            let r = Arc::clone(&running_clone);
            let metrics_collector_clone = self.metrics_collector.clone();

            // Take a worker for this thread
            let worker = workers_local.remove(0);
            let injector = Arc::clone(&injector);
            let stealers_for_thread = Arc::clone(&stealers);

            // Per-thread fetch_task closure
            // ? This closure doesn't need to be Send+Sync because it will be moved into
            // ? the thread that uses it. It only needs to be 'static because the thread
            // ? may outlive the stack frame.
            let fetch_task = move || {
                // First try to pop from this thread's worker
                if let Some(task) = worker.pop() {
                    return Some(task);
                }

                // Try stealing a batch of tasks from the injector
                match injector.steal_batch(&worker) {
                    crossbeam::deque::Steal::Success(_) => {
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
                worker_loop(r, fetch_task, metrics_collector_clone);
            });
            workers.push(WorkerHandle::new(i, handle));
        }

        ThreadPool::<WorkStealingMode> {
            running,
            workers,
            // since each thread has its own closure,
            // Just provide a dummy closure or modify ThreadPool struct to make it optional.
            _fetch_task: FetchTaskType::Function(Arc::new(|_| None)),
            submit_task: SubmitTaskType::Function(submit_task),
            _work_stealing: true,
            metrics_collector: self.metrics_collector,
            mode: WorkStealingMode,
        }
    }
}

impl ThreadPoolBuilder<PriorityState> {
    pub fn build(self) -> ThreadPool<PriorityGlobalQueueMode> {
        let running = Arc::new(AtomicBool::new(true));

        // Priority global queue mode
        let queue = PriorityQueue::new();

        let fetch_task = {
            let queue_clone = queue.clone();
            Arc::new(move |_: usize| queue_clone.pop()) // ignoring ID here
                as Arc<dyn Fn(usize) -> Option<PriorityTask> + Send + Sync>
        };

        let submit_task = {
            let queue_clone = queue.clone();
            Arc::new(move |task: PriorityTask| {
                queue_clone.push(task);
            }) as Arc<dyn Fn(PriorityTask) + Send + Sync>
        };

        let mut workers = Vec::with_capacity(self.num_threads);
        let running_clone = Arc::clone(&running);
        let metrics_collector = self.metrics_collector.clone();

        for i in 0..self.num_threads {
            let r = Arc::clone(&running_clone);
            let metrics_collector_clone = self.metrics_collector.clone();

            let ft = Arc::clone(&fetch_task);
            let handle = std::thread::spawn(move || {
                // Normal mode uses a shared fetch_task that is Sync
                // and can take an ID.
                // ? TODO: Try to move this to a separate function (worker_loop)
                while r.load(Ordering::Acquire) {
                    if let Some(pt) = ft(i) {
                        metrics_collector_clone
                            .as_ref()
                            .map(|m| m.on_task_started());

                        // Passing the ID doesn't matter in this mode
                        (pt.task)();

                        metrics_collector_clone
                            .as_ref()
                            .map(|m| m.on_task_completed());
                    } else {
                        std::thread::yield_now();
                    }
                }
                metrics_collector_clone
                    .as_ref()
                    .map(|m| m.on_worker_stopped());
            });
            workers.push(WorkerHandle::new(i, handle));
            metrics_collector.as_ref().map(|m| m.on_worker_started());
        }

        ThreadPool::<PriorityGlobalQueueMode> {
            running,
            workers,
            _fetch_task: FetchTaskType::Struct(fetch_task),
            submit_task: SubmitTaskType::Struct(submit_task),
            _work_stealing: false,
            metrics_collector: self.metrics_collector,
            mode: PriorityGlobalQueueMode,
        }
    }
}

impl ThreadPoolBuilder<PriorityWorkStealingState> {
    pub fn build(self) -> ThreadPool<PriorityWorkStealingMode> {
        let running = Arc::new(AtomicBool::new(true));

        // Non-priority work-stealing mode
        let metrics_collector_clone = self.metrics_collector.clone();
        let (injector, stealers, mut workers_local) =
            prioritized_work_stealing_queues(self.num_threads, metrics_collector_clone);

        let submit_task = {
            let injector = Arc::clone(&injector);
            Arc::new(move |task: PriorityTask| {
                injector.push(task);
            }) as Arc<dyn Fn(PriorityTask) + Send + Sync>
        };

        let stealers = Arc::new(stealers);
        let mut workers = Vec::with_capacity(self.num_threads);
        let running_clone = Arc::clone(&running);

        for i in 0..self.num_threads {
            let r = Arc::clone(&running_clone);
            let metrics_collector_clone = self.metrics_collector.clone();

            // Take a worker for this thread
            let worker = workers_local.remove(0);
            let injector = Arc::clone(&injector);
            let stealers_for_thread = Arc::clone(&stealers);

            // Per-thread fetch_task closure
            // ? This closure doesn't need to be Send+Sync because it will be moved into
            // ? the thread that uses it. It only needs to be 'static because the thread
            // ? may outlive the stack frame.
            let fetch_task = move || {
                // First try to pop from this thread's worker
                if let Some(task) = worker.pop() {
                    return Some(task);
                }

                // Try stealing a batch of tasks from the injector
                match injector.steal_batch(&worker) {
                    crate::priority_stealer::Steal::Success(_) => {
                        if let Some(task) = worker.pop() {
                            return Some(task);
                        }
                    }
                    crate::priority_stealer::Steal::Empty
                    | crate::priority_stealer::Steal::Retry => {}
                }

                // Try stealing from other workers using their stealers
                for st in stealers_for_thread.iter() {
                    match st.steal() {
                        crate::priority_stealer::Steal::Success(t) => return Some(t),
                        _ => {}
                    }
                }

                None
            };

            let handle = std::thread::spawn(move || {
                priority_worker_loop(r, fetch_task, metrics_collector_clone);
            });
            workers.push(WorkerHandle::new(i, handle));
        }

        ThreadPool::<PriorityWorkStealingMode> {
            running,
            workers,
            // since each thread has its own closure,
            // Just provide a dummy closure or modify ThreadPool struct to make it optional.
            _fetch_task: FetchTaskType::Struct(Arc::new(|_| None)),
            submit_task: SubmitTaskType::Struct(submit_task),
            _work_stealing: true,
            metrics_collector: self.metrics_collector,
            mode: PriorityWorkStealingMode,
        }
    }
}
