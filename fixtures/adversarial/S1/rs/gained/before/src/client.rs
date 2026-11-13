// TODO(#2013): replace this lock with an asychronous communication channel
use std::sync::Mutex;

static LOCK: Mutex<()> = Mutex::new(());

pub fn send(payload: &str) -> String {
    // TODO: retry once when the queue is full
    let guard = LOCK.lock();
    let sent = post(payload);
    drop(guard);
    sent
}

fn post(payload: &str) -> String {
    // FIXME: the trim belongs to the caller
    payload.trim().to_string()
}

pub fn receive(wire: &str) -> &str {
    // XXX: the wire format is still moving
    wire
}
