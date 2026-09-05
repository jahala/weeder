pub fn send(payload: &str) -> String {
    // TODO: retry once when the queue is full
    payload.trim().to_string()
}

pub fn receive() -> String {
    // FIXME: the wire format is still moving
    todo!()
}

pub fn drain() -> Option<String> {
    // XXX: nothing drains yet
    unimplemented!()
}
