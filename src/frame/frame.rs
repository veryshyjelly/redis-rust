use super::Frame;
use std::collections::{HashMap, HashSet};

macro_rules! impl_getter {
    // single variant
    ($name:ident, $variant:ident, $ty:ty) => {
        pub fn $name(self) -> Option<$ty> {
            if let Self::$variant(v) = self {
                Some(v)
            } else {
                None
            }
        }
    };
    // multiple variants
    ($name:ident, [$($variant:ident),+], $ty:ty) => {
        pub fn $name(self) -> Option<$ty> {
            match self {
                $(Self::$variant(v))|+ => Some(v),
                _ => None,
            }
        }
    };
}

impl Frame {
    impl_getter!(int, Integer, isize);
    impl_getter!(double, Double, f64);
    impl_getter!(boolean, Boolean, bool);
    impl_getter!(array, [Array, Push], Vec<Frame>);
    impl_getter!(map, [Map, Attributes], HashMap<String, Frame>);
    impl_getter!(bulk_string, BulkString, bytes::Bytes);
    impl_getter!(bulk_error, BulkError, bytes::Bytes);
    impl_getter!(set, Set, HashSet<String>);

    pub fn is_array(&self) -> bool {
        match self {
            Frame::Array(_) => true,
            _ => false,
        }
    }

    pub fn string(self) -> Option<String> {
        match self {
            Frame::SimpleString(s) => Some(s),
            Frame::BulkString(s) => String::from_utf8(s.to_vec()).ok(),
            _ => None,
        }
    }

    pub fn error(self) -> Option<String> {
        match self {
            Frame::SimpleError(s) => Some(s),
            Frame::BulkError(s) => String::from_utf8(s.to_vec()).ok(),
            _ => None,
        }
    }
}

macro_rules! impl_into_frame {
    ($ty:ty => $variant:ident) => {
        impl Into<Frame> for $ty {
            fn into(self) -> Frame {
                Frame::$variant(self)
            }
        }
    };
    // type + custom conversion (like &str -> String)
    ($ty:ty => $variant:ident, $conv:expr) => {
        impl Into<Frame> for $ty {
            fn into(self) -> Frame {
                Frame::$variant($conv(self))
            }
        }
    };
}

impl_into_frame!(usize => Integer, |v: usize| v as isize);
impl_into_frame!(isize => Integer);
impl_into_frame!(f64 => Double);
impl_into_frame!(bool => Boolean);
impl_into_frame!(&str => SimpleString, |s: &str| s.to_string());
impl_into_frame!(String => BulkString, |s: String| bytes::Bytes::from(s));
impl_into_frame!(Vec<Frame> => Array);
impl_into_frame!(Vec<String> => Array, |v: Vec<String>| v.into_iter().map(|x| x.into()).collect());
impl_into_frame!(Vec<&str> => Array, |v: Vec<&str>| v.into_iter().map(|x| x.to_string().into()).collect());
impl_into_frame!(HashSet<String> => Set);
impl_into_frame!(HashMap<String, Frame> => Map);
