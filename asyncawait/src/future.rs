pub trait Future {
    type Output;

    fn poll(&mut self) -> PollState<Self::Output>;
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

pub fn join_all<F: Future>(futures: Vec<F>) -> JoinAll<F> {
    JoinAll {
        futures: futures.into_iter().map(|fut| (false, fut)).collect(),
        finished_count: 0,
    }
}

impl<F: Future> Future for JoinAll<F> {
    type Output = String;

    fn poll(&mut self) -> PollState<Self::Output> {
        for (finished, fut) in &mut self.futures {
            if *finished {
                continue;
            }

            match fut.poll() {
                PollState::Ready(_) => {
                    *finished = true;
                    self.finished_count += 1;
                }
                PollState::NotReady => continue,
            }
        }

        if self.finished_count == self.futures.len() {
            PollState::Ready(String::new())
        } else {
            PollState::NotReady
        }
    }
}
