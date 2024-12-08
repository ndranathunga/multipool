use crossbeam::deque::{Injector, Steal, Stealer, Worker};
use std::sync::Arc;

pub struct WorkStealingQueues<T> {
    injector: Arc<Injector<T>>,
    workers: Vec<Worker<T>>,
    stealers: Vec<Stealer<T>>,
}

impl<T> WorkStealingQueues<T> {
    pub fn new(num_workers: usize) -> Self {
        let injector = Arc::new(Injector::new());
        let mut workers = Vec::with_capacity(num_workers);
        let mut stealers = Vec::with_capacity(num_workers);

        for _ in 0..num_workers {
            let w = Worker::new_fifo();
            stealers.push(w.stealer());
            workers.push(w);
        }

        Self {
            injector,
            workers,
            stealers,
        }
    }

    pub fn push(&self, task: T) {
        self.injector.push(task);
    }

    /// This is typically used by workers:
    pub fn pop(&self, idx: usize) -> Option<T> {
        // First try to pop from the worker’s own queue
        if let Some(task) = self.workers[idx].pop() {
            return Some(task);
        }

        // Otherwise, try stealing from injector
        match self.injector.steal_batch(&self.workers[idx]) {
            Steal::Success(t) => return Some(t),
            Steal::Empty => {}
            Steal::Retry => {}
        }

        None
    }

    pub fn steal_work(&self, from_idx: usize) -> Option<T> {
        // Attempt to steal from another worker
        for (i, stealer) in self.stealers.iter().enumerate() {
            if i == from_idx {
                continue;
            }
            match stealer.steal() {
                Steal::Success(t) => return Some(t),
                Steal::Empty | Steal::Retry => continue,
            }
        }
        None
    }

    pub fn worker_count(&self) -> usize {
        self.workers.len()
    }
}
