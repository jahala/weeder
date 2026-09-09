pub fn send(payload: &str) -> String {
    payload.trim().to_string()
}

pub trait Sink {
    fn send(&self, payload: &str) -> String;
}
