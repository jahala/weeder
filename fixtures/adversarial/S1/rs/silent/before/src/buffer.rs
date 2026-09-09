use crate::client::Sink;

pub struct Buffer;

impl Sink for Buffer {
    fn send(&self, payload: &str) -> String {
        payload.trim().to_string()
    }

    fn retry(&self, payload: &str) -> String {
        self.send(payload)
    }
}
