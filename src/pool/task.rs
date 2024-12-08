//! Task abstraction for the thread pool.

pub type BoxedTask = Box<dyn FnOnce() + Send + 'static>;

use std::panic::AssertUnwindSafe;
use std::sync::mpsc::channel;

pub struct TaskHandle<T> {
    receiver: std::sync::mpsc::Receiver<std::thread::Result<T>>,
}

impl<T> TaskHandle<T> {
    pub fn join(self) -> std::thread::Result<T> {
        self.receiver.recv().expect("Task result channel closed")
    }
}

pub fn spawn_task<F, T>(f: F) -> (BoxedTask, TaskHandle<T>)
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let (tx, rx) = channel();
    let task = Box::new(move || {
        let res = std::panic::catch_unwind(AssertUnwindSafe(f));
        let _ = tx.send(res);
    });
    (task, TaskHandle { receiver: rx })
}
