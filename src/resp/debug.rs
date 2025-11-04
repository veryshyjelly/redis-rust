use super::RESP;
use std::fmt::{Debug, Formatter};

impl Debug for RESP {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RESP::SimpleString(s) => write!(f, "{s:?}"),
            RESP::BulkString(s) => write!(f, "{s:?}"),
            RESP::SimpleError(e) => write!(f, "error({e:?})"),
            RESP::BulkError(e) => write!(f, "error({e:?})"),
            RESP::Integer(i) => write!(f, "{i:?}"),
            RESP::Array(a) => write!(f, "{a:?}"),
            RESP::Boolean(b) => write!(f, "{b:?}"),
            RESP::Double(d) => write!(f, "{d:?}"),
            RESP::BigNumber(b) => write!(f, "{b}"),
            RESP::VerbatimString(s) => write!(f, "{}:{:?}", s.0, s.1),
            RESP::Map(m) => write!(f, "{m:?}"),
            RESP::Attributes(a) => write!(f, "{a:?}"),
            RESP::Set(s) => write!(f, "{s:?}"),
            RESP::Push(p) => write!(f, "{p:?}"),
            RESP::RDB(v) => write!(f, "RDB FILE({} bytes)", v.len()),
            RESP::None(_) => write!(f, "None"),
        }
    }
}
