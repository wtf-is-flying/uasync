use std::arch::{asm, naked_asm};

const DEFAULT_STACK_SIZE: usize = 1024 * 1024 * 2;
const MAX_THREADS: usize = 4;

/// A raw pointer to the global runtime.
static mut RUNTIME: usize = 0;

pub struct Runtime {
    threads: Vec<Thread>,

    /// Index of the currently scheduled thread.
    current: usize,
}

impl Runtime {
    pub fn new() -> Self {
        let base_thread = Thread {
            stack: vec![0u8; DEFAULT_STACK_SIZE],
            ctx: ThreadContext::default(),
            state: State::Running,
        };

        let mut threads = vec![base_thread];
        let mut available_threads = (1..MAX_THREADS).map(|_| Thread::new()).collect();
        threads.append(&mut available_threads);

        Runtime {
            threads,
            current: 0,
        }
    }

    /// Sets the global runtime to this runtime.
    pub fn init(&self) {
        unsafe {
            let raw_ptr: *const Runtime = self;
            RUNTIME = raw_ptr as usize;
        }
    }

    pub fn run(&mut self) -> ! {
        while self.t_yield() {}
        std::process::exit(0);
    }

    pub fn spawn(&mut self, f: fn()) {
        let available = self
            .threads
            .iter_mut()
            .find(|t| t.state == State::Available)
            .expect("no available thread");

        let stack_size = available.stack.len();

        unsafe {
            let stack_ptr = available.stack.as_mut_ptr().offset(stack_size as isize);
            let stack_ptr = (stack_ptr as usize & !15) as *mut u8;

            std::ptr::write(stack_ptr.offset(-16) as *mut u64, guard as u64);
            std::ptr::write(stack_ptr.offset(-24) as *mut u64, skip as u64);
            std::ptr::write(stack_ptr.offset(-32) as *mut u64, f as u64);
            available.ctx.rsp = stack_ptr.offset(-32) as u64;
        }
        available.state = State::Ready;
    }

    #[inline(never)]
    fn t_yield(&mut self) -> bool {
        let mut pos = self.current;
        while self.threads[pos].state != State::Ready {
            pos += 1;
            if pos == self.threads.len() {
                pos = 0;
            }
            if pos == self.current {
                return false;
            }
        }

        if self.threads[self.current].state != State::Available {
            self.threads[self.current].state = State::Ready;
        }

        self.threads[pos].state = State::Running;
        let old_pos = self.current;
        self.current = pos;

        unsafe {
            let old: *mut ThreadContext = &mut self.threads[old_pos].ctx;
            let new: *const ThreadContext = &self.threads[pos].ctx;

            asm!(
                "call switch", in("rdi") old, in("rsi") new, clobber_abi("C")
            );
        }

        // This line just prevents the compiler from optimizing the code away.
        !self.threads.is_empty()
    }

    /// Marks the current thread as [`State::Available`].
    ///
    /// This return function is called  when a thread is finished.
    fn t_return(&mut self) {
        if self.current != 0 {
            self.threads[self.current].state = State::Available;
            self.t_yield();
        }
    }
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
unsafe extern "C" fn switch() {
    naked_asm!(
        "mov [rdi + 0x00], rsp",
        "mov [rdi + 0x08], r15",
        "mov [rdi + 0x10], r14",
        "mov [rdi + 0x18], r13",
        "mov [rdi + 0x20], r12",
        "mov [rdi + 0x28], rbx",
        "mov [rdi + 0x30], rbp",
        "mov rsp, [rsi + 0x00]",
        "mov r15, [rsi + 0x08]",
        "mov r14, [rsi + 0x10]",
        "mov r13, [rsi + 0x18]",
        "mov r12, [rsi + 0x20]",
        "mov rbx, [rsi + 0x28]",
        "mov rbp, [rsi + 0x30]",
        "ret"
    )
}

/// Called when the function passed in to [`Runtime::spawn`] as returned.
fn guard() {
    unsafe {
        let rt_ptr = RUNTIME as *mut Runtime;
        (*rt_ptr).t_return();
    }
}

#[unsafe(naked)]
unsafe extern "C" fn skip() {
    // `ret` just pops off the next value from the stack,
    // and jump to whatever instructions that address points to.
    naked_asm!("ret")
}

// Yields to the runtime.
//
// This function can be called from an arbitrary place in code.
pub fn yield_thread() {
    unsafe {
        let rt_ptr = RUNTIME as *mut Runtime;
        (*rt_ptr).t_yield();
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(PartialEq, Eq, Debug)]
enum State {
    /// Thread is available for a task.
    Available,
    /// Thread is running.
    Running,
    /// Thread is ready to move forward and resume execution.
    Ready,
}

struct Thread {
    /// Thread stack.
    ///
    /// Once a stack is allocated it must not move!
    /// No push() on the vector or any other methods that might trigger a reallocation.
    /// If the stack is reallocated, any pointers that we hold to it are invalidated.
    stack: Vec<u8>,
    ctx: ThreadContext,
    state: State,
}

impl Thread {
    fn new() -> Self {
        Thread {
            stack: vec![0u8; DEFAULT_STACK_SIZE],
            ctx: ThreadContext::default(),
            state: State::Available,
        }
    }
}

#[derive(Debug, Default)]
#[repr(C)]
struct ThreadContext {
    // These registers are marked as _callee saved_ by the System V ABI spec.
    // The _callee_ needs to restore them before the _caller_ is resumed.
    rsp: u64,
    r15: u64,
    r14: u64,
    r13: u64,
    r12: u64,
    rbx: u64,
    rbp: u64,
}

fn main() {
    let mut runtime = Runtime::new();
    runtime.init();

    runtime.spawn(|| {
        println!("THREAD 1 STARTING");
        let id = 1;
        for i in 0..10 {
            println!("thread: {id} counter: {i}");
            yield_thread();
        }
        println!("THREAD 1 FINISHED");
    });
    runtime.spawn(|| {
        println!("THREAD 2 STARTING");
        let id = 2;
        for i in 0..15 {
            println!("thread: {id} counter: {i}");
            yield_thread();
        }
        println!("THREAD 2 FINISHED");
    });
    runtime.run()
}
