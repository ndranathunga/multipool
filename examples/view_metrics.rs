use multipool::{
    metrics::{AtomicMetricsCollector, ThreadPoolMetrics},
    ThreadPoolBuilder,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;

fn main() {
    // Create metrics and collector
    let metrics = Arc::new(ThreadPoolMetrics::new());
    let collector = Arc::new(AtomicMetricsCollector::new(metrics.clone()));

    // Create a thread pool with the metrics collector
    let pool = ThreadPoolBuilder::new()
        .num_threads(4)
        .with_metrics_collector(collector)
        .build();

    // Create a flag to stop monitoring
    let running = Arc::new(AtomicBool::new(true));

    // Spawn a monitoring thread to display live updates
    let metrics_clone = metrics.clone();
    let running_clone = running.clone();
    let monitor_handle = thread::spawn(move || {
        while running_clone.load(Ordering::Acquire) {
            let queued = metrics_clone.queued_tasks.load(Ordering::SeqCst);
            let running = metrics_clone.running_tasks.load(Ordering::SeqCst);
            let completed = metrics_clone.completed_tasks.load(Ordering::SeqCst);
            let active_threads = metrics_clone.active_threads.load(Ordering::SeqCst);

            println!("\n--- Metrics ---");
            println!("Queued tasks: {}", queued);
            println!("Running tasks: {}", running);
            println!("Completed tasks: {}", completed);
            println!("Active threads: {}", active_threads);

            thread::sleep(Duration::from_millis(80)); // Update every 500ms
        }
    });

    // Spawn tasks with priorities
    for _ in 0..10 {
        pool.spawn(
            move || {
                // println!("Task {} running", i);
                thread::sleep(Duration::from_millis(100)); // Simulate work
            },
        );
    }

    thread::sleep(Duration::from_millis(1000)); // Wait for tasks to start

    // Wait for the thread pool to complete tasks
    pool.shutdown();

    // Stop the monitoring thread
    running.store(false, Ordering::Release);
    monitor_handle.join().unwrap();

    // Final metrics after shutdown
    println!("\n--- Final Metrics ---");
    println!(
        "Queued tasks: {}",
        metrics.queued_tasks.load(Ordering::SeqCst)
    );
    println!(
        "Running tasks: {}",
        metrics.running_tasks.load(Ordering::SeqCst)
    );
    println!(
        "Completed tasks: {}",
        metrics.completed_tasks.load(Ordering::SeqCst)
    );
    println!(
        "Active threads: {}",
        metrics.active_threads.load(Ordering::SeqCst)
    );
}
