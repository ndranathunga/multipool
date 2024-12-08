use multipool::{ThreadPool, ThreadPoolBuilder};

#[test]
fn test_basic_pool() {
    let pool = ThreadPool::new(2);
    let handle = pool.spawn(|| 42);
    assert_eq!(handle.join().unwrap(), 42);
    pool.shutdown();
}

#[test]
fn test_work_stealing_pool() {
    let pool = ThreadPoolBuilder::new()
        .num_threads(4)
        .work_stealing(true)
        .build();
    let handle = pool.spawn(|| "hello");
    assert_eq!(handle.join().unwrap(), "hello");
    pool.shutdown();
}
