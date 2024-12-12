use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use multipool::{pool::ThreadPoolBuilder, run_traditional};
use rand::Rng;

/// A CPU-bound task: compute the sum of a range.
fn cpu_task() -> u64 {
    (0..10).sum()
}

fn prepare_tasks(n: usize) -> Vec<Box<dyn FnOnce() + Send>> {
    (0..n)
        .map(|_| {
            Box::new(|| {
                let _ = cpu_task();
            }) as Box<dyn FnOnce() + Send>
        })
        .collect()
}

fn benchmark_global_queue(c: &mut Criterion) {
    let mut group = c.benchmark_group("global_queue");
    group.sample_size(10);

    // Benchmark a fixed number of tasks
    let num_threads = 4;
    let num_tasks = 10_000;

    group.bench_function("global_queue_10k_tasks", |b| {
        b.iter_batched(
            || {
                // Prepare a fresh pool and tasks each iteration

                let pool = ThreadPoolBuilder::new().num_threads(num_threads).build();
                let tasks = prepare_tasks(num_tasks);
                (pool, tasks)
            },
            |(pool, tasks)| {
                let handles: Vec<_> = tasks
                    .into_iter()
                    .map(|task| {
                        pool.spawn(move || {
                            task();
                        })
                    })
                    .collect();

                for h in handles {
                    let _ = h.join();
                }
                pool.shutdown();
            },
            BatchSize::LargeInput,
        )
    });

    group.bench_function("global_queue_with_priority_10k_tasks", |b| {
        b.iter_batched(
            || {
                // Prepare a fresh pool and tasks each iteration
                let pool = ThreadPoolBuilder::new()
                    .num_threads(num_threads)
                    .enable_priority()
                    .build();
                let tasks = prepare_tasks(num_tasks);
                (pool, tasks)
            },
            |(pool, tasks)| {
                let handles: Vec<_> = tasks
                    .into_iter()
                    .map(|task| {
                        pool.spawn_with_priority(
                            move || {
                                task();
                            },
                            rand::thread_rng().gen_range(0..=10),
                        )
                    })
                    .collect();

                for h in handles {
                    let _ = h.join();
                }
                pool.shutdown();
            },
            BatchSize::LargeInput,
        )
    });

    group.finish();
}

fn benchmark_work_stealing(c: &mut Criterion) {
    let mut group = c.benchmark_group("work_stealing");
    group.sample_size(10);

    let num_threads = 4;
    let num_tasks = 10_000;

    group.bench_function("work_stealing_10k_tasks", |b| {
        b.iter_batched(
            || {
                let pool = ThreadPoolBuilder::new()
                    .num_threads(num_threads)
                    .set_work_stealing()
                    .build();
                let tasks = prepare_tasks(num_tasks);
                (pool, tasks)
            },
            |(pool, tasks)| {
                let handles: Vec<_> = tasks
                    .into_iter()
                    .map(|task| {
                        pool.spawn(move || {
                            task();
                        })
                    })
                    .collect();

                for h in handles {
                    let _ = h.join();
                }
                pool.shutdown();
            },
            BatchSize::LargeInput,
        )
    });

    group.bench_function("work_stealing_with_priority_10k_tasks", |b| {
        b.iter_batched(
            || {
                let pool = ThreadPoolBuilder::new()
                    .num_threads(num_threads)
                    .set_work_stealing()
                    .enable_priority()
                    .build();
                let tasks = prepare_tasks(num_tasks);
                (pool, tasks)
            },
            |(pool, tasks)| {
                let handles: Vec<_> = tasks
                    .into_iter()
                    .map(|task| {
                        pool.spawn_with_priority(
                            move || {
                                task();
                            },
                            rand::thread_rng().gen_range(0..=10),
                        )
                    })
                    .collect();

                for h in handles {
                    let _ = h.join();
                }
                pool.shutdown();
            },
            BatchSize::LargeInput,
        )
    });

    group.finish();
}

fn benchmark_traditional(c: &mut Criterion) {
    let mut group = c.benchmark_group("traditional");
    group.sample_size(10);

    let num_tasks = 10_000;

    group.bench_function("traditional_10k_tasks", |b| {
        b.iter_batched(
            || prepare_tasks(num_tasks),
            |tasks| {
                run_traditional(tasks);
            },
            BatchSize::LargeInput,
        )
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_global_queue,
    benchmark_work_stealing,
    benchmark_traditional
);
criterion_main!(benches);
