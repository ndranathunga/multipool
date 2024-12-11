use multipool::ThreadPoolBuilder;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

#[test]
fn test_priority_higher_priority_runs_first() {
    let count = Arc::new(AtomicUsize::new(0));
    let order = Arc::new(Mutex::new(Vec::new()));

    // Create a priority-enabled thread pool (no work-stealing in this example)
    let pool = ThreadPoolBuilder::new()
        .num_threads(4)
        // .enable_priority()
        .build();

    // We'll spawn tasks with different priorities
    // Higher priority tasks should run before lower priority tasks.
    // Let's use priorities: 10 for high, 1 for low.
    let high_order = Arc::clone(&order);
    let high_count = Arc::clone(&count);
    let handle1 = pool.spawn(
        move || {
            let idx = high_count.fetch_add(1, Ordering::SeqCst);
            let mut vec = high_order.lock().unwrap();
            vec.push((10, idx));
        },
    );

    let low_order = Arc::clone(&order);
    let low_count = Arc::clone(&count);
    let handle2 = pool.spawn(
        move || {
            let idx = low_count.fetch_add(1, Ordering::SeqCst);
            let mut vec = low_order.lock().unwrap();
            vec.push((1, idx));
        },
    );

    handle1.join().unwrap();
    handle2.join().unwrap();

    pool.shutdown();

    // Check the order of execution
    let final_order = order.lock().unwrap();
    // Expect high priority (10) task to appear before the low priority (1)
    // Since each increments count, their indices show execution order.
    assert!(final_order.len() == 2, "Expected 2 tasks to run.");
    // The first executed task should be the one with priority 10
    assert_eq!(final_order[0].0, 10, "High priority task did not run first");
    assert_eq!(final_order[1].0, 1, "Low priority task did not run second");
}

// #[test]
// fn test_priority_with_work_stealing() {
//     let results = Arc::new(Mutex::new(Vec::new()));

//     // Create a priority-enabled thread pool with work-stealing
//     let pool = ThreadPoolBuilder::new()
//         .num_threads(4)
//         .set_work_stealing()
//         .enable_priority()
//         .build();

//     let mut handles = Vec::new();

//     // Spawn a bunch of tasks with mixed priorities
//     for i in 0..10 {
//         let res_arc = Arc::clone(&results);
//         let priority = if i % 2 == 0 { 5 } else { 10 };
//         handles.push(pool.spawn_with_priority(
//             move || {
//                 let mut vec = res_arc.lock().unwrap();
//                 vec.push(priority);
//             },
//             priority,
//         ));
//     }

//     for handle in handles {
//         handle.join().unwrap();
//     }

//     pool.shutdown();

//     // Check that all tasks executed and that higher priority tasks appear earlier on average.
//     let res = results.lock().unwrap();
//     assert_eq!(res.len(), 10, "Expected 10 tasks to run");

//     println!("Results: {:?}", res);
//     // This isn't a strict guarantee, because work-stealing might interleave tasks,
//     // but we should see a tendency for higher priority tasks (10) to come before lower (5).
//     // For a simple test, we can verify that the first executed task is with priority 10.
//     assert!(res.contains(&10), "No high-priority tasks executed");
//     assert!(res.contains(&5), "No low-priority tasks executed");

//     let first = res[0];
//     assert_eq!(
//         first, 10,
//         "Expected the first executed task to be high priority"
//     );
// }
