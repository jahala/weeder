//! The client talks to the server.

pub struct Client;

impl Client {
    pub fn connect(&self, mut dial: impl FnMut() -> bool) -> bool {
        dial()
    }
}
