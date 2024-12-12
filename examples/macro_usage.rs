use multipool::{create_thread_pool, spawn_task};

fn main() {
    let pool = create_thread_pool!(num_threads: 4); // GlobalQueueMode
    let handle = spawn_task!(pool, || println!("Task without priority"));
    handle.join().unwrap();
    pool.shutdown();

    let pool = create_thread_pool!(num_threads: 4, work_stealing: true, priority: true); // PriorityWorkStealingMode
    let handle = spawn_task!(pool, || println!("Task with priority"), priority: 1);
    handle.join().unwrap();
    pool.shutdown();
}
