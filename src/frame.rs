use embedded_can::{Frame, Id};
use serde::{Deserialize, Serialize};

use crate::{from_bytes, to_bytes};

#[derive(Debug, thiserror_no_std::Error, PartialEq)]
pub enum Error {
    #[error("remote frame")]
    RemoteFrame,
    #[error("message too long")]
    MsgTooLong,
    #[error("serde error {0}")]
    SerdeErr(crate::Error),
}

pub fn from_frame<'a, T: Deserialize<'a>, F: Frame>(f: &'a F) -> Result<T, Error> {
    if f.is_remote_frame() {
        return Err(Error::RemoteFrame)
    }

    from_bytes::<T>(f.data()).map_err(Error::SerdeErr)
}

pub fn to_frame<T: Serialize, F: Frame, I: Into<Id>>(id: I, a: &T) -> Result<F, Error> {
    let data = to_bytes::<T>(a).map_err(Error::SerdeErr)?;

    match F::new(id, data.as_slice()) {
        Some(f) => Ok(f),
        None => Err(Error::MsgTooLong)
    }
}
