//! Error types for the thread pool.

#[allow(dead_code)]
#[derive(Debug)]
pub enum PoolError {
    /// Indicates that the pool has been shut down, and no new tasks can be accepted.
    PoolShutdown,
    /// A generic join error indicating a task panicked.
    JoinError,
}

impl std::fmt::Display for PoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PoolError::PoolShutdown => write!(f, "Thread pool is shut down"),
            PoolError::JoinError => write!(f, "Task panicked or failed to join"),
        }
    }
}

impl std::error::Error for PoolError {}
