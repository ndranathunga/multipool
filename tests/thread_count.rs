use multipool::ThreadPool;

#[cfg(target_os = "windows")]
fn count_threads() -> usize {
    // use std::ptr::null_mut;
    use winapi::um::handleapi::INVALID_HANDLE_VALUE;
    use winapi::um::processthreadsapi::GetCurrentProcessId;
    use winapi::um::tlhelp32::{
        CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32,
    };

    unsafe {
        let current_process_id = GetCurrentProcessId();
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return 0;
        }

        let mut thread_entry = THREADENTRY32 {
            dwSize: std::mem::size_of::<THREADENTRY32>() as u32,
            cntUsage: 0,
            th32ThreadID: 0,
            th32OwnerProcessID: 0,
            tpBasePri: 0,
            tpDeltaPri: 0,
            dwFlags: 0,
        };

        if Thread32First(snapshot, &mut thread_entry) == 0 {
            return 0;
        }

        let mut thread_count = 0;

        loop {
            if thread_entry.th32OwnerProcessID == current_process_id {
                thread_count += 1;
            }

            if Thread32Next(snapshot, &mut thread_entry) == 0 {
                break;
            }
        }

        thread_count
    }
}

#[cfg(target_os = "linux")]
fn count_threads() -> usize {
    use procfs::process::Process;

    let process = Process::myself().expect("Failed to get process info");
    process.tasks().expect("Failed to get task list").count()
}

#[test]
fn test_threadpool_threads_lifecycle() {
    let initial_thread_count = count_threads();

    let num_threads = 4;
    let threadpool = ThreadPool::new(num_threads);

    // Wait for a short duration to allow threads to start
    std::thread::sleep(std::time::Duration::from_millis(100));
    let thread_count_after_start = count_threads();

    assert!(
        thread_count_after_start >= initial_thread_count + num_threads,
        "Expected at least {} threads to be started, found {}",
        num_threads,
        thread_count_after_start - initial_thread_count
    );

    threadpool.shutdown();

    // Wait for a short duration to allow threads to exit
    std::thread::sleep(std::time::Duration::from_millis(100));
    let final_thread_count = count_threads();

    assert_eq!(
        final_thread_count, initial_thread_count,
        "Expected all threads to terminate after shutdown"
    );
}
