//! Worker logic for the thread pool
//!
//! This module defines the logic for worker threads in the thread pool. It provides
//! the main loop for task execution, handles worker lifecycle, and integrates optional
//! metrics collection for monitoring task execution.

use crate::metrics::MetricsCollector;

use super::task::{BoxedTask, PriorityTask};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;

/// Represents a handle to a worker thread.
///
/// The `WorkerHandle` provides control over a worker thread, including the ability
/// to join it during shutdown.
pub struct WorkerHandle {
    #[allow(dead_code)]
    id: usize,
    // Optionally stores a thread handle for joining the thread during shutdown.
    thread: Option<thread::JoinHandle<()>>,
}

impl WorkerHandle {
    /// Creates a new `WorkerHandle`.
    ///
    /// # Arguments
    /// - `id`: The identifier for the worker thread.
    /// - `thread`: The thread handle for the worker thread.
    ///
    /// # Returns
    /// A new instance of `WorkerHandle`.
    pub fn new(id: usize, thread: thread::JoinHandle<()>) -> Self {
        Self {
            id,
            thread: Some(thread),
        }
    }

    /// Joins the worker thread, blocking until it completes.
    ///
    /// This is typically called during shutdown to ensure all worker threads
    /// finish execution cleanly.
    pub fn join(&mut self) {
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}

/// The main loop for a worker thread.
///
/// This function runs the worker's task execution loop, fetching tasks using
/// the provided `fetch_task` function. If a task is available, it is executed.
/// Otherwise, the thread yields to allow other threads to progress.
///
/// Optionally integrates metrics collection to track task execution.
///
/// # Arguments
/// - `running`: An `Arc<AtomicBool>` indicating whether the worker should continue running.
/// - `fetch_task`: A function that retrieves the next task to execute. Returns `None` if no tasks are available.
/// - `metrics_collector`: An optional metrics collector for monitoring task execution.
///
/// # Type Parameters
/// - `F`: A function or closure that fetches tasks, returning an `Option<BoxedTask>`.
pub fn worker_loop<F>(
    running: Arc<AtomicBool>,
    mut fetch_task: F,
    metrics_collector: Option<Arc<dyn MetricsCollector>>,
) where
    F: FnMut() -> Option<BoxedTask>,
{
    while running.load(Ordering::Acquire) {
        if let Some(task) = fetch_task() {
            metrics_collector.as_ref().map(|m| m.on_task_started());

            task();

            metrics_collector.as_ref().map(|m| m.on_task_completed());
        } else {
            std::thread::yield_now();
        }
    }
    metrics_collector.as_ref().map(|m| m.on_worker_stopped());
}

/// The main loop for a priority worker thread.
///
/// This function operates similarly to `worker_loop`, but it handles `PriorityTask`s
/// instead of generic tasks. Tasks are fetched using the provided `fetch_task` function
/// and executed based on their priority.
///
/// Optionally integrates metrics collection to track task execution.
///
/// # Arguments
/// - `running`: An `Arc<AtomicBool>` indicating whether the worker should continue running.
/// - `fetch_task`: A function that retrieves the next `PriorityTask` to execute. Returns `None` if no tasks are available.
/// - `metrics_collector`: An optional metrics collector for monitoring task execution.
///
/// # Type Parameters
/// - `F`: A function or closure that fetches tasks, returning an `Option<PriorityTask>`.
pub fn priority_worker_loop<F>(
    running: Arc<AtomicBool>,
    mut fetch_task: F,
    metrics_collector: Option<Arc<dyn MetricsCollector>>,
) where
    F: FnMut() -> Option<PriorityTask>,
{
    while running.load(Ordering::Acquire) {
        if let Some(pt) = fetch_task() {
            metrics_collector.as_ref().map(|m| m.on_task_started());

            (pt.task)();

            metrics_collector.as_ref().map(|m| m.on_task_completed());
        } else {
            std::thread::yield_now();
        }
    }
    metrics_collector.as_ref().map(|m| m.on_worker_stopped());
}
