//! # multipool
//!
//! `multipool` is a Rust library that provides a configurable thread pool
//! with both a standard task queue and work-stealing capabilities.
//!
//! ## Features
//! - Spawn tasks into a shared queue.
//! - Optional work-stealing mode for improved concurrency and load balancing.
//! - Graceful shutdown.
//! - Priority-based task scheduling.
//! - Configurable number of threads.
//! - Metrics collection for monitoring thread pool activity.
//!
//! ## Usage
//!
//! ### Basic Usage
//! ```rust
//! use multipool::ThreadPoolBuilder;
//!
//! // Create a thread pool with default settings (4 threads)
//! let pool = ThreadPoolBuilder::new().build();
//!
//! // Spawn a task and retrieve its result
//! let handle = pool.spawn(|| {
//!     println!("Hello from the thread pool!");
//! });
//! handle.join().unwrap();
//!
//! // Shut down the pool
//! pool.shutdown();
//! ```
//!
//! ### Changing the Number of Threads
//! ```rust
//! use multipool::ThreadPoolBuilder;
//!
//! // Create a thread pool with 8 threads
//! let pool = ThreadPoolBuilder::new().num_threads(8).build();
//!
//! // Spawn tasks
//! for i in 0..8 {
//!     pool.spawn(move || {
//!         println!("Task {} executed", i);
//!     });
//! }
//!
//! pool.shutdown();
//! ```
//!
//! ### Work-Stealing Mode
//! ```rust
//! use multipool::ThreadPoolBuilder;
//!
//! // Create a thread pool with work-stealing enabled
//! let pool = ThreadPoolBuilder::new()
//!     .set_work_stealing()
//!     .build();
//!
//! // Spawn tasks
//! let handles: Vec<_> = (0..10)
//!     .map(|i| pool.spawn(move || println!("Task {} executed", i)))
//!     .collect();
//!
//! for handle in handles {
//!     handle.join().unwrap();
//! }
//!
//! pool.shutdown();
//! ```
//!
//! ### Priority-Based Scheduling
//! ```rust
//! use multipool::ThreadPoolBuilder;
//!
//! // Create a thread pool with priority-based task scheduling
//! let pool = ThreadPoolBuilder::new()
//!     .enable_priority()
//!     .build();
//!
//! // Spawn tasks with different priorities
//! pool.spawn_with_priority(|| println!("Low priority task"), 10);
//! pool.spawn_with_priority(|| println!("High priority task"), 1);
//!
//! pool.shutdown();
//! ```
//!
//! ### Priority Work-Stealing Mode
//! ```rust
//! use multipool::ThreadPoolBuilder;
//!
//! // Create a thread pool with priority-based work-stealing
//! let pool = ThreadPoolBuilder::new()
//!     .set_work_stealing()
//!     .enable_priority()
//!     .build();
//!
//! pool.spawn_with_priority(|| println!("Low priority task"), 10);
//! pool.spawn_with_priority(|| println!("High priority task"), 1);
//!
//! pool.shutdown();
//! ```
//!
//! ### Collecting Metrics
//! ```rust
//! use multipool::{metrics::{ThreadPoolMetrics, AtomicMetricsCollector}, ThreadPoolBuilder};
//! use std::sync::Arc;
//!
//! // Create metrics and collector
//! let metrics = Arc::new(ThreadPoolMetrics::new());
//! let collector = Arc::new(AtomicMetricsCollector::new(metrics.clone()));
//!
//! // Create a thread pool with the metrics collector
//! let pool = ThreadPoolBuilder::new()
//!     .num_threads(4)
//!     .with_metrics_collector(collector)
//!     .build();
//!
//! // Spawn tasks
//! for i in 0..5 {
//!     pool.spawn(move || println!("Task {} executed", i));
//! }
//!
//! // Inspect metrics
//! println!("Queued tasks: {}", metrics.queued_tasks.load(std::sync::atomic::Ordering::SeqCst));
//! println!("Running tasks: {}", metrics.running_tasks.load(std::sync::atomic::Ordering::SeqCst));
//! println!("Completed tasks: {}", metrics.completed_tasks.load(std::sync::atomic::Ordering::SeqCst));
//! println!("Active threads: {}", metrics.active_threads.load(std::sync::atomic::Ordering::SeqCst));
//!
//! pool.shutdown();
//! ```

mod errors;
pub mod metrics;
pub mod pool;
mod priority_stealer;
mod queue;
mod stealer;

#[allow(dead_code)]
#[allow(unused_imports)]
use pool::task::BoxedTask;
pub use pool::ThreadPoolBuilder;

/// Runs a set of tasks in traditional multi-threading mode (without the thread pool).
///
/// This function spawns one thread per task and waits for all threads to complete.
///
/// # Arguments
/// - `tasks`: A vector of boxed tasks (`BoxedTask`) to execute.
///
/// # Example
/// ```rust
/// use multipool::run_traditional;
///
/// let tasks: Vec<_> = (0..4)
///     .map(|i| Box::new(move || println!("Task {} executed", i)) as Box<dyn FnOnce() + Send>)
///     .collect();
///
/// run_traditional(tasks);
/// ```
#[cfg(any(debug_assertions, test, feature = "bench"))]
pub fn run_traditional(tasks: Vec<BoxedTask>) {
    let handles: Vec<_> = tasks
        .into_iter()
        .map(|task| std::thread::spawn(task))
        .collect();

    for h in handles {
        let _ = h.join();
    }
}
