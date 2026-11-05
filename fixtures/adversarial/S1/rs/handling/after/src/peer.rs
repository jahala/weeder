use crate::errors::Error;
use crate::errors::NotImplemented;

pub fn answer(code: u16) -> String {
    code.to_string()
}

pub fn reply(error: &Error) -> &'static str {
    match error {
        Error::NotImplemented(_) => "the peer has not written this method",
        _ => "failed",
    }
}

pub fn is_unwritten(error: &Error) -> bool {
    matches!(error, Error::NotImplemented(_))
}

pub fn wrap(error: NotImplemented) -> Error {
    Error::from(error)
}

pub fn resume(_name: &str) -> String {
    unimplemented!()
}
