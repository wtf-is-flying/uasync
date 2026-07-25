use std::{
    io::{self, Result},
    net::TcpStream,
    os::fd::AsRawFd,
};

use crate::ffi;

type Events = Vec<ffi::Event>;

/// Represents the event queue.
pub struct Poll {
    registry: Registry,
}

impl Poll {
    pub fn new() -> Result<Self> {
        let raw_fd = unsafe { ffi::epoll_create(1) };
        if raw_fd < 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(Self {
            registry: Registry { raw_fd },
        })
    }

    /// Returns a reference to the registry to register interest in new events.
    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    /// Blocks the thread until an event is ready or it times out.
    pub fn poll(&mut self, events: &mut Events, timeout: Option<i32>) -> Result<()> {
        let n_events = unsafe {
            ffi::epoll_wait(
                self.registry.raw_fd,
                events.as_mut_ptr(),
                events.capacity() as i32,
                timeout.unwrap_or(-1),
            )
        };
        if n_events < 0 {
            return Err(io::Error::last_os_error());
        }

        unsafe { events.set_len(n_events as usize) };

        Ok(())
    }
}

/// A handle to register interest in new events.
pub struct Registry {
    raw_fd: i32,
}

impl Registry {
    pub fn register(&self, source: &TcpStream, token: usize, interests: u32) -> Result<()> {
        let mut event = ffi::Event {
            events: interests,
            epoll_data: token,
        };

        let res = unsafe {
            ffi::epoll_ctl(
                self.raw_fd,
                ffi::EPOLL_CTL_ADD,
                source.as_raw_fd(),
                &mut event,
            )
        };
        if res < 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }
}

impl Drop for Registry {
    fn drop(&mut self) {
        if unsafe { ffi::close(self.raw_fd) } < 0 {
            let err = io::Error::last_os_error();
            eprintln!("ERROR: {err:?}");
        }
    }
}
