use crate::mutator::Mutator;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum DriverMode {
    Reproduce,
    Run,
}

/// Helper to manage fuzzer threads, thread state, and global state.
pub struct FuzzerDriver<T> {
    thread_count: usize,
    threads: RwLock<Vec<thread::JoinHandle<()>>>,
    num_iterations: AtomicUsize,
    num_failed_iterations: AtomicUsize,
    exit: AtomicBool,
    seed: u64,
    global_context: Option<Arc<RwLock<T>>>,
    mode: DriverMode,
    start_iteration: u64,
    end_iteration: u64,
    thread_last_execution_time: Vec<AtomicUsize>,
    thread_timeout: Duration,
}

impl<T: 'static + Send + Sync> Default for FuzzerDriver<T> {
    /// Instantiates new FuzzerDriver with 1 thread per logical CPU and uses
    /// the thread-local RNG to generate a seed
    fn default() -> Self {
        FuzzerDriver::<T>::new(1)
    }
}

impl<T: 'static + Send + Sync> FuzzerDriver<T> {
    /// Instantiates new FuzzerDriver with the specified number of threads and uses
    /// the thread-local RNG to generate a seed
    pub fn new(num_threads: usize) -> Self {
        let mut last_execution_times = Vec::with_capacity(num_threads);
        let since_the_epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

        for _i in 0..num_threads {
            last_execution_times.push(AtomicUsize::new(since_the_epoch.as_secs() as usize));
        }

        FuzzerDriver {
            thread_count: num_threads,
            threads: RwLock::new(Vec::with_capacity(num_threads)),
            num_iterations: Default::default(),
            num_failed_iterations: Default::default(),
            exit: Default::default(),
            seed: rand::random(),
            global_context: Default::default(),
            mode: DriverMode::Run,
            start_iteration: 0,
            end_iteration: 0,
            thread_last_execution_time: last_execution_times,
            thread_timeout: Duration::from_secs(10u64),
        }
    }

    pub fn thread_count(&self) -> usize {
        self.thread_count
    }

    /// Sets the driver mode to attempt to reproduce a crash. When [start_fuzzer] is called, the
    /// routine will configure each thread's RNG state to match what it was at start_iteration,
    /// the threads will begin to run, and end at end_iteration.
    pub fn set_to_reproduce_mode(&mut self, start_iteration: u64, end_iteration: u64) {
        self.mode = DriverMode::Reproduce;
        // TODO: start_iteration probably isn't necessary
        self.start_iteration = start_iteration;
        self.end_iteration = end_iteration;
        self.num_iterations
            .store(start_iteration as usize, Ordering::SeqCst);
    }

    /// Returns the total number of fuzzing iterations overall.
    pub fn num_iterations(&self) -> usize {
        self.num_iterations.load(Ordering::SeqCst)
    }

    /// Returns the number of iterations that returned an error result
    pub fn num_failed_iterations(&self) -> usize {
        self.num_failed_iterations.load(Ordering::SeqCst)
    }

    pub fn set_global_context(&mut self, context: Arc<RwLock<T>>) {
        self.global_context = Some(context);
    }

    pub fn global_context(&self) -> Option<Arc<RwLock<T>>> {
        if let Some(ref context) = self.global_context {
            Some(context.clone())
        } else {
            None
        }
    }

    /// Sets the root seed
    pub fn set_seed(&mut self, seed: u64) {
        self.seed = seed;
    }

    /// The root seed
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Signals that all threads should be exiting
    pub fn signal_exit(&self) {
        self.exit.store(true, Ordering::SeqCst);
    }

    /// Waits for all fuzzing threads to join
    pub fn join_threads(&self) {
        let mut threads = self.threads.write().unwrap();
        loop {
            let handle = threads.pop();
            match handle {
                Some(handle) => {
                    let thread_name = handle.thread().name().map_or(
                        String::from("UNNAMED_THREAD"),
                        std::borrow::ToOwned::to_owned,
                    );

                    handle
                        .join()
                        .unwrap_or_else(|_| println!("thread {} failed to join", thread_name));
                }
                None => break,
            }
        }
    }

    /// Returns a boolean indicating whether the calling thread should exit
    pub(crate) fn should_exit(&self) -> bool {
        if self.mode == DriverMode::Reproduce {
            return self.num_iterations() == self.end_iteration as usize;
        }

        self.exit.load(Ordering::SeqCst)
    }

    pub fn mode(&self) -> DriverMode {
        self.mode
    }

    pub(crate) fn set_thread_last_execution_time(&self, thread_index: usize) {
        let since_the_epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

        self.thread_last_execution_time[thread_index]
            .store(since_the_epoch.as_secs() as usize, Ordering::SeqCst);
    }

    /// Returns a bool indicating whether any threads have a last execution time > NOW() - timeout
    pub fn check_for_stalled_threads(&self) -> bool {
        let mut threads_have_stalled = false;
        let since_the_epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

        for i in 0..self.thread_count {
            let last_update = self.thread_last_execution_time[i].load(Ordering::SeqCst) as u64;
            let last_update = Duration::from_secs(last_update);

            // the thread could have updated its state while we're looping. check here to see
            // if that's the case
            if last_update > since_the_epoch {
                continue;
            }

            if since_the_epoch - last_update > self.thread_timeout {
                error!(
                    "{:?} has stalled!",
                    self.threads.read().unwrap()[i].thread().id()
                );
                threads_have_stalled = true;
            }
        }

        threads_have_stalled
    }

    /// Sets the max duration before a thread is flagged as stalled
    pub fn set_thread_timeout(&mut self, duration: Duration) {
        self.thread_timeout = duration
    }
}

