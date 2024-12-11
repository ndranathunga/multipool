# Multipool

**Multipool** is a Rust library that provides a configurable and efficient thread pool with optional work-stealing capabilities. It allows you to easily spawn tasks and handle concurrent workloads, while also offering a builder API to customize the number of threads and choose between the global queue or the work-stealing mode.

## Features

- **Configurable Threads:** Specify the number of worker threads for concurrent task handling.
- **Global Queue or Work-Stealing:** Choose between simple global queues or efficient work-stealing for load balancing.
- **Priority Scheduling:** Assign priorities to tasks for more control over execution order.
- **Graceful Shutdown:** Ensure all tasks complete properly before shutting down.

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

## Documentation

- [Crate on crates.io](https://crates.io/crates/multipool)
- [API Documentation](https://docs.rs/multipool)

## Feedback

If you encounter any issues or have feature requests, please open an issue in the [GitHub repository issues](https://github.com/ndranathunga/multipool/issues).
