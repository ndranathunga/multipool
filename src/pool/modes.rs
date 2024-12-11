pub trait TaskFetchMode {
    fn mode(&self) -> &'static str;
}

pub trait NonPriorityTaskFetchMode: TaskFetchMode {}
pub trait PriorityTaskFetchMode: TaskFetchMode {}

pub struct GlobalQueueMode;

impl TaskFetchMode for GlobalQueueMode {
    fn mode(&self) -> &'static str {
        "GlobalQueue"
    }
}

impl NonPriorityTaskFetchMode for GlobalQueueMode {}

pub struct WorkStealingMode;

impl TaskFetchMode for WorkStealingMode {
    fn mode(&self) -> &'static str {
        "WorkStealing"
    }
}

impl NonPriorityTaskFetchMode for WorkStealingMode {}

pub struct PriorityGlobalQueueMode;

impl TaskFetchMode for PriorityGlobalQueueMode {
    fn mode(&self) -> &'static str {
        "PriorityGlobalQueue"
    }
}

impl PriorityTaskFetchMode for PriorityGlobalQueueMode {}

pub struct PriorityWorkStealingMode;

impl TaskFetchMode for PriorityWorkStealingMode {
    fn mode(&self) -> &'static str {
        "PriorityWorkStealing"
    }
}

impl PriorityTaskFetchMode for PriorityWorkStealingMode {}
