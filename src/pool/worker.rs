//! Worker logic for the thread pool

use super::task::BoxedTask;
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
pub fn worker_loop<F>(running: Arc<AtomicBool>, mut fetch_task: F)
where
    F: FnMut() -> Option<BoxedTask>,
{
    while running.load(Ordering::Acquire) {
        if let Some(task) = fetch_task() {
            task();
        } else {
            std::thread::yield_now();
        }
    }
}
