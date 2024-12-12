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
- **Macros:** Simplify task submission, metrics logging, and thread pool creation with ergonomic macros.
- **Typed-State Builder Pattern:** The ThreadPoolBuilder ensures that thread pools are built correctly with a strongly-typed API, enforcing valid combinations of features like priority scheduling and work-stealing.
- **Graceful Shutdown:** Cleanly shut down the thread pool while ensuring that all tasks are completed.
- **Customizable Metrics Collector:** Integrate your own metrics collection logic for application-specific monitoring and analytics.

## Installation

Add `multipool` as a dependency in your `Cargo.toml`:

```toml
[dependencies]
multipool = "0.2"
```

## Documentation

- [Crate on crates.io](https://crates.io/crates/multipool) 
- [API Documentation](https://docs.rs/multipool) **(with examples)**

## Feedback

If you encounter any issues or have feature requests, please open an issue in the [GitHub repository issues](https://github.com/ndranathunga/multipool/issues).
