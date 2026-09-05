use crate::wire::{post, read, Failure};

pub fn send(payload: &str) -> String {
    post(payload).unwrap_or_default()
}

pub fn receive() -> Option<String> {
    read().ok()
}
