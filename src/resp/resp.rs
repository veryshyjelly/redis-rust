use std::collections::{HashMap, HashSet};

#[derive(Clone)]
pub enum RESP {
    SimpleString(String),                // +
    BulkString(String),                  // $
    SimpleError(String),                 // -
    BulkError(String),                   // !
    Integer(isize),                      // :
    Array(Vec<RESP>),                    // *
    Boolean(bool),                       // #
    Double(f64),                         // ,
    BigNumber(String),                   // (
    VerbatimString((String, String)),    // =
    Map(HashMap<Hashable, RESP>),        // %
    Attributes(HashMap<Hashable, RESP>), // |
    Set(HashSet<Hashable>),              // ~
    Push(Vec<RESP>),                     // >
    None,                                // _
}

impl Default for RESP {
    fn default() -> Self {
        RESP::None
    }
}

#[derive(Clone, Hash, Eq, PartialEq)]
pub enum Hashable {
    String(String),
    Integer(isize),
    Array(Vec<Hashable>),
    Boolean(bool),
    None,
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
    pub fn hashable(self) -> std::io::Result<Hashable> {
        use Hashable::*;

        let r = match self {
            RESP::SimpleString(s) => String(s),
            RESP::BulkString(s) => String(s),
            RESP::SimpleError(e) => String(e),
            RESP::BulkError(e) => String(e),
            RESP::Integer(i) => Integer(i),
            RESP::Array(a) => {
                let mut v = vec![];
                for ai in a {
                    v.push(ai.hashable()?);
                } 
                Array(v)
            },
            RESP::Boolean(b) => Boolean(b),
            RESP::BigNumber(s) => String(s),
            RESP::VerbatimString((_, v)) => String(v),
            RESP::None => None,
            _ => return Err(std::io::ErrorKind::InvalidData)?,
        };
        
        Ok(r)
    }

    impl_getter!(int, Integer, isize);
    impl_getter!(double, Double, f64);
    impl_getter!(boolean, Boolean, bool);
    impl_getter!(string, [SimpleString, BulkString], String);
    impl_getter!(error, [SimpleError, BulkError], String);
    impl_getter!(array, [Array, Push], Vec<RESP>);
    impl_getter!(map, [Map, Attributes], HashMap<Hashable, RESP>);
    impl_getter!(set, Set, HashSet<Hashable>);
}

impl Hashable {
    impl_getter!(string, String, String);
    impl_getter!(int, Integer, isize);
    impl_getter!(array, Array, Vec<Hashable>);
    impl_getter!(boolean, Boolean, bool);
}

impl Into<RESP> for Hashable {
    fn into(self) -> RESP {
        match self {
            Hashable::String(s) => RESP::BulkString(s),
            Hashable::Integer(i) => RESP::Integer(i),
            Hashable::Array(a) => RESP::Array(a.into_iter().map(|v| v.into()).collect()),
            Hashable::Boolean(b) => RESP::Boolean(b),
            Hashable::None => RESP::None,
        }
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
impl_into_resp!(HashSet<Hashable> => Set);
impl_into_resp!(HashMap<Hashable, RESP> => Map);