mod executor;
mod reactor;

pub use executor::{Executor, MyWaker, spawn};
pub use reactor::reactor;

pub fn init() -> Executor {
    reactor::start();
    Executor::new()
}
