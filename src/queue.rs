//! Simple queue wrapper for tasks.

use crossbeam::queue::SegQueue;
use std::collections::BinaryHeap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::{cmp::Reverse, sync::Mutex};

use crate::pool::task::{IPriority, Priority};

pub struct TaskQueue<T> {
    inner: Arc<SegQueue<T>>,
}

#[allow(dead_code)]
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

pub struct PriorityQueue<T> {
    heap: Arc<Mutex<BinaryHeap<Reverse<(Priority, usize, T)>>>>, // Reverse for min-heap behavior
    counter: Arc<AtomicUsize>, // To ensure FIFO order for tasks with the same priority
    // ! FIXME: This is a temporary solution. Need to find a better way to handle priority. counter never decreases. could overflow.
}

#[allow(dead_code)]
impl<T: std::cmp::Ord + IPriority> PriorityQueue<T> {
    pub fn new() -> Self {
        PriorityQueue {
            heap: Arc::new(Mutex::new(BinaryHeap::new())),
            counter: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn push(&self, item: T) {
        let mut heap = self.heap.lock().unwrap();
        let count = self.counter.load(Ordering::SeqCst);
        heap.push(Reverse((item.priority(), count, item)));
        self.counter.fetch_add(1, Ordering::SeqCst);
    }

    pub fn pop(&self) -> Option<T> {
        let mut heap = self.heap.lock().unwrap();
        heap.pop().map(|Reverse((_, _, task))| task)
    }

    pub fn clone_inner(&self) -> Arc<Mutex<BinaryHeap<Reverse<(Priority, usize, T)>>>> {
        self.heap.clone()
    }

    pub fn len(&self) -> usize {
        self.heap.lock().unwrap().len()
    }
}

impl<T: std::cmp::Ord + IPriority> Clone for PriorityQueue<T> {
    fn clone(&self) -> Self {
        PriorityQueue {
            heap: self.heap.clone(),
            counter: Arc::clone(&self.counter),
        }
    }
}
