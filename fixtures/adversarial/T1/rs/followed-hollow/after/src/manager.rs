//! The connection manager keeps one connection open.

pub struct Manager;

impl Manager {
    pub fn open(&self, mut dial: impl FnMut() -> bool) -> bool {
        dial()
    }
}
