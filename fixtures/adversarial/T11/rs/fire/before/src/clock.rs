pub trait Clock {
    fn now(&self) -> u64;
}

pub struct Stamp {
    pub seconds: u64,
}

impl Stamp {
    pub fn from(clock: &dyn Clock) -> Stamp {
        Stamp {
            seconds: clock.now(),
        }
    }
}
