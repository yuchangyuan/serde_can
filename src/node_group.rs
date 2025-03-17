use core::default::Default;
use core::marker::PhantomData;

/* | xxxx |   yyyyy   |  zzzz  |   -> total 29bits or 11bits
     BASE   (node_id)  (msg_id)
 */

use core::any::{TypeId, Any};
use embedded_can::{Frame, Id, ExtendedId};
use super::{from_frame, to_frame};
use serde::{Serialize, Deserialize};

#[derive(Debug, thiserror_no_std::Error, PartialEq)]
pub enum Error {
   #[error("node_id out of range, {0} > 2**{1}")]
    EncNodeIdOutOfRange(u32, usize),
    #[error("can msg id of {0} out of range")]
    EncCanIdOutOfRange(u32),
    #[error("msg_id mismatch, {0} != {1}")]
    DecMsgIdMismatch(u32, u32),
    #[error("not this node group")]
    DecNodeGroupMismatch,
    #[error("frame error: {0}")]
    FrameErr(crate::frame::Error),
    #[error("serde error: {0}")]
    SerdeErr(crate::Error),
}

// -------------------------------------- msg list
pub trait List: private::Sealed {
    fn msg_id<T: Any>() -> i32;
    const LEN: usize;
}

pub trait Elem<T: List> {}

mod private {
    use super::{List, Cons, Nil};
    use core::any::Any;

    pub trait Sealed {}
    impl Sealed for Nil {}
    impl <H: Any, T: List> Sealed for Cons<H, T> {}
}

#[derive(Default, Debug)]
pub struct Nil;
impl List for Nil {
    const LEN: usize = 0;

    fn msg_id<T: Any>() -> i32 { -1 }
}

#[derive(Default, Debug)]
pub struct Cons<H: Any, T: List> {
    _phantom: PhantomData<(H,T)>
}

impl <H: Any, T: List> List for Cons<H, T> {
    const LEN: usize = T::LEN + 1;

    fn msg_id<X: Any>() -> i32 {
        if TypeId::of::<X>() == TypeId::of::<H>() { return 0 as i32 }
        let mut r = T::msg_id::<X>();
        if r >= 0 { r += 1 }
        r
    }
}

#[macro_export]
macro_rules! node_group_msg_list {
    [] => { Nil };

    [ $head:ty $(,)? ] => { Cons<$head, Nil> };

    [ $head:ty, $( $tail:ty ),+ $(,)? ] => {
        Cons<$head, $crate::node_group_msg_list![$( $tail ),+]>
    };
}

#[macro_export]
macro_rules! node_group_msg_impl_elem {
    ( $tp: ident, [$h: ty $(,)? ]) => {
        impl Elem<$tp> for $h {}
    };

    ( $tp: ident, [$h: ty, $( $t: ty ),* $(,)? ] ) => {
        impl Elem<$tp> for $h {}
        $crate::node_group_msg_impl_elem!{$tp, [$( $t ),*]}
    }
}

#[macro_export]
macro_rules! node_group_msg_def {
    ( $tp: ident, [$( $e: ty ),* $(,)? ] ) => {
        type $tp = $crate::node_group_msg_list![$( $e ),*];
        $crate::node_group_msg_impl_elem!{$tp, [$( $e ),*]}
    }
}

// ---------------------------------- node group
type NodeId = u32;
type MsgId  = u32;


#[derive(Default, Debug)]
pub struct NodeGroup<L: List, const BASE: u32, const NODE_ID_LEN: usize, const MSG_ID_LEN: usize> {
    pub name: &'static str,
    _phantom: PhantomData<L>,
}

