# Multipool

![rustc](https://img.shields.io/badge/rustc-1.78+-blue?logo=rust)
[![License](https://img.shields.io/badge/license-MIT-blue)](https://opensource.org/licenses/MIT)
[![Crates.io](https://img.shields.io/crates/v/multipool)](https://crates.io/crates/multipool)
[![Docs](https://docs.rs/multipool/badge.svg)](https://docs.rs/multipool)

**Multipool** is a Rust library that provides a configurable and efficient thread pool with optional work-stealing capabilities. It allows you to easily spawn tasks and handle concurrent workloads, while also offering a builder API to customize the number of threads and choose between the global queue or the work-stealing mode.

## Features

- **Configurable Threads:** Specify the number of worker threads for concurrent task handling.
- **Global Queue or Work-Stealing:** Choose between simple global queues or efficient work-stealing for load balancing.
- **Priority Scheduling:** Assign priorities to tasks for more control over execution order.
- **Metrics and Monitoring:** Track active threads, queued tasks, running tasks and completed tasks in real-time.
<!-- - **Graceful Shutdown:** Ensure all tasks complete properly before shutting down. -->

## Installation

Add `multipool` as a dependency in your `Cargo.toml`:

```toml
[dependencies]
multipool = "0.2"
```

## Examples

### Basic Usage with Work-Stealing

```rust
use multipool::ThreadPoolBuilder;

fn main() {
    let pool = ThreadPoolBuilder::new()
        .num_threads(4)
        .set_work_stealing()
        .build();

    for i in 0..10 {
        pool.spawn(move || println!("Task {} running", i));
    }

    pool.shutdown();
}
```

### Using a Global Queue

```rust
use multipool::ThreadPoolBuilder;

fn main() {
    let pool = ThreadPoolBuilder::new()
        .num_threads(4)
        .set_work_stealing()
        .build();

    for i in 0..10 {
        pool.spawn(move || println!("Task {} running", i));
    }

    pool.shutdown();
}
```

### Using Priority Scheduling

With version `0.2.0`, **Multipool** supports priority scheduling. You can assign priorities to tasks when using the `priority` mode.

```rust
use multipool::ThreadPoolBuilder;

fn main() {
    let pool = ThreadPoolBuilder::new()
        .num_threads(4)
        .set_work_stealing() // Optional: Enable work-stealing
        .enable_priority()
        .build();

    for i in 0..10 {
        pool.spawn_with_priority(move || println!("Task with priority {} running", i), i);
    }

    pool.shutdown();
}
```

In this example:

- Tasks with higher priority (low `i` values) will execute before lower-priority tasks.
- The `spawn_with_priority` method allows you to specify the priority level for each task.

### Using `TaskHandle` to Retrieve Results

In some cases, you might want to retrieve the result of a task after it completes. This can be done using the `TaskHandle` returned by the `spawn` or `spawn_with_priority` method.

```rust
fn main() {
    let pool = multipool::ThreadPoolBuilder::new().num_threads(4).build();

    let handle = pool.spawn(|| {
        println!("Hello from the thread pool!");
        10 // Returning a value from the task
    });

    let res = handle.join().unwrap(); // Wait for the task to complete and retrieve the result
    println!("Result from task: {}", res);

    pool.shutdown();
}
```

In this example:

- The `spawn` method returns a `TaskHandle`.
- The `join` method blocks until the task completes and retrieves the result.
- Ensure to handle potential errors from `join` gracefully, such as using `unwrap` or other error-handling mechanisms.

### Live Metrics Monitoring

With version `0.2.3`, **Multipool** introduces live metrics monitoring to track the thread pool's performance in real-time. You can monitor the following metrics:

- **Queued Tasks:** The number of tasks waiting to be executed.
- **Running Tasks:** The number of tasks currently being executed.
- **Completed Tasks:** The number of tasks that have finished execution.
- **Active Threads:** The number of threads currently active in the thread pool.

Example:

```rust
use multipool::{ThreadPoolBuilder, metrics::{AtomicMetricsCollector, ThreadPoolMetrics}};
use std::sync::Arc;
use std::time::Duration;
use std::thread;

fn main() {
    // Create metrics and collector
    let metrics = Arc::new(ThreadPoolMetrics::new());
    let collector = Arc::new(AtomicMetricsCollector::new(metrics.clone()));

    // Create a thread pool with the metrics collector
    let pool = ThreadPoolBuilder::new()
        .num_threads(4)
        .with_metrics_collector(collector)
        .build();

    for i in 0..10 {
        pool.spawn(move || {
            println!("Task {} running", i);
            thread::sleep(Duration::from_millis(100)); // Simulated work
        });
    }

    // Spawn a monitoring thread
    let metrics_clone = metrics.clone();
    let monitor_handle = thread::spawn(move || {
        for _ in 0..10 {
            println!("\n--- Metrics ---");
            println!("Queued tasks: {}", metrics_clone.queued_tasks.load(Ordering::SeqCst));
            println!("Running tasks: {}", metrics_clone.running_tasks.load(Ordering::SeqCst));
            println!("Completed tasks: {}", metrics_clone.completed_tasks.load(Ordering::SeqCst));
            println!("Active threads: {}", metrics_clone.active_threads.load(Ordering::SeqCst));

            thread::sleep(Duration::from_secs(1));
        }
    });

    pool.shutdown();
    monitor_handle.join().unwrap();
}
```

In this example:

- Metrics are updated in real-time by the thread pool.
- A monitoring thread periodically logs the metrics to the terminal.
- The metrics are accessible through the `ThreadPoolMetrics` struct, which uses atomic counters for thread-safe updates.

## Documentation

- [Crate on crates.io](https://crates.io/crates/multipool)
- [API Documentation](https://docs.rs/multipool)

## Feedback

If you encounter any issues or have feature requests, please open an issue in the [GitHub repository issues](https://github.com/ndranathunga/multipool/issues).
