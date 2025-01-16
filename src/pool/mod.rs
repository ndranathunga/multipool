//! Thread pool implementation with configurable task fetching modes.
//!
//! This module defines a thread pool that supports various task scheduling
//! and fetching strategies, such as:
//! - Global queue-based task distribution.
//! - Work-stealing for efficient load balancing.
//! - Priority-based task scheduling with optional work-stealing.
//!
//! It also provides a `ThreadPoolBuilder` to configure and create a thread pool
//! with the desired settings.

pub mod modes;
pub(crate) mod task;
mod worker;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use super::queue::TaskQueue;
use super::stealer::work_stealing_queues;
use crate::{
    metrics::MetricsCollector, priority_stealer::prioritized_work_stealing_queues,
    queue::PriorityQueue,
};
use modes::{
    GlobalQueueMode, NonPriorityTaskFetchMode, PriorityGlobalQueueMode, PriorityTaskFetchMode,
    PriorityWorkStealingMode, TaskFetchMode, WorkStealingMode,
};
use task::{spawn_task, BoxedTask, Priority, PriorityTask, TaskHandle};
use worker::{priority_worker_loop, worker_loop, WorkerHandle};

/// Represents the type of task submission for the thread pool.
///
/// This determines how tasks are added to the pool, either as functions
/// or as structured tasks with priorities.
pub enum SubmitTaskType {
    /// Task submission for non-priority tasks using dynamic dispatch.
    Function(Arc<dyn Fn(BoxedTask) + Send + Sync>),
    /// Task submission for priority-based tasks.
    Struct(Arc<dyn Fn(PriorityTask) + Send + Sync>),
}

/// Represents the type of task fetching for the thread pool.
///
/// This determines how tasks are fetched from the pool for execution.
pub enum FetchTaskType {
    /// Task fetching for non-priority tasks.
    Function(Arc<dyn Fn(usize) -> Option<BoxedTask> + Send + Sync>),
    /// Task fetching for priority-based tasks.
    Struct(Arc<dyn Fn(usize) -> Option<PriorityTask> + Send + Sync>),
}

/// A configurable thread pool that can operate with a global queue or in work-stealing mode.
///
/// The thread pool is generic over the `TaskFetchMode`, allowing different strategies for
/// task scheduling and execution.
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
    /// Submits a task to the thread pool and returns a handle to retrieve its result.
    ///
    /// # Arguments
    /// - `f`: A closure representing the task to execute.
    ///
    /// # Returns
    /// A `TaskHandle<T>` for accessing the task's result.
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
    /// Submits a task with a priority to the thread pool and returns a handle to retrieve its result.
    ///
    /// # Arguments
    /// - `f`: A closure representing the task to execute.
    /// - `priority`: The priority of the task, where lower values indicate higher priority.
    ///
    /// # Returns
    /// A `TaskHandle<T>` for accessing the task's result.
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
    /// Shuts down the thread pool, stopping all workers and cleaning up resources.
    pub fn shutdown(mut self) {
        self.running.store(false, Ordering::Release);
        for worker in &mut self.workers {
            worker.join();
        }
    }

    /// Returns the name of the task fetching mode used by the thread pool.
    pub fn mode(&self) -> &str {
        self.mode.mode()
    }
}

/// A trait for defining the states of the `ThreadPoolBuilder`.
pub trait IBuilderState {}

/// States for the `ThreadPoolBuilder`.
///
/// The `ThreadPoolBuilder` uses a typed-state pattern to enforce proper configuration
/// of the thread pool before building it. Each state represents a specific configuration
/// combination, ensuring that invalid combinations are caught at compile time.

/// The default state for the `ThreadPoolBuilder`.
///
/// In this state:
/// - No specific features are enabled.
/// - The thread pool uses a global queue to manage tasks.
/// - Tasks are processed in the order they are submitted (FIFO).
///
/// This state is the starting point of the builder and represents the simplest configuration.
pub struct DefaultModeState;

impl IBuilderState for DefaultModeState {}

