//! Worker logic for the thread pool

use super::task::BoxedTask;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;

pub struct WorkerHandle {
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
pub fn worker_loop(
    id: usize,
    running: Arc<AtomicBool>,
    fetch_task: Arc<dyn Fn(usize) -> Option<BoxedTask> + Send + Sync>,
) {
    while running.load(Ordering::Acquire) {
        if let Some(task) = fetch_task(id) {
            task();
        } else {
            // Optional: use a small sleep or yield to avoid busy spinning
            std::thread::yield_now();
        }
    }
}
