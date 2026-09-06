pub trait Clock {
    fn now(&self) -> u64;
}

pub struct Steady;

impl Clock for Steady {
    fn now(&self) -> u64 {
        0
    }
}
