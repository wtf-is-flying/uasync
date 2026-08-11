use std::io::{Read, Write};

use crate::future::{Future, PollState};

pub struct Http;

impl Http {
    pub fn get(path: &str) -> impl Future<Output = String> {
        HttpGetFuture::new(path)
    }
}

struct HttpGetFuture {
    // Option since we don't necessarily connect to the stream
    // when the future is created.
    stream: Option<mio::net::TcpStream>,
    buffer: Vec<u8>,
    path: String,
}

impl HttpGetFuture {
    fn new(path: &str) -> Self {
        Self {
            stream: None,
            buffer: Vec::new(),
            path: path.to_owned(),
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

    fn poll(&mut self) -> PollState<Self::Output> {
        if self.stream.is_none() {
            println!("FIRST POLL - START OPERATION");
            self.write_request();
            return PollState::NotReady;
        }

        let mut buf = vec![0u8; 4096];
        loop {
            match self.stream.as_mut().unwrap().read(&mut buf) {
                Ok(0) => {
                    let s = String::from_utf8_lossy(&self.buffer);
                    break PollState::Ready(s.to_string());
                }
                Ok(n) => {
                    self.buffer.extend(&buf[0..n]);
                    continue;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
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