impl <L: List, const BASE: u32, const NODE_ID_LEN: usize, const MSG_ID_LEN: usize>
    NodeGroup<L, BASE, NODE_ID_LEN, MSG_ID_LEN>
{

    const MSG_ID_MASK: u32  = ((1 << MSG_ID_LEN) - 1) as u32;
    const NODE_ID_MASK: u32 = (((1 << NODE_ID_LEN) - 1) as u32) << MSG_ID_LEN;
    const BASE_MASK: u32    = !(Self::MSG_ID_MASK | Self::NODE_ID_MASK);

    pub fn msg_id<X: Any + Elem<L>>() -> i32 { L::msg_id::<X>() }

    fn id2raw(id: &Id) -> u32 {
        match id {
            Id::Standard(x) => x.as_raw() as u32,
            Id::Extended(x) => x.as_raw()
        }
    }

    pub const fn new(name: &'static str) -> Self {
        assert!(NODE_ID_LEN + MSG_ID_LEN <= 29,
                "sum of NODE_ID_LEN & MSG_ID_LEN too large");
        assert!(BASE & (Self::MSG_ID_MASK | Self::NODE_ID_MASK) == 0,
                "msg_id part & node_id part of `BASE' should be 0");
        assert!(L::LEN <= (1 << MSG_ID_LEN),
                "number of msg should fit within MSG_ID_LEN bits");

        assert!(BASE & 0xe000_0000 == 0, "BASE length > 29");

        Self { name, _phantom: PhantomData {} }
    }

    pub fn encode_ext<F: Frame, X: Serialize + Any + Elem<L>>(node_id: NodeId, x: &X) -> Result<F, Error> {
        let msg_id = Self::msg_id::<X>();

        let can_id = BASE | (node_id << (MSG_ID_LEN as u32)) | (msg_id as u32);
        let Some(ext_id) = ExtendedId::new(can_id) else {
            return Err(Error::EncCanIdOutOfRange(can_id))
        };

        if node_id >= (1 << NODE_ID_LEN) {
            return Err(Error::EncNodeIdOutOfRange(node_id, NODE_ID_LEN))
        }

        to_frame(Id::Extended(ext_id), x).map_err(|x| match x {
                crate::frame::Error::SerdeErr(e) => Error::SerdeErr(e),
                _ => Error::FrameErr(x),
        })
    }

    fn extract(id: &Id) -> Option<(NodeId, MsgId)> {
        let id_raw = Self::id2raw(id);

        if (id_raw & Self::BASE_MASK) != BASE { return None }
        let msg_id = id_raw & Self::MSG_ID_MASK;
        let node_id = (id_raw & Self::NODE_ID_MASK) >> MSG_ID_LEN;

        Some((node_id, msg_id))
    }

    pub fn decode<'a, T: Any + Deserialize<'a> + Elem<L>, F: Frame>(f: &'a F) -> Result<(NodeId, T), Error> {
        let Some((node_id, msg_id)) = Self::extract(&f.id()) else {
            return Err(Error::DecNodeGroupMismatch);
        };

        if msg_id != Self::msg_id::<T>() as u32 {
            return Err(Error::DecMsgIdMismatch(msg_id, Self::msg_id::<T>() as u32));
        }

        let res = from_frame::<T, F>(&f).map_err(|err| match err {
            crate::frame::Error::SerdeErr(e) => Error::SerdeErr(e),
            _ => Error::FrameErr(err),
        })?;

        Ok((node_id, res))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    extern crate std;

    #[derive(Debug, PartialEq)]
    struct Frame { id: Id, data: [u8; 8], dlc: usize, remote: bool }
    impl embedded_can::Frame for Frame {
        fn new(id: impl Into<Id>, data: &[u8]) -> Option<Self> {
            if data.len() > 8 { return None }
            let mut res_data = [0u8; 8];
            for i in 0..data.len() { res_data[i] = data[i]; }
            let res_dlc = data.len();
            Some(Frame {id: id.into(), data: res_data, dlc: res_dlc, remote: false})
        }

        fn new_remote(id: impl Into<Id>, dlc: usize) -> Option<Self> {
            if dlc > 8 { return None }
            Some(Frame {id: id.into(), data: [0u8;8], dlc, remote: true})
        }

        fn is_extended(&self) -> bool {
            match self.id {
                embedded_can::Id::Extended(_) => true,
                _ => false
            }
        }

        fn is_remote_frame(&self) -> bool { self.remote }
        fn id(&self) -> Id { self.id }
        fn dlc(&self) -> usize { self.dlc }
        fn data<'a>(&'a self) -> &'a [u8] {
            if self.remote { return &[] }
            &self.data[0..self.dlc]
        }
    }

    node_group_msg_def!(T4, [isize, u8, i8, usize]);
    node_group_msg_def!(T5, [u32, isize, u8, i8, usize,]);

    #[test]
    #[should_panic]
    fn assert_base() {
        let _g = NodeGroup::<T4, 0x1230, 2, 3>::new("g");
    }

    #[test]
    #[should_panic]
    fn assert_base_len() {
        let _g = NodeGroup::<T4, 0x2000_0000, 2, 3>::new("g");
    }

    #[test]
    #[should_panic]
    fn assert_total_len() {
        let _g = NodeGroup::<T4, 0x0, 15, 15>::new("g");
    }

    #[test]
    #[should_panic]
    fn assert_msg_num() {
        let _g = NodeGroup::<T5, 0x0, 2, 2>::new("g");
    }

    #[test]
    fn msg_id() {
        assert_eq!(T4::msg_id::<usize>(), 3);
        assert_eq!(T4::msg_id::<i8>(),    2);
        assert_eq!(T4::msg_id::<u8>(),    1);
        assert_eq!(T4::msg_id::<isize>(), 0);
        //assert_eq!(T4::msg_id::<u32>(),  -1);

        assert_eq!(T5::msg_id::<usize>(), 4);
        assert_eq!(T5::msg_id::<u32>(),   0);
        //assert_eq!(T5::msg_id::<i32>(),  -1);
    }

    #[test]
    fn endec() {
        type G0 = NodeGroup::<T5, 0x1_9876_540, 3, 3>;

        let Ok(f) = G0::encode_ext::<Frame,_>(3, &12345u32) else { panic!("fail") };

        match G0::decode::<u32,_>(&f) {
            Ok((node_id, msg)) => {
                assert_eq!(node_id, 3);
                assert_eq!(msg, 12345);
            },
            _ => panic!("fail"),
        }

        if let Ok(_) = G0::decode::<usize, _>(&f) {
            panic!("fail");
        }
    }

    #[test]
    fn err() {
        type G0 = NodeGroup::<T5, 0x1_9876_540, 3, 3>;
        type G1 = NodeGroup::<T4, 0x1_1234_560, 3, 3>;
        type G2 = NodeGroup::<T4, 0x2_0000_000, 3, 3>; // BASE error, should fail when call G2::new('')

        assert_eq!(G1::encode_ext::<Frame, _>(8, &0u8), Err(Error::EncNodeIdOutOfRange(8, 3)));
        assert_eq!(G2::encode_ext::<Frame, _>(0, &0u8), Err(Error::EncCanIdOutOfRange(0x2_0000_001)));

        let Ok(f) = G0::encode_ext::<Frame, _>(5, &-123i8) else { panic!("fail") };

        assert_eq!(G0::decode::<u8, _>(&f), Err(Error::DecMsgIdMismatch(3, 2)));
        assert_eq!(G1::decode::<i8, _>(&f), Err(Error::DecNodeGroupMismatch));
    }
}
