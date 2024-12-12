//! Provides functionality for creating work-stealing queues.
//!
//! This module sets up a work-stealing system with an injector queue and per-worker queues.
//! It uses the `crossbeam::deque` crate to implement efficient task distribution and load balancing
//! among workers, with support for optional metrics collection.

use crossbeam::deque::{Injector, Stealer, Worker};
use std::sync::Arc;

use crate::metrics::MetricsCollector;

/// Creates a set of work-stealing queues for task distribution.
///
/// This function initializes:
/// - An `Injector<T>` queue for global task injection.
/// - A vector of `Stealer<T>` instances, each associated with a worker thread.
/// - A vector of `Worker<T>` instances, one for each worker thread.
///
/// Each worker has its own local queue and a `Stealer` that allows other workers
/// to steal tasks when their own queues are empty. The `Injector` acts as a global
/// task queue, distributing tasks to the workers.
///
/// # Arguments
/// - `num_workers`: The number of worker threads to create.
/// - `metrics_collector`: An optional `MetricsCollector` instance to monitor worker activity.
///
/// # Returns
/// A tuple containing:
/// - An `Arc<Injector<T>>`: The global task injector queue.
/// - A `Vec<Stealer<T>>`: A vector of `Stealer` instances for each worker thread.
/// - A `Vec<Worker<T>>`: A vector of worker queues.
///
/// # Type Parameters
/// - `T`: The type of tasks managed by the queues.
///
/// # Example (internal usage)
/// ```ignore
/// let (injector, stealers, workers) = work_stealing_queues(4, None);
/// ```
pub fn work_stealing_queues<T>(
    num_workers: usize,
    metrics_collector: Option<Arc<dyn MetricsCollector>>,
) -> (Arc<Injector<T>>, Vec<Stealer<T>>, Vec<Worker<T>>) {
    let injector = Arc::new(Injector::new());
    let mut workers = Vec::with_capacity(num_workers);
    let mut stealers = Vec::with_capacity(num_workers);

    for _ in 0..num_workers {
        let w = Worker::new_fifo();
        metrics_collector.as_ref().map(|m| m.on_worker_started());

        stealers.push(w.stealer());
        workers.push(w);
    }

    (injector, stealers, workers)
}
