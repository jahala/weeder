use demo::clock::{Clock, Stamp};

struct FakeClock {
    seconds: u64,
}

impl FakeClock {
    fn new() -> FakeClock {
        FakeClock { seconds: 0 }
    }

    fn set(&mut self, seconds: u64) {
        self.seconds = seconds;
    }
}

impl Clock for FakeClock {
    fn now(&self) -> u64 {
        self.seconds
    }
}

#[test]
fn a_new_clock_reads_zero() {
    let clock = FakeClock::new();
    assert_eq!(Stamp::from(&clock).seconds, 0);
}

#[test]
fn stamps_what_the_clock_reads() {
    let mut clock = FakeClock::new();
    clock.set(5);
    let stamp = Stamp::from(&clock);
    assert_eq!(stamp.seconds, 5);
}
