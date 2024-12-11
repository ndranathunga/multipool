pub trait MetricsCollector: Send + Sync {
    fn on_task_submitted(&self);
    fn on_task_started(&self);
    fn on_task_completed(&self);
    fn on_worker_started(&self);
    fn on_worker_stopped(&self);
}

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub struct ThreadPoolMetrics {
    pub queued_tasks: AtomicUsize,
    pub running_tasks: AtomicUsize,
    pub completed_tasks: AtomicUsize,
    pub active_threads: AtomicUsize,
}

impl ThreadPoolMetrics {
    pub fn new() -> Self {
        Self {
            queued_tasks: AtomicUsize::new(0),
            running_tasks: AtomicUsize::new(0),
            completed_tasks: AtomicUsize::new(0),
            active_threads: AtomicUsize::new(0),
        }
    }
}

/// a default collector that uses atomic counters
pub struct AtomicMetricsCollector {
    pub metrics: Arc<ThreadPoolMetrics>,
}

impl AtomicMetricsCollector {
    pub fn new(metrics: Arc<ThreadPoolMetrics>) -> Self {
        Self { metrics }
    }
}

impl MetricsCollector for AtomicMetricsCollector {
    fn on_task_submitted(&self) {
        self.metrics.queued_tasks.fetch_add(1, Ordering::SeqCst);
    }
    fn on_task_started(&self) {
        self.metrics.queued_tasks.fetch_sub(1, Ordering::SeqCst);
        self.metrics.running_tasks.fetch_add(1, Ordering::SeqCst);
    }
    fn on_task_completed(&self) {
        self.metrics.running_tasks.fetch_sub(1, Ordering::SeqCst);
        self.metrics.completed_tasks.fetch_add(1, Ordering::SeqCst);
    }
    fn on_worker_started(&self) {
        self.metrics.active_threads.fetch_add(1, Ordering::SeqCst);
    }
    fn on_worker_stopped(&self) {
        self.metrics.active_threads.fetch_sub(1, Ordering::SeqCst);
    }
}
