fn main() {
    let pool = multipool::ThreadPoolBuilder::new().num_threads(4).build();
    let handle = pool.spawn(|| {
        println!("Hello from the basic thread pool!");
        10
    });
    let res = handle.join().unwrap();
    println!("Result from task: {}", res);
    pool.shutdown();
}
