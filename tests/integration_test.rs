use multipool::ThreadPoolBuilder;

#[test]
fn test_basic_pool() {
    let pool = ThreadPoolBuilder::new().build();
    let handle = pool.spawn(|| 42);
    assert_eq!(handle.join().unwrap(), 42);
    pool.shutdown();
}

#[test]
fn test_work_stealing_pool() {
    let pool = ThreadPoolBuilder::new()
        .num_threads(4)
        .set_work_stealing()
        .build();
    let handle = pool.spawn(|| "hello");
    assert_eq!(handle.join().unwrap(), "hello");
    pool.shutdown();
}

#[test]
fn test_priority_pool() {
    let pool = ThreadPoolBuilder::new()
        .num_threads(4)
        .enable_priority()
        .build();
    let handle = pool.spawn_with_priority(|| "hello", 1);
    assert_eq!(handle.join().unwrap(), "hello");
    pool.shutdown();
}

#[test]
fn test_priority_pool_multiple() {
    let pool = ThreadPoolBuilder::new()
        .num_threads(4)
        .enable_priority()
        .build();
    let handle = pool.spawn_with_priority(|| "hello", 1);
    let handle2 = pool.spawn_with_priority(|| "world", 2);
    assert_eq!(handle.join().unwrap(), "hello");
    assert_eq!(handle2.join().unwrap(), "world");
    pool.shutdown();
}
