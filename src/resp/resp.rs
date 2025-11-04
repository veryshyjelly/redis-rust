use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;

#[derive(Copy, Clone)]
pub enum TypedNone {
    String,
    Array,
    Nil,
}

#[derive(Clone)]
pub enum RESP {
    SimpleString(String),              // +
    BulkString(String),                // $
    SimpleError(String),               // -
    BulkError(String),                 // !
    Integer(isize),                    // :
    Array(Vec<RESP>),                  // *
    Boolean(bool),                     // #
    Double(f64),                       // ,
    BigNumber(String),                 // (
    VerbatimString((String, String)),  // =
    Map(HashMap<String, RESP>),        // %
    Attributes(HashMap<String, RESP>), // |
    Set(HashSet<String>),              // ~
    Push(Vec<RESP>),                   // >
    None(TypedNone),                   // _
}

impl Default for RESP {
    fn default() -> Self {
        RESP::None(TypedNone::Nil)
    }
}

pub type Result = Option<(usize, RESP)>;

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

impl RESP {
    impl_getter!(int, Integer, isize);
    impl_getter!(double, Double, f64);
    impl_getter!(boolean, Boolean, bool);
    impl_getter!(string, [SimpleString, BulkString], String);
    impl_getter!(error, [SimpleError, BulkError], String);
    impl_getter!(array, [Array, Push], Vec<RESP>);
    impl_getter!(map, [Map, Attributes], HashMap<String, RESP>);
    impl_getter!(set, Set, HashSet<String>);
}

impl From<Vec<&str>> for RESP {
    fn from(value: Vec<&str>) -> Self {
        value
            .into_iter()
            .map(|v| v.to_string().into())
            .collect::<Vec<RESP>>()
            .into()
    }
}

impl From<Vec<String>> for RESP {
    fn from(value: Vec<String>) -> Self {
        value
            .into_iter()
            .map(|v| v.into())
            .collect::<Vec<RESP>>()
            .into()
    }
}

macro_rules! impl_into_resp {
    // single type to variant
    ($ty:ty => $variant:ident) => {
        impl Into<RESP> for $ty {
            fn into(self) -> RESP {
                RESP::$variant(self)
            }
        }
    };
    // type + custom conversion (like &str -> String)
    ($ty:ty => $variant:ident, $conv:expr) => {
        impl Into<RESP> for $ty {
            fn into(self) -> RESP {
                RESP::$variant($conv(self))
            }
        }
    };
}

impl_into_resp!(usize => Integer, |v: usize| v as isize);
impl_into_resp!(isize => Integer);
impl_into_resp!(f64 => Double);
impl_into_resp!(bool => Boolean);
impl_into_resp!(&str => SimpleString, |s: &str| s.to_string());
impl_into_resp!(String => BulkString);
impl_into_resp!(Vec<RESP> => Array);
impl_into_resp!(HashSet<String> => Set);
impl_into_resp!(HashMap<String, RESP> => Map);
