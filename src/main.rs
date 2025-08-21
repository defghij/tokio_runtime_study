use tracing::info;

/// Resources:
/// - [Async: What is blocking?](https://ryhl.io/blog/async-what-is-blocking/)
/// - [fn spawn_blocking](https://docs.rs/tokio/1.47.1/tokio/task/fn.spawn_blocking.html)
fn main() {
    setup_tracing(tracing::Level::TRACE);

    // Below are different ways to do concurrent "async" tasks. The first two spawn tasks in a
    // runtime that is only given a single thread. In the last three function calls the runtime is
    // given two threads. The final function call off-loads the blocking task to Rayon.
    singlethreaded::three_nonblocking_tasks();
    singlethreaded::one_blocking_two_nonblocking_tasks();
    multithreaded::tokio_only::one_blocking_two_nonblocking_tasks_a();
    multithreaded::tokio_only::one_blocking_two_nonblocking_tasks_b();
    multithreaded::with_rayon::one_blocking_two_nonblocking_tasks();
    multithreaded::with_thread::one_blocking_two_nonblocking_tasks();
    println!("Hello, world!");
}


mod singlethreaded {
    use super::*;

    #[allow(unused)]
    pub fn three_nonblocking_tasks() {
        info!("Starting: three_nonblocking_tasks");
        let rt = runtime::singlethreaded();

        let t1 = rt.spawn(async { sleep::nonblocking().await });
        let t2 = rt.spawn(async { sleep::nonblocking().await });
        let t3 = rt.spawn(async { sleep::nonblocking().await });

        rt.block_on(async { let _ = tokio::join!(t1,t2,t3); });
        info!("Ending: three_nonblocking_tasks");
    }

    /// Spawns three tasks into a single threaded run-time. The first task is async or non-blocking.
    /// The second task is sync or blocking. Finally, we spawn a third task which is non-blocking.
    /// We then `join!` all three tasks. Joining tells the run-time that we want to return the results
    /// of both tasks. The join is contained in a `block_on` which will block the calling thread 
    /// until the join (and tasks) finish. 
    ///
    /// When this function runs you'll see something like the following output
    /// ```text
    /// 2025-08-21T02:03:35.178592Z  INFO ThreadId(01) async_test::sleep:29: Starting: nonblocking
    /// 2025-08-21T02:03:35.178650Z  INFO ThreadId(01) async_test::sleep:23: Starting: blocking
    /// 2025-08-21T02:03:45.179087Z  INFO ThreadId(01) async_test::sleep:25: Ending: blocking
    /// 2025-08-21T02:03:45.179355Z  INFO ThreadId(01) async_test::sleep:29: Starting: nonblocking
    /// 2025-08-21T02:03:45.179683Z  INFO ThreadId(01) async_test::sleep:31: Ending: nonblocking
    /// 2025-08-21T02:03:46.181643Z  INFO ThreadId(01) async_test::sleep:31: Ending: nonblocking
    /// ```
    /// Notice that we see that the nonblocking and blocking calls start-- first and second line,
    /// respectively. The third task, which is non-blocking, does not start until the second,
    /// blocking, task returns and ends. Because we are blocking the runtime, the first
    /// nonblocking task is also not able to advance.
    ///
    /// Stated more succinctly, the blocking task prevents other other tasks from making progress
    /// because it does not just block the other tasks it blocks the run-time.
    ///
    ///
    /// Note: we need to call `block_on` otherwise after we call join the function will exit and
    /// we'll carry on to other parts of the program. So the `block_on` here is kind of saying: "no
    /// really, wait until you finish both tasks. For real."
    #[allow(unused)]
    pub fn one_blocking_two_nonblocking_tasks() {
        info!("Starting: one_blocking_two_nonblocking_tasks");
        let rt = runtime::singlethreaded();

        let t1 = rt.spawn(async { sleep::nonblocking().await });
        let t2 = rt.spawn(async { sleep::blocking() });
        let t3 = rt.spawn(async { sleep::nonblocking().await });

        rt.block_on(async { let _ = tokio::join!(t1,t2,t3); });
        info!("Ending: one_blocking_two_nonblocking_tasks");
    }
}

mod multithreaded {
    use super::*;

    pub mod tokio_only {
        use super::*;

