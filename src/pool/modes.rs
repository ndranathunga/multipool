//! Task fetch modes for the thread pool.
//!
//! This module defines traits and structures that represent different task fetching strategies
//! used by the thread pool. These modes determine how tasks are fetched and scheduled
//! for execution, and whether they support task prioritization.

/// A trait representing a task fetching mode.
///
/// Implementations of this trait define how tasks are fetched from the task queues.
/// This trait is the base for more specific modes, such as priority and non-priority modes.
pub trait TaskFetchMode {
    /// Returns the name of the task fetching mode.
    fn mode(&self) -> &'static str;
}

/// A marker trait for task fetching modes that do not support priorities.
///
/// Modes implementing this trait fetch tasks without considering their priorities.
pub trait NonPriorityTaskFetchMode: TaskFetchMode {}

/// A marker trait for task fetching modes that support task priorities.
///
/// Modes implementing this trait fetch tasks based on their priorities.
pub trait PriorityTaskFetchMode: TaskFetchMode {}

/// A task fetching mode using a global queue.
///
/// In this mode, all tasks are placed into a single global queue, and workers fetch
/// tasks directly from it without considering task priorities.
pub struct GlobalQueueMode;

impl TaskFetchMode for GlobalQueueMode {
    fn mode(&self) -> &'static str {
        "GlobalQueue"
    }
}

impl NonPriorityTaskFetchMode for GlobalQueueMode {}

/// A task fetching mode using work stealing.
///
/// In this mode, each worker has its own local queue. Workers fetch tasks from their
/// local queues, and if a worker's queue is empty, it can steal tasks from other workers.
/// This mode does not consider task priorities.
pub struct WorkStealingMode;

impl TaskFetchMode for WorkStealingMode {
    fn mode(&self) -> &'static str {
        "WorkStealing"
    }
}

impl NonPriorityTaskFetchMode for WorkStealingMode {}

/// A task fetching mode using a priority global queue.
///
/// In this mode, all tasks are placed into a single global queue that orders tasks
/// based on their priorities. Workers fetch the highest-priority tasks from the queue.
pub struct PriorityGlobalQueueMode;

impl TaskFetchMode for PriorityGlobalQueueMode {
    fn mode(&self) -> &'static str {
        "PriorityGlobalQueue"
    }
}

impl PriorityTaskFetchMode for PriorityGlobalQueueMode {}

/// A task fetching mode using priority-based work stealing.
///
/// In this mode, each worker has its own local priority queue. Workers fetch tasks
/// based on priority from their local queues, and can steal tasks from other workers
/// if their own queues are empty.
pub struct PriorityWorkStealingMode;

impl TaskFetchMode for PriorityWorkStealingMode {
    fn mode(&self) -> &'static str {
        "PriorityWorkStealing"
    }
}

impl PriorityTaskFetchMode for PriorityWorkStealingMode {}
