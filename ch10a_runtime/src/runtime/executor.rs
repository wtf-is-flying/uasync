use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, VecDeque},
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Wake, Waker},
    thread::{self, Thread},
};

type Task = Pin<Box<dyn Future<Output = ()>>>;

thread_local! {
    static CURRENT_EXECUTOR: ExecutorCore = ExecutorCore::default();
}

/// Registers a new top-level future with thread-local executor.
pub fn spawn(future: impl Future<Output = ()> + 'static) {
    CURRENT_EXECUTOR.with(|executor| {
        // Get the next available ID.
        let id = executor.next_id.get();
        // Store the future with this ID.
        executor.tasks.borrow_mut().insert(id, Box::pin(future));
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
pub struct MyWaker {
    /// Handle to the [`Executor`]'s thread.
    thread: Thread,
    /// Identifies which task this Waker is associated with.
    id: usize,
    /// Shared reference to a queue of ready task IDs.
    ready_queue: Arc<Mutex<VecDeque<usize>>>,
}

impl Wake for MyWaker {
    /// Puts this Waker's task ID in the ready queue and unpark the [`Executor`]'s thread.
    fn wake(self: Arc<Self>) {
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

    pub fn block_on(&mut self, future: impl Future<Output = ()> + 'static) {
        // Spawn the future onto ourselves
        spawn(future);

        loop {
            while let Some(id) = self.pop_ready() {
                let mut future = match self.remove_future(id) {
                    Some(f) => f,
                    // Guard against false wakeups
                    None => continue,
                };

                let waker: Waker = self.get_waker(id).into();
                let mut cx = Context::from_waker(&waker);

                match future.as_mut().poll(&mut cx) {
                    Poll::Pending => self.insert_task(id, future),
                    Poll::Ready(_) => continue,
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
    fn get_waker(&self, id: usize) -> Arc<MyWaker> {
        Arc::new(MyWaker {
            id,
            thread: thread::current(),
            ready_queue: CURRENT_EXECUTOR.with(|e| e.ready_queue.clone()),
        })
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