        /// This function spawns three tasks. The first and last are non-blocking. The second is
        /// blocking.
        ///
        /// In the example output below we notice a few things. First, we spawn the two non-blocking
        /// tasks immediately They also return very shortly after the blocking task is spawned. Looking
        /// at the time stamps we can see that they return after roughly one second. This is not the
        /// case for the blocking. Instead, it returns last after about 10 seconds. This is what we
        /// would expect. 
        ///
        /// Another thing to notice is the thread IDs. Specifically, _both_ of the nonblocking tasks
        /// start on the same thread. However, only one task ends on the same thread. Instead, one of
        /// them is "resumed" on a new thread. This is not the case with the blocking task. This should
        /// be the case because in this function we've used `spawn_blocking` which makes use of a
        /// blocking thread pool in the multithreaded runtime.
        ///
        /// ```text
        /// 2025-08-21T02:18:28.959054Z  INFO ThreadId(02) async_test::sleep:29: Starting: nonblocking
        /// 2025-08-21T02:18:28.959127Z  INFO ThreadId(02) async_test::sleep:29: Starting: nonblocking
        /// 2025-08-21T02:18:28.959435Z  INFO ThreadId(04) async_test::sleep:23: Starting: blocking
        /// 2025-08-21T02:18:29.959817Z  INFO ThreadId(02) async_test::sleep:31: Ending: nonblocking
        /// 2025-08-21T02:18:29.959864Z  INFO ThreadId(03) async_test::sleep:31: Ending: nonblocking
        /// 2025-08-21T02:18:38.959965Z  INFO ThreadId(04) async_test::sleep:25: Ending: blocking
        /// ```
        #[allow(unused)]
        pub fn one_blocking_two_nonblocking_tasks_a() {
            info!("Starting: one_blocking_two_nonblocking_tasks_a");
            let rt = runtime::multithreaded();

            let t1 = rt.spawn(async { sleep::nonblocking().await });
            let t2 = rt.spawn_blocking(|| { sleep::blocking() });
            let t3 = rt.spawn(async { sleep::nonblocking().await });

            rt.block_on(async { let _ = tokio::join!(t1,t2,t3); });
            info!("Ending: one_blocking_two_nonblocking_tasks_a");
        }

        /// This is the same as the above function (`one_blocking_two_nonblocking_tasks_a`) except that
        /// here we spawn `sleep::blocking` using `spawn` which will not make use of the multithreaded
        /// pool. The other tasks will still make progress because the runtime on the other thread can
        /// still progress other other two (non-blocking) tasks.
        #[allow(unused)]
        pub fn one_blocking_two_nonblocking_tasks_b() {
            info!("Starting: one_blocking_two_nonblocking_tasks_b");
            let rt = runtime::multithreaded();

            let t1 = rt.spawn(async { sleep::nonblocking().await });
            let t2 = rt.spawn(async { sleep::blocking() });
            let t3 = rt.spawn(async { sleep::nonblocking().await });

            rt.block_on(async { let _ = tokio::join!(t1,t2,t3); });
            info!("Ending: one_blocking_two_nonblocking_tasks_b");
        }
    }

    pub mod with_rayon {
        use super::*;

        /// In this function we modify where we run the blocking task. Rather than using the tokio
        /// runtime and its threadpools, we make use of Rayon. We spawn a thread within the rayon
        /// threadpool to execute the blocking task. This does not consume any of tokio runtime
        /// threads. Instead, the `spawn` for second (blocking) task is "blocked" on the `recv`
        /// async function from `tokio::sync::oneshot`. 
        ///
        /// This has kind of turned this from a sync blocking task into an async non-blocking task.
        /// But this is only because we've "moved" that task to another thread (in rayon).
        ///
        /// If we look at the output below, it looks very similar to the other functions outputs in
        /// the parent module. Thus demonstrating that this is a viable strategy for managing async
        /// tasks. 
        ///
        /// ```text
        /// 2025-08-21T02:51:17.455132Z  INFO ThreadId(03) async_test::sleep:29: Starting: nonblocking
        /// 2025-08-21T02:51:17.455678Z  INFO ThreadId(03) async_test::sleep:29: Starting: nonblocking
        /// 2025-08-21T02:51:17.455948Z  INFO ThreadId(04) async_test::sleep:23: Starting: blocking
        /// 2025-08-21T02:51:18.456531Z  INFO ThreadId(02) async_test::sleep:31: Ending: nonblocking
        /// 2025-08-21T02:51:18.456506Z  INFO ThreadId(03) async_test::sleep:31: Ending: nonblocking
        /// 2025-08-21T02:51:27.456461Z  INFO ThreadId(04) async_test::sleep:25: Ending: blocking
        /// ```
        #[allow(unused)]
        pub fn one_blocking_two_nonblocking_tasks() {
            info!("Starting: one_blocking_two_nonblocking_tasks_c");
            let rt = runtime::multithreaded();

            let (send, recv) = tokio::sync::oneshot::channel();

            let t1 = rt.spawn(async { sleep::nonblocking().await });
            let t2 = rt.spawn(async {
                let _ = recv.await // this waits for the rayon thread to send something.
                    .expect("sleep does not panic and send should not.");
            });
            let t3 = rt.spawn(async { sleep::nonblocking().await });

            // Spawn a task on rayon
            // Note: this is _not_ async! It is also non-blocking (in _this_ thread)!
            rayon::spawn(move || {
                sleep::blocking();
                let _ = send.send(true);
            });

            rt.block_on(async { let _ = tokio::join!(t1,t2,t3); });
            info!("Ending: one_blocking_two_nonblocking_tasks_c");
        }
    }

