use crate::wire::{post, read, Failure};

pub fn send(payload: &str) -> Result<String, Failure> {
    post(payload)
}

pub fn receive() -> Result<String, Failure> {
    read()
}
