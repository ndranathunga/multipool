//! Metrics collection for the thread pool.
//!
//! This module defines the `MetricsCollector` trait for collecting metrics about the
//! thread pool's activity, as well as default implementations for atomic metrics collection.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// A trait for collecting metrics from the thread pool.
///
/// Implementations of this trait provide hooks to track key events in the thread pool,
/// such as task submission, execution, and worker lifecycle changes.
pub trait MetricsCollector: Send + Sync {
    /// Called when a task is submitted to the thread pool.
    fn on_task_submitted(&self);
    /// Called when a task starts executing.
    fn on_task_started(&self);
    /// Called when a task completes execution.
    fn on_task_completed(&self);
    /// Called when a worker thread starts.
    fn on_worker_started(&self);
    /// Called when a worker thread stops.
    fn on_worker_stopped(&self);
}

/// Stores metrics for the thread pool using atomic counters.
///
/// The `ThreadPoolMetrics` struct tracks the following:
/// - `queued_tasks`: Number of tasks currently in the queue.
/// - `running_tasks`: Number of tasks currently being executed.
/// - `completed_tasks`: Total number of tasks that have completed execution.
/// - `active_threads`: Number of worker threads currently active.
pub struct ThreadPoolMetrics {
    /// Number of tasks currently queued for execution.
    pub queued_tasks: AtomicUsize,
    /// Number of tasks currently being executed.
    pub running_tasks: AtomicUsize,
    /// Total number of tasks that have been completed.
    pub completed_tasks: AtomicUsize,
    /// Number of worker threads currently active.
    pub active_threads: AtomicUsize,
}

impl ThreadPoolMetrics {
    /// Creates a new `ThreadPoolMetrics` instance with all counters initialized to zero.
    ///
    /// # Returns
    /// A new instance of `ThreadPoolMetrics`.
    pub fn new() -> Self {
        Self {
            queued_tasks: AtomicUsize::new(0),
            running_tasks: AtomicUsize::new(0),
            completed_tasks: AtomicUsize::new(0),
            active_threads: AtomicUsize::new(0),
        }
    }
}

/// A default implementation of `MetricsCollector` using atomic counters.
///
/// The `AtomicMetricsCollector` collects thread pool metrics and updates the counters
/// in a thread-safe manner. It is backed by an `Arc<ThreadPoolMetrics>` to share metrics
/// across multiple components.
pub struct AtomicMetricsCollector {
    /// Shared metrics storage.
    pub metrics: Arc<ThreadPoolMetrics>,
}

impl AtomicMetricsCollector {
    /// Creates a new `AtomicMetricsCollector` with the provided metrics.
    ///
    /// # Arguments
    /// - `metrics`: An `Arc<ThreadPoolMetrics>` instance to store and share metrics.
    ///
    /// # Returns
    /// A new instance of `AtomicMetricsCollector`.
    pub fn new(metrics: Arc<ThreadPoolMetrics>) -> Self {
        Self { metrics }
    }
}

impl MetricsCollector for AtomicMetricsCollector {
    /// Increments the count of queued tasks.
    fn on_task_submitted(&self) {
        self.metrics.queued_tasks.fetch_add(1, Ordering::SeqCst);
    }

    /// Decrements the count of queued tasks and increments the count of running tasks.
    fn on_task_started(&self) {
        self.metrics.queued_tasks.fetch_sub(1, Ordering::SeqCst);
        self.metrics.running_tasks.fetch_add(1, Ordering::SeqCst);
    }

    /// Decrements the count of running tasks and increments the count of completed tasks.
    fn on_task_completed(&self) {
        self.metrics.running_tasks.fetch_sub(1, Ordering::SeqCst);
        self.metrics.completed_tasks.fetch_add(1, Ordering::SeqCst);
    }

    /// Increments the count of active threads.
    fn on_worker_started(&self) {
        self.metrics.active_threads.fetch_add(1, Ordering::SeqCst);
    }

    /// Decrements the count of active threads.
    fn on_worker_stopped(&self) {
        self.metrics.active_threads.fetch_sub(1, Ordering::SeqCst);
    }
}
