use crate::wire::{post, read, Failure};

pub fn send(payload: &str) -> Result<String, Failure> {
    post(payload).map_err(|error| Failure::Send(error.to_string()))
}

pub fn receive() -> Result<String, Failure> {
    match read() {
        Ok(value) => Ok(value),
        Err(error) => {
            report(&format!("receive refused: {error}"));
            Err(Failure::Receive)
        }
    }
}

fn report(what: &str) {
    log::warn!("{what}");
}
