use multipool::ThreadPoolBuilder;

fn main() {
    let pool = ThreadPoolBuilder::new()
        .num_threads(4)
        .set_work_stealing()
        .build();
    let handle = pool.spawn(|| {
        println!("Hello from the work-stealing thread pool!");
        "done"
    });
    let res = handle.join().unwrap();
    println!("Task returned: {}", res);
    pool.shutdown();
}
