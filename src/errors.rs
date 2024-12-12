//! Error types for the thread pool.
//!
//! This module defines errors that may occur during the operation of the thread pool.
//! These errors include situations such as task submission to a shut-down pool or task execution failures.

/// Represents errors that can occur in the thread pool.
///
/// This enum provides error types for common issues encountered while managing
/// and executing tasks in the thread pool.
#[allow(dead_code)]
#[derive(Debug)]
pub enum PoolError {
    /// The thread pool has been shut down, and no new tasks can be accepted.
    PoolShutdown,
    /// A generic error indicating that a task panicked or failed to join.
    JoinError,
}

impl std::fmt::Display for PoolError {
    /// Formats the `PoolError` for display.
    ///
    /// # Arguments
    /// - `f`: A mutable reference to the formatter.
    ///
    /// # Returns
    /// A `Result` indicating whether the formatting was successful.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PoolError::PoolShutdown => write!(f, "Thread pool is shut down"),
            PoolError::JoinError => write!(f, "Task panicked or failed to join"),
        }
    }
}

impl std::error::Error for PoolError {}
