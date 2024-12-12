//! # Macros for `multipool`
//!
//! This module contains macros to simplify usage of the `multipool` library. These macros
//! reduce boilerplate and improve ergonomics when configuring thread pools, spawning tasks,
//! logging metrics, and running traditional multi-threaded tasks.

/// Simplifies spawning tasks into the thread pool.
///
/// This macro supports both non-priority and priority task submission.
///
/// # Examples
/// ```rust
/// use multipool::{ThreadPoolBuilder, spawn_task};
///
/// let pool = ThreadPoolBuilder::new().build();
///
/// // Spawn a task without priority
/// let handle = spawn_task!(pool, || println!("Task without priority"));
///
/// // Spawn a task with priority
/// let priority_handle = spawn_task!(pool, || println!("Task with priority"), priority: 1);
///
/// handle.join().unwrap();
/// priority_handle.join().unwrap();
/// pool.shutdown();
/// ```
#[macro_export]
macro_rules! spawn_task {
    ($pool:expr, $task:expr) => {
        $pool.spawn($task)
    };
    ($pool:expr, $task:expr, priority: $priority:expr) => {
        $pool.spawn_with_priority($task, $priority)
    };
}

/// Logs the current metrics of the thread pool.
///
/// This macro prints key metrics such as the number of queued, running, and completed tasks,
/// as well as the number of active threads.
///
/// # Example
/// ```rust
/// use multipool::{metrics::{ThreadPoolMetrics, AtomicMetricsCollector}, ThreadPoolBuilder, log_metrics};
/// use std::sync::Arc;
///
/// let metrics = Arc::new(ThreadPoolMetrics::new());
/// let collector = Arc::new(AtomicMetricsCollector::new(metrics.clone()));
/// let pool = ThreadPoolBuilder::new().with_metrics_collector(collector).build();
///
/// log_metrics!(metrics);
/// pool.shutdown();
/// ```
#[macro_export]
macro_rules! log_metrics {
    ($metrics:expr) => {
        println!(
            "Queued tasks: {}",
            $metrics
                .queued_tasks
                .load(std::sync::atomic::Ordering::SeqCst)
        );
        println!(
            "Running tasks: {}",
            $metrics
                .running_tasks
                .load(std::sync::atomic::Ordering::SeqCst)
        );
        println!(
            "Completed tasks: {}",
            $metrics
                .completed_tasks
                .load(std::sync::atomic::Ordering::SeqCst)
        );
        println!(
            "Active threads: {}",
            $metrics
                .active_threads
                .load(std::sync::atomic::Ordering::SeqCst)
        );
    };
}

/// Creates a thread pool with various configurations.
///
/// This macro supports thread pool creation with custom numbers of threads, work-stealing,
/// and priority-based scheduling.
///
/// # Examples
/// ```rust
/// use multipool::create_thread_pool;
///
/// let pool = create_thread_pool!(num_threads: 8, work_stealing: true, priority: true);
/// pool.shutdown();
/// ```
#[macro_export]
macro_rules! create_thread_pool {
    (num_threads: $num:expr) => {
        ThreadPoolBuilder::new().num_threads($num).build()
    };
    (num_threads: $num:expr, work_stealing: true) => {
        ThreadPoolBuilder::new()
            .num_threads($num)
            .set_work_stealing()
            .build()
    };
    (num_threads: $num:expr, priority: true) => {
        ThreadPoolBuilder::new()
            .num_threads($num)
            .enable_priority()
            .build()
    };
    (num_threads: $num:expr, work_stealing: true, priority: true) => {
        ThreadPoolBuilder::new()
            .num_threads($num)
            .set_work_stealing()
            .enable_priority()
            .build()
    };
}
