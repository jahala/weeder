//! The connection manager keeps one connection open.

/// Dials until the server answers or the tries run out, and says how many
/// tries it took and whether it connected.
pub fn retry_connect(mut dial: impl FnMut() -> bool, tries: u32) -> (u32, bool) {
    for attempt in 1..=tries {
        if dial() {
            return (attempt, true);
        }
    }
    (tries, false)
}

pub struct Manager;

impl Manager {
    pub fn open(&self, dial: impl FnMut() -> bool) -> bool {
        retry_connect(dial, 1).1
    }
}
