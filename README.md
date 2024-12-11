# Multipool

**Multipool** is a Rust library that provides a configurable and efficient thread pool with optional work-stealing capabilities. It allows you to easily spawn tasks and handle concurrent workloads, while also offering a builder API to customize the number of threads and choose between the global queue or the work-stealing mode.

## Features

- **Configurable Threads:** Specify the number of worker threads for concurrent task handling.
- **Global Queue or Work-Stealing:** Choose between simple global queues or efficient work-stealing for load balancing.
- **Graceful Shutdown:** Ensure all tasks complete properly before shutting down.

## Installation

Add `multipool` as a dependency in your `Cargo.toml`:

```toml
[dependencies]
multipool = "0.1"
```

## Examples

### Basic Usage with Work-Stealing

```rust
use multipool::ThreadPoolBuilder;

fn main() {
    let pool = ThreadPoolBuilder::new()
        .num_threads(4)
        .work_stealing(true)
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
        .work_stealing(false)
        .build();

    for i in 0..10 {
        pool.spawn(move || println!("Task {} running", i));
    }

    pool.shutdown();
}
```

### Using `TaskHandle` to Retrieve Results

In some cases, you might want to retrieve the result of a task after it completes. This can be done using the `TaskHandle` returned by the `spawn` method.

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

## Feedback

If you encounter any issues or have feature requests, please open an issue in the [GitHub repository issues](https://github.com/ndranathunga/multipool/issues).

