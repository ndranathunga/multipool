//! Simple queue wrapper for tasks.
use crossbeam::queue::SegQueue;
use std::sync::Arc;

pub struct TaskQueue<T> {
    inner: Arc<SegQueue<T>>,
}

impl<T> TaskQueue<T> {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(SegQueue::new()),
        }
    }

    pub fn push(&self, t: T) {
        self.inner.push(t);
    }

    pub fn pop(&self) -> Option<T> {
        self.inner.pop()
    }

    pub fn clone_inner(&self) -> Arc<SegQueue<T>> {
        Arc::clone(&self.inner)
    }
}