/// Kicks off a fuzzing job using the driver and callback function.
/// 
/// The callback should look something like:
/// 
/// ```compile_fail
/// fn iteration_routine<R: Rng>(mutator: &mut Mutator<R>, thread_context: &mut FuzzerThreadContext, _global_context: Option<Arc<RwLock<GlobalContext>>>) -> Result<(), ()>
/// ```
pub fn start_fuzzer<F: 'static, C: 'static, T: 'static + Send + Sync>(
    driver: Arc<FuzzerDriver<T>>,
    callback: F,
) where
    F: Fn(&mut Mutator<StdRng>, &mut C, Option<Arc<RwLock<T>>>) -> Result<(), ()>
        + std::marker::Send
        + std::marker::Sync
        + Copy,
    C: Default,
{
    let mut root_rng = StdRng::seed_from_u64(driver.seed());

    let mut threads = driver.threads.write().unwrap();

    for i in 0..threads.capacity() {
        let thread_driver = driver.clone();
        let thread_name = format!("Fuzzer thread {}", i);

        let thread_seed: u64 = root_rng.gen();

        let join_handle = thread::Builder::new()
            .name(thread_name)
            .spawn(move || {
                // this is mostly to satisfy the requirement for Mutator::new. It'll be overwritten
                // on the first loop iteration
                let thread_rng = StdRng::seed_from_u64(0u64);
                let mut mutator = Mutator::new(thread_rng);
                let mut context = C::default();

                // loop until we get a signal that we should exit
                loop {
                    thread_driver.set_thread_last_execution_time(i);

                    // TODO: here be dragons? num_iterations is a usize and we're casting it to a u64. on 64-bit systems this
                    // isn't a problem since usize should be a u64, but it's worth noting that this could be a potential issue
                    let new_seed = thread_seed.wrapping_add(thread_driver.num_iterations() as u64);
                    mutator.rng = StdRng::seed_from_u64(new_seed);

                    if thread_driver.should_exit() {
                        log::info!("{} exiting", thread::current().name().unwrap());
                        return;
                    }

                    mutator.begin_new_iteration();

                    if let Err(_) = (callback)(&mut mutator, &mut context, thread_driver.global_context()) {
                        thread_driver
                            .num_failed_iterations
                            .fetch_add(1, Ordering::SeqCst);
                    }

                    thread_driver.num_iterations.fetch_add(1, Ordering::SeqCst);
                }
            })
            .unwrap_or_else(|_| panic!("could not create new thread"));

        threads.push(join_handle);
    }
}
