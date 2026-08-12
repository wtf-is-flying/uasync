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
