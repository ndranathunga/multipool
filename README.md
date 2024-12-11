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

###
