use std::sync::OnceLock;

use mio::{Events, Poll, Registry};

use crate::future::{Future, PollState};

static REGISTRY: OnceLock<Registry> = OnceLock::new();

pub fn registry() -> &'static Registry {
    REGISTRY
        .get()
        .expect("registry() must be called inside a runtime context")
}

pub struct Runtime {
    poll: Poll,
}

impl Runtime {
    pub fn new() -> Self {
        let poll = Poll::new().unwrap();
        let registry = poll.registry().try_clone().unwrap();
        REGISTRY.set(registry).unwrap();
        Self { poll }
    }

    pub fn block_on(&mut self, mut future: impl Future<Output = String>) {
        loop {
            match future.poll() {
                PollState::Ready(_) => break,
                PollState::NotReady => {
                    println!("Scheduling other tasks");
                    let mut events = Events::with_capacity(100);
                    self.poll.poll(&mut events, None).unwrap();
                }
            }
        }
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}
