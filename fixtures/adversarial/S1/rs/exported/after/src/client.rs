pub trait Sink {
    fn send(&self, payload: &str) -> String;

    fn retry(&self, payload: &str) -> String {
        todo!()
    }
}
