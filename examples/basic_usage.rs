fn main() {
    let pool = multipool::ThreadPool::new(4);
    let handle = pool.spawn(|| {
        println!("Hello from the basic thread pool!");
        10
    });
    let res = handle.join().unwrap();
    println!("Result from task: {}", res);
    pool.shutdown();
}
