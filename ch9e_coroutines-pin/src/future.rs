use std::pin::Pin;

use crate::runtime::Waker;

pub trait Future {
    type Output;

    fn poll(self: Pin<&mut Self>, waker: &Waker) -> PollState<Self::Output>;
}

pub enum PollState<T> {
    Ready(T),
    NotReady,
}

impl<T> PollState<T> {
    pub fn is_ready(&self) -> bool {
        matches!(self, PollState::Ready(_))
    }
}

impl<T> From<PollState<T>> for std::task::Poll<T> {
    fn from(value: PollState<T>) -> Self {
        match value {
            PollState::Ready(x) => Self::Ready(x),
            PollState::NotReady => Self::Pending,
        }
    }
}

pub struct JoinAll<F> {
    futures: Vec<(bool, F)>,
    finished_count: usize,
}

pub fn join_all<F: Future>(futures: Vec<F>) -> JoinAll<Pin<Box<F>>> {
    JoinAll {
        futures: futures
            .into_iter()
            .map(|fut| (false, Box::pin(fut)))
            .collect(),
        finished_count: 0,
    }
}

impl<F: Future> Future for JoinAll<Pin<Box<F>>> {
    type Output = String;

    fn poll(mut self: Pin<&mut Self>, waker: &Waker) -> PollState<Self::Output> {
        let mut finished_count = 0;
        for (finished, fut) in &mut self.futures {
            if *finished {
                continue;
            }

            match fut.as_mut().poll(waker) {
                PollState::Ready(_) => {
                    *finished = true;
                    finished_count += 1;
                }
                PollState::NotReady => continue,
            }
        }

        self.finished_count = finished_count;
        if self.finished_count == self.futures.len() {
            PollState::Ready(String::new())
        } else {
            PollState::NotReady
        }
    }
}
