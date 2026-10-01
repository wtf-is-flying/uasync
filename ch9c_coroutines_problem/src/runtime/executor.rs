use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
    thread::{self, Thread},
};

use crate::future::{Future, PollState};

type Task = Box<dyn Future<Output = String>>;

thread_local! {
    static CURRENT_EXECUTOR: ExecutorCore = ExecutorCore::default();
}

/// Registers a new top-level future with thread-local executor.
pub fn spawn(future: impl Future<Output = String> + 'static) {
    CURRENT_EXECUTOR.with(|executor| {
        // Get the next available ID.
        let id = executor.next_id.get();
        // Store the future with this ID.
        executor.tasks.borrow_mut().insert(id, Box::new(future));
        // Add the ID to the `ready_queue` so that the task is polled at least once.
        executor
            .ready_queue
            .lock()
            .map(|mut q| q.push_back(id))
            .unwrap();
        // Set the next available ID.
        executor.next_id.set(id + 1);
    })
}

#[derive(Debug, Clone)]
pub struct Waker {
    /// Handle to the [`Executor`]'s thread.
    thread: Thread,
    /// Identifies which task this Waker is associated with.
    id: usize,
    /// Shared reference to a queue of ready task IDs.
    ready_queue: Arc<Mutex<VecDeque<usize>>>,
}

impl Waker {
    /// Puts this Waker's task ID in the ready queue and unpark the [`Executor`]'s thread.
    pub fn wake(&self) {
        self.ready_queue
            .lock()
            .map(|mut q| q.push_back(self.id))
            .unwrap();
        self.thread.unpark();
    }
}

#[derive(Default)]
struct ExecutorCore {
    /// Holds all top-level futures associated with this executor on this thread.
    /// Static variable aren't mutable, so we use a RefCell for interior mutability.
    tasks: RefCell<HashMap<usize, Task>>,
    /// Queue of ready task IDs that should be polled by the executor.
    /// The waker can signal that a task is ready from any thread.
    ready_queue: Arc<Mutex<VecDeque<usize>>>,
    /// Monotonic counter that gives out the next available top-level future ID.
    next_id: Cell<usize>,
}

#[derive(Default)]
pub struct Executor {}

impl Executor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn block_on(&mut self, future: impl Future<Output = String> + 'static) {
        // Optimization: assume ready ---------
        let waker = self.get_waker(usize::MAX); // Assume usize is never reached
        let mut future = future;
        match future.poll(&waker) {
            PollState::NotReady => (),
            PollState::Ready(_) => return,
        }
        // Optimization: end ------------------

        // Spawn the future onto ourselves
        spawn(future);

        loop {
            while let Some(id) = self.pop_ready() {
                let mut future = match self.remove_future(id) {
                    Some(f) => f,
                    // Guard against false wakeups
                    None => continue,
                };

                let waker = self.get_waker(id);
                match future.poll(&waker) {
                    PollState::NotReady => self.insert_task(id, future),
                    PollState::Ready(_) => continue,
                }
            }

            let task_count = self.task_count();
            let name = thread::current().name().unwrap_or_default().to_string();
            if task_count > 0 {
                println!("{name}: {task_count} pending tasks. Sleeping until notified.");
                thread::park();
            } else {
                println!("{name}: All tasks completed.");
                break;
            }
        }
    }

    /// Pops off an ID of a ready task.
    fn pop_ready(&self) -> Option<usize> {
        CURRENT_EXECUTOR.with(|e| e.ready_queue.lock().map(|mut q| q.pop_front()).unwrap())
    }

    /// Removes a top-level [`Task`]
    ///
    /// If the task returns [`PollState::NotReady`], it must be added back again.
    fn remove_future(&self, id: usize) -> Option<Task> {
        CURRENT_EXECUTOR.with(|e| e.tasks.borrow_mut().remove(&id))
    }

    /// Creates a new [`Waker`].
    fn get_waker(&self, id: usize) -> Waker {
        Waker {
            id,
            thread: thread::current(),
            ready_queue: CURRENT_EXECUTOR.with(|e| e.ready_queue.clone()),
        }
    }

    /// Inserts a [`Task`].
    fn insert_task(&self, id: usize, task: Task) {
        CURRENT_EXECUTOR.with(|e| e.tasks.borrow_mut().insert(id, task));
    }

    /// Returns the number of tasks in queue.
    fn task_count(&self) -> usize {
        CURRENT_EXECUTOR.with(|e| e.tasks.borrow().len())
    }
}