    pub mod with_thread {
        use super::*;

        /// In this function we modify where we run the blocking task. Rather than using the tokio
        /// runtime and its threadpools, we make use of Rayon. We spawn a thread within the rayon
        /// threadpool to execute the blocking task. This does not consume any of tokio runtime
        /// threads. Instead, the `spawn` for second (blocking) task is "blocked" on the `recv`
        /// async function from `tokio::sync::oneshot`. 
        ///
        /// This has kind of turned this from a sync blocking task into an async non-blocking task.
        /// But this is only because we've "moved" that task to another thread (in rayon).
        ///
        /// If we look at the output below, it looks very similar to the other functions outputs in
        /// the parent module. Thus demonstrating that this is a viable strategy for managing async
        /// tasks. 
        ///
        /// ```text
        /// 2025-08-21T02:51:17.455132Z  INFO ThreadId(03) async_test::sleep:29: Starting: nonblocking
        /// 2025-08-21T02:51:17.455678Z  INFO ThreadId(03) async_test::sleep:29: Starting: nonblocking
        /// 2025-08-21T02:51:17.455948Z  INFO ThreadId(04) async_test::sleep:23: Starting: blocking
        /// 2025-08-21T02:51:18.456531Z  INFO ThreadId(02) async_test::sleep:31: Ending: nonblocking
        /// 2025-08-21T02:51:18.456506Z  INFO ThreadId(03) async_test::sleep:31: Ending: nonblocking
        /// 2025-08-21T02:51:27.456461Z  INFO ThreadId(04) async_test::sleep:25: Ending: blocking
        /// ```
        #[allow(unused)]
        pub fn one_blocking_two_nonblocking_tasks() {
            info!("Starting: one_blocking_two_nonblocking_tasks_c");
            let rt = runtime::multithreaded();

            let (send, recv) = tokio::sync::oneshot::channel();

            let t1 = rt.spawn(async { sleep::nonblocking().await });
            let t2 = rt.spawn(async {
                let _ = recv.await // this waits for the rayon thread to send something.
                    .expect("sleep does not panic and send should not.");
            });
            let t3 = rt.spawn(async { sleep::nonblocking().await });

            // Spawn a task on rayon
            // Note: this is _not_ async! It is also non-blocking (in _this_ thread)!
            std::thread::spawn(move || {
                sleep::blocking();
                let _ = send.send(true);
            });

            rt.block_on(async { let _ = tokio::join!(t1,t2,t3); });
            info!("Ending: one_blocking_two_nonblocking_tasks_c");
        }
    }
}

mod sleep {
    use super::*;
    use std::thread::sleep as restful_sleep;
    use std::time::Duration;
    use tokio::time::sleep as unrestful_sleep;

    pub fn blocking() {
        info!("Starting: blocking");
        restful_sleep(Duration::new(10,0));
        info!("Ending: blocking");
    }

    pub async fn nonblocking() { 
        info!("Starting: nonblocking");
        unrestful_sleep(Duration::new(1,0)).await;
        info!("Ending: nonblocking");
    }
}

mod runtime {
    use super::*;

    pub fn singlethreaded() -> tokio::runtime::Runtime {
        info!("Starting: create_runtime");
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("multi-threaded-runtime")
            .thread_stack_size(3 * 1024 * 1024)
            .enable_all()
            .build()
            .unwrap();
        info!("Ending: create_runtime");
        runtime
    }

    pub fn multithreaded() -> tokio::runtime::Runtime {
        info!("Starting: create_runtime");
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("multi-threaded-runtime")
            .thread_stack_size(3 * 1024 * 1024)
            .enable_all()
            .build()
            .unwrap();
        info!("Ending: create_runtime");
        runtime
    }


}

pub fn setup_tracing(level: tracing::Level) {
    // construct a subscriber that prints formatted traces to stdout
    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_line_number(true)
        .with_thread_ids(true)
        .with_max_level(level)
        .finish();

    // use that subscriber to process traces emitted after this point
    tracing::subscriber::set_global_default(subscriber).expect("Subscriber setup should succeed");
}
