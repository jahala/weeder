// TODO(#2013): replace this lock with an asynchronous communication channel
use std::sync::Mutex;

static LOCK: Mutex<()> = Mutex::new(());

pub fn send(payload: &str) -> String {
    let guard = LOCK.lock();
    let sent = post(payload);
    // TODO: retry once when the queue is full
    drop(guard);
    sent
}

fn post(payload: &str) -> String {
    payload.trim().to_string()
}
