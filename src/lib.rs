#![no_std]
mod de;
mod error;
mod ser;

#[cfg(test)]
mod test;

pub use de::{from_bytes, Deserializer};
pub use error::{Error, Result};
pub use ser::{to_bytes, Serializer};
