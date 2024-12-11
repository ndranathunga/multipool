//! Worker logic for the thread pool

use crate::metrics::MetricsCollector;

use super::task::{BoxedTask, PriorityTask};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;

pub struct WorkerHandle {
    #[allow(dead_code)]
    id: usize,
    // We might store a thread handle if we want to join threads during shutdown
    thread: Option<thread::JoinHandle<()>>,
}

impl WorkerHandle {
    pub fn new(id: usize, thread: thread::JoinHandle<()>) -> Self {
        Self {
            id,
            thread: Some(thread),
        }
    }

    pub fn join(&mut self) {
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}

/// Worker thread main loop
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
