//! Task abstraction for the thread pool.

pub type BoxedTask = Box<dyn FnOnce() + Send + 'static>;

use std::cmp::Ordering;
use std::panic::AssertUnwindSafe;
use std::sync::mpsc::channel;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Priority(pub usize);

pub struct PriorityTask {
    pub(crate) task: BoxedTask,
    priority: Priority,
}

impl PriorityTask {
    pub fn new<F>(f: F, priority: Priority) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        Self {
            task: Box::new(f),
            priority,
        }
    }

    pub fn priority(&self) -> Priority {
        self.priority
    }
}

impl IPriority for PriorityTask {
    fn priority(&self) -> Priority {
        self.priority.clone()
    }
}

// Implement `Ord` for `Task` based on `priority`
impl Ord for PriorityTask {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority.cmp(&other.priority)
    }
}

impl PartialOrd for PriorityTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for PriorityTask {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl Eq for PriorityTask {}

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


pub trait IPriority {
    fn priority(&self) -> Priority;
}