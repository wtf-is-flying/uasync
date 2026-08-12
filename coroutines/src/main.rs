use std::{
    task::{Context, Waker},
    thread,
    time::Duration,
};

use coroutines::{
    future::{Future, PollState},
    http::Http,
};

struct Coroutine {
    state: State,
}

enum State {
    Start,
    Wait1(Box<dyn Future<Output = String>>),
    Wait2(Box<dyn Future<Output = String>>),
    Resolved,
}

impl Coroutine {
    fn new() -> Self {
        Coroutine {
            state: State::Start,
        }
    }
}

impl Future for Coroutine {
    type Output = ();

    fn poll(&mut self) -> PollState<Self::Output> {
        loop {
            match &mut self.state {
                State::Start => {
                    println!("Starting coroutine");
                    let fut = Box::new(Http::get("/600/HelloWorld1"));
                    self.state = State::Wait1(fut);
                }
                State::Wait1(fut) => match fut.poll() {
                    PollState::Ready(s) => {
                        println!("{s}");
                        let fut = Box::new(Http::get("/400/HelloWorld2"));
                        self.state = State::Wait2(fut);
                    }
                    PollState::NotReady => {
                        break PollState::NotReady;
                    }
                },
                State::Wait2(fut) => match fut.poll() {
                    PollState::Ready(s) => {
                        println!("{s}");
                        self.state = State::Resolved;
                        break PollState::Ready(());
                    }
                    PollState::NotReady => {
                        break PollState::NotReady;
                    }
                },
                State::Resolved => panic!("polled a resolved future"),
            }
        }
    }
}

fn async_main() -> impl Future<Output = ()> {
    Coroutine::new()
}

async fn std_async_main() {
    println!("Starting coroutine");
    let txt = Http::std_get("/600/HelloWorld1").await;
    println!("{txt}");
    let txt = Http::std_get("/400/HelloWorld2").await;
    println!("{txt}");
}

fn main() {
    let mut future = async_main();
    while !future.poll().is_ready() {
        println!("Future not ready; you schedule other tasks in the meantime");
        thread::sleep(Duration::from_millis(100));
    }

    let future = std_async_main();
    let mut pinned = Box::pin(future);
    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);

    while pinned.as_mut().poll(&mut cx).is_pending() {
        println!("Future not ready; you schedule other tasks in the meantime");
        thread::sleep(Duration::from_millis(100));
    }
}
