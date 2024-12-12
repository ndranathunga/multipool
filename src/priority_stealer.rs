//! Provides functionality for priority-based work-stealing queues.
//!
//! This module introduces structures and utilities for managing tasks with priorities
//! using a global priority queue (`PriorityInjector`) and local worker queues
//! (`PriorityWorker`). Tasks can be stolen from local queues via `PriorityStealer`
//! to maintain load balancing across workers.

use std::sync::Arc;

use crate::metrics::MetricsCollector;
use crate::pool::task::IPriority;
use crate::queue::PriorityQueue;

/// Represents the result of a steal operation.
pub enum Steal<T> {
    /// The queue is empty, no tasks available to steal.
    Empty,
    /// A task was successfully stolen.
    Success(T),
    /// The operation should be retried (e.g., due to contention).
    Retry,
}

/// A global priority injector queue.
///
/// The `PriorityInjector` maintains a shared global priority queue for
/// distributing tasks across workers. Tasks are prioritized based on
/// their priority levels, with lower values indicating higher priority.
pub struct PriorityInjector<T> {
    global_queue: Arc<PriorityQueue<T>>,
}

#[allow(dead_code)]
impl<T: std::cmp::Ord + IPriority> PriorityInjector<T> {
    /// Creates a new `PriorityInjector` with an empty global queue.
    pub fn new() -> Self {
        PriorityInjector {
            global_queue: Arc::new(PriorityQueue::new()),
        }
    }

    /// Pushes a task into the global priority queue.
    ///
    /// # Arguments
    /// - `item`: The task to be added to the global queue.
    pub fn push(&self, item: T) {
        self.global_queue.push(item);
    }

    /// Attempts to pop the highest-priority task from the global queue.
    ///
    /// Returns `None` if the queue is empty.
    pub(self) fn pop(&self) -> Option<T> {
        self.global_queue.pop()
    }

    /// Steals tasks in batches from the global queue and pushes them into a worker's local queue.
    ///
    /// # Arguments
    /// - `dest`: The `PriorityWorker` where the stolen tasks will be pushed.
    ///
    /// # Returns
    /// - `Steal::Empty` if no tasks were available.
    /// - `Steal::Success(())` if tasks were successfully stolen.
    pub fn steal_batch(&self, dest: &PriorityWorker<T>) -> Steal<()> {
        let count = self.global_queue.len();
        if count == 0 {
            return Steal::Empty;
        }

        let mut stolen = Vec::new();

        // Steal half of the tasks
        for _ in 0..((count + 1) / 2) {
            // ! FIXME: This could potentially be wrong. Need proper investigation.
            if let Some(task) = self.global_queue.pop() {
                stolen.push(task);
            }
        }
        dest.push_batch(stolen);
        return Steal::Success(());
    }

    /// Returns a clone of the global queue.
    fn arc_clone(&self) -> Arc<PriorityQueue<T>> {
        Arc::clone(&self.global_queue)
    }
}

/// A worker with a local priority queue.
///
/// Each `PriorityWorker` maintains its own local queue for managing tasks
/// with priorities. Tasks are pushed and popped based on their priority levels.
pub struct PriorityWorker<T> {
    local: Arc<PriorityQueue<T>>,
}

#[allow(dead_code)]
impl<T: std::cmp::Ord + IPriority> PriorityWorker<T> {
    /// Creates a new `PriorityWorker` with an empty local queue.
    pub fn new() -> Self {
        PriorityWorker {
            local: Arc::new(PriorityQueue::new()),
        }
    }

    /// Pushes a task into the local worker queue.
    ///
    /// # Arguments
    /// - `task`: The task to be added to the worker's local queue.
    pub fn push(&self, task: T) {
        self.local.push(task);
    }

    /// Pushes a batch of tasks into the local worker queue.
    ///
    /// # Arguments
    /// - `items`: A vector of tasks to be added to the queue.
    pub(self) fn push_batch(&self, items: Vec<T>) {
        for item in items {
            self.local.push(item);
        }
    }

    /// Pops the highest-priority task from the local queue.
    ///
    /// Returns `None` if the queue is empty.
    pub fn pop(&self) -> Option<T> {
        self.local.pop()
    }

    /// Creates a `PriorityStealer` for stealing tasks from this worker's local queue.
    pub fn stealer(&self) -> PriorityStealer<T> {
        PriorityStealer {
            local: Arc::clone(&self.local),
        }
    }
}

impl<T> Clone for PriorityWorker<T> {
    /// Clones the `PriorityWorker`, sharing the same underlying local queue.
    fn clone(&self) -> Self {
        PriorityWorker {
            local: Arc::clone(&self.local),
        }
    }
}

/// A stealer that can steal tasks from a worker's local priority queue.
///
/// The `PriorityStealer` allows tasks to be stolen from a `PriorityWorker`
/// to balance the load across multiple workers.
pub struct PriorityStealer<T> {
    local: Arc<PriorityQueue<T>>,
}

impl<T: std::cmp::Ord + IPriority> PriorityStealer<T> {
    /// Attempts to steal a task from the associated worker's local queue.
    ///
    /// # Returns
    /// - `Steal::Empty` if no tasks are available.
    /// - `Steal::Success(T)` if a task was successfully stolen.
    /// - `Steal::Retry` if the operation should be retried.
    pub fn steal(&self) -> Steal<T> {
        if self.local.len() == 0 {
            return Steal::Empty;
        }

        self.local.pop().map_or(Steal::Retry, Steal::Success)
    }
}

/// Creates prioritized work-stealing queues for task distribution.
///
/// This function initializes:
/// - A global priority injector queue (`PriorityInjector<T>`).
/// - A vector of local worker queues (`PriorityWorker<T>`).
/// - A vector of task stealers (`PriorityStealer<T>`) for each worker.
///
/// # Arguments
/// - `num_workers`: The number of worker threads to create.
/// - `metrics_collector`: An optional metrics collector for monitoring worker activity.
///
/// # Returns
/// A tuple containing:
/// - An `Arc<PriorityInjector<T>>`: The global priority queue.
/// - A `Vec<PriorityStealer<T>>`: A vector of stealers for each worker.
/// - A `Vec<PriorityWorker<T>>`: A vector of worker queues.
pub fn prioritized_work_stealing_queues<T: std::cmp::Ord + IPriority>(
    num_workers: usize,
    metrics_collector: Option<Arc<dyn MetricsCollector>>,
) -> (
    Arc<PriorityInjector<T>>,
    Vec<PriorityStealer<T>>,
    Vec<PriorityWorker<T>>,
) {
    let injector = Arc::new(PriorityInjector::new());
    let mut workers = Vec::with_capacity(num_workers);
    let mut stealers = Vec::with_capacity(num_workers);

    for _ in 0..num_workers {
        let w = PriorityWorker::new();
        metrics_collector.as_ref().map(|m| m.on_worker_started());

        stealers.push(w.stealer());
        workers.push(w);
    }

    (injector, stealers, workers)
}
