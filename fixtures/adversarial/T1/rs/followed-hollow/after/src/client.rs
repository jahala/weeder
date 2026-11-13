//! The client talks to the server.

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

pub struct Client;

impl Client {
    pub fn connect(&self, dial: impl FnMut() -> bool) -> bool {
        retry_connect(dial, 3).1
    }
}
