use std::{fmt::Write, marker::PhantomPinned, pin::Pin};

use ch9e_coroutines_pin::{
    future::{Future, PollState},
    http::Http,
    runtime::{self, Waker},
};

fn main() {
    let mut executor = runtime::init();
    executor.block_on(async_main());
}

// coroutine fn async_main() {
//     let buffer = String::from("\nBUFFER:\n----\n");
//     let writer = &mut buffer;
//
//     println!("Program starting");
//
//     let txt = Http::get("/600/HelloAsyncAwait").wait;
//     writeln!(writer, "{txt}").unwrap();
//
//     let txt = Http::get("/400/HelloAsyncAwait").wait;
//     writeln!(writer, "{txt}").unwrap();
//
//     println!("{}", buffer);
// }

fn async_main() -> impl Future<Output = String> {
    Coroutine0::new()
}

enum State0 {
    Start,
    Wait1(Pin<Box<dyn Future<Output = String>>>),
    Wait2(Pin<Box<dyn Future<Output = String>>>),
    Resolved,
}

/// This is called "stack",
/// but it can (and likely will) live on the heap!
#[derive(Default)]
struct Stack0 {
    buffer: Option<String>,
    /// A reference to [`Self::buffer`].
    writer: Option<*mut String>,
}

struct Coroutine0 {
    stack: Stack0,
    state: State0,
    _pin: PhantomPinned,
}

impl Coroutine0 {
    fn new() -> Self {
        Self {
            stack: Stack0::default(),
            state: State0::Start,
            _pin: PhantomPinned,
        }
    }
}

impl Future for Coroutine0 {
    type Output = String;

    fn poll(self: Pin<&mut Self>, waker: &Waker) -> PollState<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        loop {
            match this.state {
                State0::Start => {
                    // ------- Initialize stack --------
                    this.stack.buffer = Some(String::from("\nBUFFER:\n----\n"));
                    this.stack.writer = Some(this.stack.buffer.as_mut().unwrap());

                    // ---- Code you actually wrote ----
                    println!("Program starting");

                    // ---------------------------------

                    let fut1 = Box::pin(Http::get("/600/HelloAsyncAwait"));
                    this.state = State0::Wait1(fut1);

                    // ---------- Save stack -----------
                }

                State0::Wait1(ref mut f1) => {
                    match f1.as_mut().poll(waker) {
                        PollState::Ready(txt) => {
                            // --------- Restore stack ---------
                            let writer = unsafe { &mut *this.stack.writer.take().unwrap() };

                            // ---- Code you actually wrote ----
                            writeln!(writer, "{txt}").unwrap();

                            // ---------------------------------
                            let fut2 = Box::pin(Http::get("/400/HelloAsyncAwait"));
                            this.state = State0::Wait2(fut2);

                            // ---------- Save stack -----------
                            this.stack.writer = Some(writer);
                        }
                        PollState::NotReady => break PollState::NotReady,
                    }
                }

                State0::Wait2(ref mut f2) => {
                    match f2.as_mut().poll(waker) {
                        PollState::Ready(txt) => {
                            // --------- Restore stack ---------
                            // Taking a ref because taking ownership would invalidate writer!
                            let buffer = this.stack.buffer.as_ref().unwrap();
                            let writer = unsafe { &mut *this.stack.writer.take().unwrap() };

                            // ---- Code you actually wrote ----
                            writeln!(writer, "{txt}").unwrap();
                            println!("{buffer}");

                            // --------- Free resources --------
                            this.state = State0::Resolved;
                            let _ = this.stack.buffer.take().unwrap();

                            // ---------------------------------

                            break PollState::Ready(String::new());
                        }
                        PollState::NotReady => break PollState::NotReady,
                    }
                }

                State0::Resolved => panic!("Polled a resolved future"),
            }
        }
    }
}
