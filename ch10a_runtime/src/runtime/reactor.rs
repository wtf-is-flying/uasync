use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Waker},
    thread,
};

use mio::{Events, Interest, Poll, Registry, Token, net::TcpStream};

type Wakers = Arc<Mutex<HashMap<usize, Waker>>>;

// Single global [`Reactor`] instance.
static REACTOR: OnceLock<Reactor> = OnceLock::new();

pub fn reactor() -> &'static Reactor {
    REACTOR
        .get()
        .expect("reactor() must be called inside a runtime context")
}

pub fn start() {
    let wakers = Arc::new(Mutex::new(HashMap::new()));
    let poll = Poll::new().unwrap();
    let registry = poll.registry().try_clone().unwrap();
    let next_id = AtomicUsize::new(1);
    let reactor = Reactor {
        wakers: Arc::clone(&wakers),
        registry,
        next_id,
    };

    REACTOR.set(reactor).expect("Reactor is already running");
    thread::spawn(|| event_loop(poll, wakers));
}

#[derive(Debug)]
pub struct Reactor {
    wakers: Wakers,
    registry: Registry,
    next_id: AtomicUsize,
}

impl Reactor {
    pub fn register(&self, stream: &mut TcpStream, interest: Interest, id: usize) {
        self.registry.register(stream, Token(id), interest).unwrap();
    }

    pub fn set_waker(&self, cx: &Context, id: usize) {
        self.wakers
            .lock()
            .map(|mut wakers| wakers.insert(id, cx.waker().clone()))
            .unwrap();
    }

    pub fn deregister(&self, stream: &mut TcpStream, id: usize) {
        self.wakers.lock().map(|mut w| w.remove(&id)).unwrap();
        self.registry.deregister(stream).unwrap();
    }

    pub fn next_id(&self) -> usize {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }
}

fn event_loop(mut poll: Poll, wakers: Wakers) {
    let mut events = Events::with_capacity(100);
    loop {
        poll.poll(&mut events, None).unwrap();
        for e in &events {
            let Token(id) = e.token();
            let wakers = wakers.lock().unwrap();
            if let Some(waker) = wakers.get(&id) {
                waker.wake_by_ref();
            }
        }
    }
}
