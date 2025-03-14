use serde::{de, ser};
use core::fmt::Display;

#[derive(Debug, thiserror_no_std::Error, PartialEq)]
pub enum Error {
    #[error("message too long, not fit in 8 bytes")]
    SerMsgTooLong,
    #[error("index of field {1} in {0} too large")]
    SerFieldIndexTooLarge(&'static str, &'static str),
    #[error("{0} length of {1} too large")]
    SerLengthTooLarge(&'static str, usize),
    #[error("length unknown")]
    SerLengthUnknow,
    #[error("string/char decode fail, not valid utf-8")]
    DeUtf8DecodeFail,
    #[error("decode char fail, empty string")]
    DeCharFail,
    #[error("not enough bits to decode")]
    DeMsgTooLong,
    #[error("other error: {0}")]
    Other(&'static str),
    #[error("type {0} unsupport")]
    Unsupport(&'static str),
    #[error("serialize custom error")]
    SerCustom,
    #[error("deserialize custom error")]
    DeCustom,
}

impl core::error::Error for Error {}

pub type Result<T> = core::result::Result<T, Error>;

impl ser::Error for Error {
    fn custom<T: Display>(_msg: T) -> Self {
        Error::SerCustom
    }
}

impl de::Error for Error {
    fn custom<T: Display>(_msg: T) -> Self {
        Error::DeCustom
    }
}

#[cfg(test)]
mod test {
    use serde::{Serialize, Deserialize, de::DeserializeOwned};
    use core::fmt::Debug;
    use super::*;
    extern crate std;

    fn e_ser<T: Serialize + PartialEq + Debug>(a: &T, err: Error) {
        match crate::to_bytes(a) {
            Err(e) => assert_eq!(err, e),
            Ok(_) => panic!("ser should fail"),
        }
    }

    fn e_de<T: DeserializeOwned + PartialEq + Debug>(b: &[u8], err: Error) {
        match crate::from_bytes::<T>(b) {
            Err(e) => assert_eq!(err, e),
            Ok(_)  => panic!("de should fail"),
        }
    }

    #[test]
    fn ser_msg_too_long() {
        e_ser(&(0x1, 0x1, 0u8), Error::SerMsgTooLong);
    }

    #[test]
    fn ser_field_index_too_large() {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        enum E { A, B, C, E, F, G, H, I,
                 J, K, L, M, N, O, P, Q, FIdx16 }
        e_ser(&E::FIdx16, Error::SerFieldIndexTooLarge("E", "FIdx16"));
    }

    #[test]
    fn ser_length_too_large() {
        let x = std::vec![0u8; 16];
        e_ser(&x, Error::SerLengthTooLarge("seq", 16));
    }

    #[test]
    fn de_utf8_decode_fail() {
        e_de::<char>(&[0x2c, 0x32, 0x80], Error::DeUtf8DecodeFail);
    }

    #[test]
    fn de_char_fail() {
        e_de::<char>(&[0x0], Error::DeCharFail);
    }

    #[test]
    fn de_msg_too_long() {
        e_de::<[u8; 9]>(&[0; 9], Error::DeMsgTooLong);
    }
}
