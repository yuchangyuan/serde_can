extern crate std;
use core::fmt::Debug;
use serde::{Serialize, Deserialize, de::DeserializeOwned};

use super::*;

fn pass<T: Serialize + DeserializeOwned + PartialEq + Debug>(a: &T, b: &[u8]) {
    let s = to_bytes(a).unwrap();
    assert_eq!(s.len(), b.len());
    for i in 0..s.len() {
        assert_eq!(s[i], b[i]);
    }

    let b: T = crate::from_bytes(s.as_slice()).unwrap();
    assert_eq!(a, &b)
}

#[test]
fn t_bool() {
    pass(&false, &[0]);
    pass(&true, &[0x80]);

    let bin_ary = [true, false, true, false,
                   false, true, true, false,
                   false, false, true];
    pass(&bin_ary, &[0xa6, 0x20]);
}

#[test]
fn t_unit() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    enum E { A, B, }
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct S;

    pass(&(), &[]);
    pass(&E::A, &[0 << 4]);
    pass(&E::B, &[1 << 4]);
    pass(&(S), &[]);
}

#[test]
fn t_int() {
    pass(&0u8, &[0]);
    pass(&1u8, &[1]);
    pass(&100u8, &[100]);
    pass(&255u8, &[255]);

    pass(&0i8, &[0]);
    pass(&-1i8, &[0xff]);
    pass(&127i8, &[127]);
    pass(&-128i8, &[0x80]);


    pass(&0u16, &[0, 0]);
    pass(&1u16, &[0, 1]);
    pass(&1000u16, &[3, 232]);
    pass(&0xffffu16, &[0xff, 0xff]);

    pass(&0i16, &[0, 0]);
    pass(&-1i16, &[0xff, 0xff]);
    pass(&0x7f_ffi16, &[0x7f, 0xff]);
    pass(&-0x80_00i16, &[0x80, 0]);

    pass(&0u32, &[0, 0, 0, 0]);
    pass(&1u32, &[0, 0, 0, 1]);
    pass(&[0xffff_ffffu32, 0x1234_5678],
         &[0xff, 0xff, 0xff, 0xff, 0x12, 0x34, 0x56, 0x78]);

    pass(&0i32, &[0, 0, 0, 0]);
    pass(&-1i32, &[0xff, 0xff, 0xff, 0xff]);
    pass(&[0x7fff_ffffi32, 0x1234_5678],
         &[0x7f, 0xff, 0xff, 0xff, 0x12, 0x34, 0x56, 0x78]);

    pass(&0u64, &[0;8]);
    pass(&0xffffffff_ffffffffu64, &[0xff;8]);
    pass(&0xabcd1234_7856aa55u64, &[0xab, 0xcd, 0x12, 0x34,
                                    0x78, 0x56, 0xaa, 0x55]);

    pass(&0i64, &[0;8]);
    pass(&-1i64, &[0xff;8]);
    pass(&0x7654_3210_fedc_ba98u64, &[0x76, 0x54, 0x32, 0x10,
                                      0xfe, 0xdc, 0xba, 0x98]);
}

#[test]
fn t_float() {
    pass(&0.0f32, &[0;4]);
    pass(&-1.125f32, &[0xbf, 0x90, 0, 0]);
    pass(&1.234e-18f32, &[0x21, 0xb6, 0x1b, 0x34]);
    pass(&[65536.25f32, -1122.1234f32], &[0x47, 0x80, 0, 0x20, 0xc4, 0x8c, 0x43, 0xf3]);
    pass(&[std::f32::consts::PI, std::f32::consts::E], &[0x40, 0x49, 0x0f, 0xdb, 0x40, 0x2d, 0xf8, 0x54]);
    pass(&[std::f32::INFINITY, std::f32::NEG_INFINITY], &[0x7f,0x80,0,0, 0xff,0x80,0,0]);

    pass(&0f64, &[0;8]);
    pass(&1.234e-18, &[0x3c, 0x36, 0xc3, 0x66, 0x76, 0x1e, 0x9a, 0x29]);
    pass(&-3.4567891234125e10, &[0xc2, 0x20, 0x18, 0xd0, 0x52, 0x44, 0x40, 0x00]);
    pass(&std::f64::consts::PI, &[0x40, 0x09, 0x21, 0xfb, 0x54, 0x44, 0x2d, 0x18]);
    pass(&std::f64::INFINITY, &[0x7f,0xf0,0,0, 0,0,0,0]);
}

