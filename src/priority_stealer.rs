use std::{
    cmp::Reverse,
    collections::BinaryHeap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

use crate::pool::task::{IPriority, Priority};
use crate::queue::PriorityQueue;

pub enum Steal<T> {
    Empty,
    Success(T),
    Retry,
}

pub struct PriorityInjector<T> {
    global_queue: Arc<PriorityQueue<T>>,
}

impl<T: std::cmp::Ord + IPriority> PriorityInjector<T> {
    pub fn new() -> Self {
        PriorityInjector {
            global_queue: Arc::new(PriorityQueue::new()),
        }
    }

    /// Push a task with a given priority into the global queue.
    pub fn push(&self, item: T) {
        self.global_queue.push(item);
    }

    /// Try to pop the highest priority task from the global queue.
    pub(self) fn pop(&self) -> Option<T> {
        self.global_queue.pop()
    }

    pub fn steal_batch(&self, dest: PriorityWorker<T>) -> Steal<()> {
        let count = dest.local_counter.load(Ordering::SeqCst);
        if count == 0 {
            return Steal::Empty;
        }

        let mut stolen = Vec::new();

        // Steal half of the tasks
        for _ in 0..(count + 1) / 2 {
            // ! FIXME: This could potentially be wrong. Need proper investigation.
            if let Some(task) = self.global_queue.pop() {
                stolen.push(task);
            }
        }
        dest.push_batch(stolen);
        return Steal::Success(());
    }

    fn arc_clone(&self) -> Arc<PriorityQueue<T>> {
        Arc::clone(&self.global_queue)
    }
}

/// A worker with a local priority queue.
pub struct PriorityWorker<T> {
    local: Arc<Mutex<BinaryHeap<Reverse<(Priority, usize, T)>>>>,
    local_counter: Arc<AtomicUsize>,
}

impl<T: std::cmp::Ord + IPriority> PriorityWorker<T> {
    pub fn new() -> Self {
        PriorityWorker {
            local: Arc::new(Mutex::new(BinaryHeap::new())),
            local_counter: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Push a task into the local worker queue with given priority.
    pub fn push(&self, task: T, priority: Priority) {
        let count = self.local_counter.fetch_add(1, Ordering::SeqCst);
        let mut heap = self.local.lock().unwrap();
        heap.push(std::cmp::Reverse((priority, count, task)));
    }

    pub(self) fn push_batch(&self, items: Vec<T>) {
        let mut heap = self.local.lock().unwrap();
        for (_, item) in items.into_iter().enumerate() {
            heap.push(std::cmp::Reverse((
                item.priority(),
                self.local_counter.fetch_add(1, Ordering::SeqCst),
                item,
            )));
        }
    }

    /// Pop the highest priority task from the local queue.
    pub fn pop(&self) -> Option<T> {
        let mut heap = self.local.lock().unwrap();
        self.local_counter.fetch_sub(1, Ordering::SeqCst);
        heap.pop().map(|std::cmp::Reverse((_, _, task))| task)
    }

    /// Create a stealer that can steal tasks from this worker’s queue.
    pub fn stealer(&self) -> PriorityStealer<T> {
        PriorityStealer {
            local: Arc::clone(&self.local),
            local_counter: Arc::clone(&self.local_counter),
        }
    }
}

impl<T> Clone for PriorityWorker<T> {
    fn clone(&self) -> Self {
        PriorityWorker {
            local: Arc::clone(&self.local),
            local_counter: Arc::clone(&self.local_counter),
        }
    }
}

/// A stealer that can steal tasks from a worker's local queue.
pub struct PriorityStealer<T> {
    local: Arc<Mutex<BinaryHeap<Reverse<(Priority, usize, T)>>>>,
    local_counter: Arc<AtomicUsize>,
}

impl<T: std::cmp::Ord + IPriority> PriorityStealer<T> {
    /// Steal tries to pop a task from the associated worker’s queue.
    pub fn steal(&self) -> Steal<T> {
        let mut heap = self.local.lock().unwrap();
        let count = self.local_counter.load(Ordering::SeqCst);
        if count == 0 {
            return Steal::Empty;
        }

        if let Some(std::cmp::Reverse((_, _, task))) = heap.pop() {
            self.local_counter.fetch_sub(1, Ordering::SeqCst);
            return Steal::Success(task);
        } else {
            return Steal::Retry;
        }
    }
}

pub fn prioritized_work_stealing_queues<T: std::cmp::Ord + IPriority>(
    num_workers: usize,
) -> (
    Arc<PriorityInjector<T>>,
    Vec<PriorityStealer<T>>,
    Vec<PriorityWorker<T>>,
) {
    let injector = Arc::new(PriorityInjector::new());
    let mut workers = Vec::with_capacity(num_workers);
    let mut stealers = Vec::with_capacity(num_workers);

    for _ in 0..num_workers {
        let w = PriorityWorker::new();
        stealers.push(w.stealer());
        workers.push(w);
    }

    (injector, stealers, workers)
}
