//! Simple queue wrapper for tasks.
//!
//! This module provides two queue implementations for managing tasks:
//! - `TaskQueue<T>`: A simple queue based on `crossbeam::queue::SegQueue`.
//! - `PriorityQueue<T>`: A priority queue that supports task prioritization
//!   and ensures FIFO order for tasks with the same priority level.

use crossbeam::queue::SegQueue;
use std::collections::BinaryHeap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::{cmp::Reverse, sync::Mutex};

use crate::pool::task::{IPriority, Priority};

/// A simple queue for managing tasks.
///
/// This is a thread-safe wrapper around `crossbeam::queue::SegQueue`,
/// which provides a lock-free queue for high-performance task management.
pub struct TaskQueue<T> {
    inner: Arc<SegQueue<T>>,
}

#[allow(dead_code)]
impl<T> TaskQueue<T> {
    /// Creates a new, empty `TaskQueue`.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(SegQueue::new()),
        }
    }

    /// Adds a task to the queue.
    ///
    /// # Arguments
    /// - `t`: The task to add to the queue.
    pub fn push(&self, t: T) {
        self.inner.push(t);
    }

    /// Removes and returns a task from the queue.
    ///
    /// Returns `None` if the queue is empty.
    pub fn pop(&self) -> Option<T> {
        self.inner.pop()
    }

    /// Returns a clone of the underlying queue.
    ///
    /// This can be used for sharing the queue between multiple components.
    pub fn clone_inner(&self) -> Arc<SegQueue<T>> {
        Arc::clone(&self.inner)
    }
}

/// A priority queue for managing tasks with priority levels.
///
/// This queue uses a min-heap internally to order tasks based on their priority.
/// Tasks with the same priority are processed in FIFO order using a counter.
///
/// # Type Parameters
/// - `T`: The type of elements to store in the queue. Must implement `std::cmp::Ord` and `IPriority`.
pub struct PriorityQueue<T> {
    /// The underlying heap used to store tasks with priority.
    heap: Arc<Mutex<BinaryHeap<Reverse<(Priority, usize, T)>>>>, // Reverse for min-heap behavior
    /// A counter used to ensure FIFO order for tasks with the same priority.
    counter: Arc<AtomicUsize>, // To ensure FIFO order for tasks with the same priority
                               // ! FIXME: This is a temporary solution. Need to find a better way to handle priority. counter never decreases. could overflow.
}

#[allow(dead_code)]
impl<T: std::cmp::Ord + IPriority> PriorityQueue<T> {
    /// Creates a new, empty `PriorityQueue`.
    pub fn new() -> Self {
        PriorityQueue {
            heap: Arc::new(Mutex::new(BinaryHeap::new())),
            counter: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Adds a task to the priority queue.
    ///
    /// Tasks are ordered based on their priority (lower values have higher priority).
    /// If two tasks have the same priority, the one added earlier will be processed first.
    ///
    /// # Arguments
    /// - `item`: The task to add to the queue.
    pub fn push(&self, item: T) {
        let mut heap = self.heap.lock().unwrap();
        let count = self.counter.load(Ordering::SeqCst);
        heap.push(Reverse((item.priority(), count, item)));
        self.counter.fetch_add(1, Ordering::SeqCst);
    }

    /// Removes and returns the highest-priority task from the queue.
    ///
    /// Returns `None` if the queue is empty.
    pub fn pop(&self) -> Option<T> {
        let mut heap = self.heap.lock().unwrap();
        heap.pop().map(|Reverse((_, _, task))| task)
    }

    /// Returns a clone of the underlying heap.
    ///
    /// This can be used for shared access to the queue's internal state.
    pub fn clone_inner(&self) -> Arc<Mutex<BinaryHeap<Reverse<(Priority, usize, T)>>>> {
        self.heap.clone()
    }

    /// Returns the number of tasks currently in the queue.
    pub fn len(&self) -> usize {
        self.heap.lock().unwrap().len()
    }
}

impl<T: std::cmp::Ord + IPriority> Clone for PriorityQueue<T> {
    /// Creates a new `PriorityQueue` that shares the same internal heap and counter.
    ///
    /// This allows multiple components to interact with the same queue instance.
    fn clone(&self) -> Self {
        PriorityQueue {
            heap: self.heap.clone(),
            counter: Arc::clone(&self.counter),
        }
    }
}
