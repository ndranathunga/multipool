# Multipool

**Multipool** is a Rust library that provides a configurable and efficient thread pool with optional work-stealing capabilities. It allows you to easily spawn tasks and handle concurrent workloads, while also offering a builder API to customize the number of threads and choose between the global queue or the work-stealing mode.

## Features

- **Configurable Threads:** Easily specify the number of worker threads to handle tasks concurrently.
- **Global Queue or Work-Stealing:** Choose between the simpler global queue or the work-stealing mode for load balancing.
- **Graceful Shutdown:** Properly shut down the thread pool, ensuring all tasks are completed before exiting.

## Installation

Add `multipool` as a dependency in your `Cargo.toml`:

```toml
[dependencies]
multipool = "0.1.0"
```
