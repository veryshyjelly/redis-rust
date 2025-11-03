use std::collections::{HashMap, HashSet};

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

impl RESP {
    pub fn int(self) -> Option<isize> {
        match self {
            RESP::Integer(v) => Some(v),
            _ => None,
        }
    }

    pub fn string(self) -> Option<String> {
        match self {
            RESP::SimpleString(s) => Some(s),
            RESP::BulkString(s) => Some(s),
            _ => None,
        }
    }

    pub fn error(self) -> Option<String> {
        match self {
            RESP::SimpleError(e) => Some(e),
            RESP::BulkError(e) => Some(e),
            _ => None,
        }
    }

    pub fn double(self) -> Option<f64> {
        match self {
            RESP::Double(f) => Some(f),
            _ => None,
        }
    }

    pub fn array(self) -> Option<Vec<RESP>> {
        match self {
            RESP::Array(v) => Some(v),
            RESP::Push(v) => Some(v),
            _ => None,
        }
    }

    pub fn map(self) -> Option<HashMap<Hashable, RESP>> {
        match self {
            RESP::Map(v) => Some(v),
            RESP::Attributes(v) => Some(v),
            _ => None,
        }
    }

    pub fn hashable(self) -> Hashable {
        use Hashable::*;

        match self {
            RESP::SimpleString(s) => String(s),
            RESP::BulkString(s) => String(s),
            RESP::SimpleError(e) => String(e),
            RESP::BulkError(e) => String(e),
            RESP::Integer(i) => Integer(i),
            RESP::Array(a) => Array(a.into_iter().map(|x| x.hashable()).collect()),
            RESP::Boolean(b) => Boolean(b),
            RESP::BigNumber(s) => String(s),
            RESP::VerbatimString((_, v)) => String(v),
            RESP::None => None,
            _ => panic!("unhashable type given to hash"),
        }
    }
}

impl Into<RESP> for String {
    fn into(self) -> RESP {
        RESP::BulkString(self)
    }
}

impl Into<RESP> for &str {
    fn into(self) -> RESP {
        RESP::SimpleString(self.to_string())
    }
}
