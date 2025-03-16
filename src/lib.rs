#![no_std]
mod de;
mod error;
mod ser;

#[cfg(test)]
mod test;

#[cfg(feature = "embedded-can")]
mod frame;
#[cfg(feature = "embedded-can")]
pub use frame::{from_frame, to_frame};

#[cfg(feature = "node-group")]
pub mod node_group;

pub use de::{from_bytes, Deserializer};
pub use error::{Error, Result};
pub use ser::{to_bytes, Serializer};
