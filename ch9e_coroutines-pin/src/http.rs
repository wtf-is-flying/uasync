use std::{
    io::{Read, Write},
    pin::Pin,
};

use mio::Interest;

use crate::{
    future::{Future, PollState},
    runtime::{self, Waker},
};

pub struct Http;

impl Http {
    pub fn get(path: &str) -> impl Future<Output = String> + use<> {
        HttpGetFuture::new(path)
    }
}

struct HttpGetFuture {
    // Option since we don't necessarily connect to the stream
    // when the future is created.
    stream: Option<mio::net::TcpStream>,
    buffer: Vec<u8>,
    path: String,
    id: usize,
}

impl HttpGetFuture {
    fn new(path: &str) -> Self {
        Self {
            stream: None,
            buffer: Vec::new(),
            path: path.to_owned(),
            id: runtime::reactor().next_id(),
        }
    }

    fn write_request(&mut self) {
        let stream = std::net::TcpStream::connect("127.0.0.1:8080")
            .expect("failed to create and connect stream");
        stream
            .set_nonblocking(true)
            .expect("failed to set stream to non blocking");
        let mut stream = mio::net::TcpStream::from_std(stream);
        stream
            .write_all(get_request(&self.path).as_bytes())
            .unwrap();
        self.stream = Some(stream);
    }
}

impl Future for HttpGetFuture {
    type Output = String;

    fn poll(mut self: Pin<&mut Self>, waker: &Waker) -> PollState<Self::Output> {
        let id = self.id;
        if self.stream.is_none() {
            println!("FIRST POLL - START OPERATION");
            self.write_request();
            let stream = self.stream.as_mut().unwrap();
            runtime::reactor().register(stream, Interest::READABLE, id);
            runtime::reactor().set_waker(waker, self.id);
        }

        let mut buf = vec![0u8; 4096];
        loop {
            match self.stream.as_mut().unwrap().read(&mut buf) {
                Ok(0) => {
                    let s = String::from_utf8_lossy(&self.buffer).to_string();
                    runtime::reactor().deregister(self.stream.as_mut().unwrap(), id);
                    break PollState::Ready(s);
                }
                Ok(n) => {
                    self.buffer.extend(&buf[0..n]);
                    continue;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // Make sure that the Waker from the *most recent poll() call*
                    // (i.e., the one passed in arguments) is scheduled to wake up.
                    //
                    // The reason is that the future could have been moved
                    // to a different executor in between calls,
                    // though it's not possible with our implementation.
                    // See: https://doc.rust-lang.org/stable/std/future/trait.Future.html#tymethod.poll
                    runtime::reactor().set_waker(waker, self.id);
                    break PollState::NotReady;
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {
                    continue;
                }
                Err(e) => panic!("{e:?}"),
            }
        }
    }
}

fn get_request(path: &str) -> String {
    format!(
        "GET {path} HTTP/1.1\r\n\
        Host: localhost\r\n\
        Connection: close \r\n\
        \r\n"
    )
}
