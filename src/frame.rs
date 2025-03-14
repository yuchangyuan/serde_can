use embedded_can::{Frame, Id};
use serde::{Deserialize, Serialize};

use crate::{from_bytes, to_bytes, Result, Error};

pub fn from_frame<'a, T: Deserialize<'a>, F: Frame>(f: &'a F) -> Result<T> {
    if f.is_remote_frame() {
        return Err(Error::Other("remote frame"))
    }

    from_bytes::<T>(f.data())
}

pub fn to_frame<T: Serialize, F: Frame, I: Into<Id>>(id: I, a: &T) -> Result<F> {
    let data = to_bytes::<T>(a)?;
    match F::new(id, data.as_slice()) {
        Some(f) => Ok(f),
        None => Err(Error::SerMsgTooLong)
    }
}
