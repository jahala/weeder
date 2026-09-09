pub trait Sink {
    fn send(&self, payload: &str) -> String;
}
