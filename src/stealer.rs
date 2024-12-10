use crossbeam::deque::{Injector, Stealer, Worker};
use std::sync::Arc;

pub fn work_stealing_queues<T>(
    num_workers: usize,
) -> (Arc<Injector<T>>, Vec<Stealer<T>>, Vec<Worker<T>>) {
    let injector = Arc::new(Injector::new());
    let mut workers = Vec::with_capacity(num_workers);
    let mut stealers = Vec::with_capacity(num_workers);

    for _ in 0..num_workers {
        let w = Worker::new_fifo();
        stealers.push(w.stealer());
        workers.push(w);
    }

    (injector, stealers, workers)
}
