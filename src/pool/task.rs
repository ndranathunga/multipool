//! Task abstraction for the thread pool.
//!
//! This module provides the core building blocks for tasks within the thread pool, including
//! support for priority-based tasks and task handles for managing results. It also defines
//! utilities to spawn tasks and abstract their execution.

pub type BoxedTask = Box<dyn FnOnce() + Send + 'static>;

use std::cmp::Ordering;
use std::panic::AssertUnwindSafe;
use std::sync::mpsc::channel;

/// Represents the priority of a task.
///
/// Lower values indicate higher priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Priority(pub usize);

pub struct PriorityTask {
    pub(crate) task: BoxedTask,
    priority: Priority,
}

/// A task with an associated priority.
///
/// This struct encapsulates a task (a function or closure) and its priority level,
/// allowing the thread pool to manage tasks based on their importance.
impl PriorityTask {
    /// Creates a new `PriorityTask`.
    ///
    /// # Arguments
    /// - `f`: The closure or function to execute as the task.
    /// - `priority`: The priority of the task, where lower values have higher priority.
    ///
    /// # Returns
    /// A new `PriorityTask` instance.
    pub fn new<F>(f: F, priority: Priority) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        Self {
            task: Box::new(f),
            priority,
        }
    }

    /// Returns the priority of the task.
    pub fn priority(&self) -> Priority {
        self.priority
    }
}

impl IPriority for PriorityTask {
    /// Retrieves the priority of the task.
    fn priority(&self) -> Priority {
        self.priority.clone()
    }
}

// Implement `Ord` for `PriorityTask` to enable comparisons based on priority.
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

/// A handle to a task's result.
///
/// This allows the user to retrieve the result of a task once it has completed.
/// The result can be accessed using the `join` method.
impl<T> TaskHandle<T> {
    /// Waits for the task to complete and returns its result.
    ///
    /// # Returns
    /// A `Result` containing the task's output or a panic error if the task panicked.
    ///
    /// # Panics
    /// This method will panic if the channel is unexpectedly closed.
    pub fn join(self) -> std::thread::Result<T> {
        self.receiver.recv().expect("Task result channel closed")
    }
}

/// Spawns a task and returns the task along with a handle to retrieve its result.
///
/// The task is wrapped in a `BoxedTask` to allow execution within the thread pool.
/// The result of the task can be retrieved using the returned `TaskHandle`.
///
/// # Arguments
/// - `f`: The closure or function to execute as the task.
///
/// # Returns
/// A tuple containing:
/// - A `BoxedTask` representing the task.
/// - A `TaskHandle<T>` for accessing the task's result.
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

/// A trait for tasks with priorities.
///
/// This trait abstracts the ability to retrieve a priority from a task, allowing
/// various task types to be integrated into the priority queue system.
pub trait IPriority {
    /// Retrieves the priority of the task.
    fn priority(&self) -> Priority;
}
