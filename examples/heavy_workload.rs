use multipool::ThreadPoolBuilder;
use std::thread;
use std::time::Instant;

fn cpu_task(n: u64) -> u64 {
    (0..n).map(|x| x * x).sum()
}

fn main() {
    let num_threads = 12;
    let num_tasks = 1_000_000;

    // Measure time for thread pool
    let pool_start = Instant::now();

    let pool = ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .work_stealing(true)
        .build();
    let mut pool_handles = Vec::with_capacity(num_tasks);

    for _ in 0..num_tasks {
        pool_handles.push(pool.spawn(move || {
            let _ = cpu_task(10_000);
        }));
    }

    for handle in pool_handles {
        let _ = handle.join();
    }

    pool.shutdown();

    let pool_duration = pool_start.elapsed();
    println!(
        "Time taken with thread pool (work-stealing): {:.2?} seconds",
        pool_duration
    );

    // Measure time for thread pool
    let pool_start = Instant::now();

    let pool = ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .work_stealing(false)
        .build();
    let mut pool_handles = Vec::with_capacity(num_tasks);

    for _ in 0..num_tasks {
        pool_handles.push(pool.spawn(move || {
            let _ = cpu_task(10_000);
        }));
    }

    for handle in pool_handles {
        let _ = handle.join();
    }

    pool.shutdown();

    let pool_duration = pool_start.elapsed();
    println!(
        "Time taken with thread pool (task-queue): {:.2?} seconds",
        pool_duration
    );

    // Measure time for traditional threads
    let traditional_start = Instant::now();

    let mut thread_handles = Vec::with_capacity(num_tasks);
    for _ in 0..num_tasks {
        thread_handles.push(thread::spawn(move || {
            let _ = cpu_task(10_000);
        }));
    }

    for handle in thread_handles {
        let _ = handle.join();
    }

    let traditional_duration = traditional_start.elapsed();
    println!(
        "Time taken with traditional threads: {:.2?} seconds",
        traditional_duration
    );
}