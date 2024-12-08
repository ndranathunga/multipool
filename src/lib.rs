//! multipool
//
// `multipool` is a Rust library that provides a configurable thread pool
// with both a standard task queue and work-stealing capabilities.
//

//? ## Features
//? - Spawn tasks into a shared queue.
//? - Optional work-stealing mode for improved concurrency and load balancing.
//? - Graceful shutdown.
//? - Configurable number of threads.

mod errors;
pub mod pool;
mod queue;
mod stealer;

pub use pool::{ThreadPool, ThreadPoolBuilder};