/// The work-stealing state for the `ThreadPoolBuilder`.
///
/// In this state:
/// - Work-stealing is enabled, which means each worker has its own local queue.
/// - If a worker's local queue is empty, it can steal tasks from other workers' queues.
/// - Priority-based task scheduling is not enabled in this state.
/// - Tasks are processed in the order they are submitted to the local or global queues.
///
/// Work-stealing is useful for load balancing, as idle workers can take tasks
/// from busy workers, improving overall throughput.
pub struct WorkStealingState;

impl IBuilderState for WorkStealingState {}

/// The priority-based state for the `ThreadPoolBuilder`.
///
/// In this state:
/// - Priority-based task scheduling is enabled.
/// - All tasks are stored in a global priority queue, where tasks with lower
///   priority values are executed before those with higher priority values.
/// - Work-stealing is not enabled in this state.
/// - Tasks are executed strictly based on their priorities, regardless of the
///   order in which they are submitted.
///
/// This state is suitable for scenarios where task prioritization is critical, such
/// as time-sensitive computations or real-time systems.
pub struct PriorityState;

impl IBuilderState for PriorityState {}

/// The priority-based work-stealing state for the `ThreadPoolBuilder`.
///
/// In this state:
/// - Both priority-based task scheduling and work-stealing are enabled.
/// - Each worker has its own local priority queue to store and process tasks.
/// - If a worker's local queue is empty, it can steal tasks from other workers'
///   queues, prioritizing tasks with lower priority values.
/// - Tasks are executed based on their priorities, with lower priority values
///   being processed first.
///
/// This state is ideal for systems that require both task prioritization and dynamic
/// load balancing, combining the benefits of both approaches.
pub struct PriorityWorkStealingState;

impl IBuilderState for PriorityWorkStealingState {}

/// A builder for configuring and creating a thread pool.
///
/// This uses a typed-state builder pattern to enforce proper configuration.
pub struct ThreadPoolBuilder<S: IBuilderState = DefaultModeState> {
    num_threads: usize,
    metrics_collector: Option<Arc<dyn MetricsCollector>>,
    _state: std::marker::PhantomData<S>,
}

impl<S: IBuilderState> ThreadPoolBuilder<S> {
    /// Adds a metrics collector to the builder.
    ///
    /// # Arguments
    /// - `collector`: An `Arc<dyn MetricsCollector>` for monitoring the thread pool's behavior.
    pub fn with_metrics_collector(mut self, collector: Arc<dyn MetricsCollector>) -> Self {
        self.metrics_collector = Some(collector);
        self
    }

    /// Sets the number of threads for the thread pool.
    ///
    /// # Arguments
    /// - `n`: The number of threads to create in the thread pool.
    pub fn num_threads(mut self, n: usize) -> Self {
        self.num_threads = n;
        self
    }
}

impl ThreadPoolBuilder<DefaultModeState> {
    /// Creates a new `ThreadPoolBuilder` with default settings.
    pub fn new() -> Self {
        Self {
            num_threads: 4,
            metrics_collector: None,
            _state: std::marker::PhantomData,
        }
    }

    /// Configures the builder to enable work-stealing mode.
    pub fn set_work_stealing(self) -> ThreadPoolBuilder<WorkStealingState> {
        ThreadPoolBuilder {
            num_threads: self.num_threads,
            metrics_collector: self.metrics_collector,
            _state: std::marker::PhantomData,
        }
    }

    /// Configures the builder to enable priority-based task scheduling.
    pub fn enable_priority(self) -> ThreadPoolBuilder<PriorityState> {
        ThreadPoolBuilder {
            num_threads: self.num_threads,
            metrics_collector: self.metrics_collector,
            _state: std::marker::PhantomData,
        }
    }

    /// Builds the thread pool with the default global queue mode.
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
    /// Configures the builder to enable priority-based task scheduling.
    pub fn enable_priority(self) -> ThreadPoolBuilder<PriorityWorkStealingState> {
        ThreadPoolBuilder {
            num_threads: self.num_threads,
            metrics_collector: self.metrics_collector,
            _state: std::marker::PhantomData,
        }
    }

    /// Builds the thread pool with work-stealing mode.
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
    /// Builds the thread pool with priority global queue mode.
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
    /// Builds the thread pool with priority work-stealing mode.
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