#[test]
fn t_char() {
    pass(&'a', &[0x16, 0x10]);
    pass(&'好', &[0x3e, 0x5a, 0x5b, 0xd0]);
}

#[test]
fn t_str() {
    use std::borrow::ToOwned;

    pass(&"abcdefg".to_owned(), &[0x76, 0x16, 0x26, 0x36, 0x46, 0x56, 0x66, 0x70]);
    pass(&"你好".to_owned(), &[0x6e, 0x4b, 0xda, 0x0e, 0x5a, 0x5b, 0xd0]);
}

#[test]
fn t_bytes() {
    #[derive(Deserialize, Serialize, PartialEq, Debug)]
    struct S {
        #[serde(with = "serde_bytes")]
        byte_buf: std::vec::Vec<u8>,
        #[serde(with = "serde_bytes")]
        byte_array: [u8; 3],
    }
    let x = S {
        byte_buf: std::vec![0x37, 0x21],
        byte_array: [0x55, 0xaa, 0x0f]
    };

    pass(&x, &[0x23, 0x72, 0x13, 0x55, 0xaa, 0x0f]);
}

#[test]
fn t_seq() {
    pass(&std::vec![0x1234,0x5678,0x9abcu16], &[0x31,0x23,0x45,0x67, 0x89, 0xab, 0xc0]);
}

#[test]
fn t_struct() {
    #[derive(Deserialize, Serialize, PartialEq, Debug)]
    struct Sub1 {
        a: i8,
        b: [u8; 1],
    }

    #[derive(Deserialize, Serialize, PartialEq, Debug)]
    struct Sub2(u16, u8);

    #[derive(Deserialize, Serialize, PartialEq, Debug)]
    struct Sub3(i8);

    #[derive(Deserialize, Serialize, PartialEq, Debug)]
    struct Top {
        x: u16,
        y: Sub1,
        z: Sub2,
        u: Sub3,
    }

    let v = Top {
        x: 0x1234,
        y: Sub1 { a: 0x24, b: [0x68] },
        z: Sub2(0xfedc, 0xba),
        u: Sub3(-1),
    };

    pass(&v, &[0x12, 0x34, 0x24, 0x68, 0xfe, 0xdc, 0xba, 0xff]);
}

#[test]
fn t_tuple() {
    pass(&(0x34u8, [0x56, 0x78u8], 0x9abcu16, (0xdeu8, 0xf1u8)),
         &[0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf1]);
    pass(&(0x12345678, (0xabcdu16, 0xefu8)),
         &[0x12, 0x34, 0x56, 0x78, 0xab, 0xcd, 0xef]);
}

#[test]
fn t_enum() {
    #[derive(Deserialize, Serialize, PartialEq, Debug)]
    enum E {
        A,
        B(i16, u16),
        C { x: i16, y: [u8; 2] },
        D(u32),
    }

    pass(&E::A, &[0x0]);
    pass(&E::B(-2, 0x1234), &[0x1f, 0xff, 0xe1, 0x23, 0x40]);
    pass(&E::C {x: -0x5679, y: [0x12, 0x34]}, &[0x2a, 0x98, 0x71, 0x23, 0x40]);
    pass(&E::D(0x8765_4321), &[0x38, 0x76, 0x54, 0x32, 0x10]);
}

#[test]
fn t_option() {
    //#[derive(Deserialize, Serialize, PartialEq, Debug)]
    type T1 = Option<u32>;
    type T2 = (T1, u16);
    let a: T1  = None;
    let b: T2 = (None, 0x8765);
    let c: T1 = Some(0x2345_6789);
    let d: T2 = (Some(0x5678_4321), 0xa987);

    pass(&a, &[0x0]);
    pass(&b, &[0x43, 0xb2, 0x80]);
    pass(&c, &[0x91, 0xa2, 0xb3, 0xc4, 0x80]);
    pass(&d, &[0xab, 0x3c, 0x21, 0x90, 0xd4, 0xc3, 0x80]);
}
