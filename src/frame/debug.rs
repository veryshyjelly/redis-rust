use super::Frame;
use std::fmt::{Debug, Formatter};

impl Debug for Frame {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use Frame::*;
        match self {
            SimpleString(s) => write!(f, "{s:?}"),
            BulkString(s) => write!(f, "{s:?}"),
            SimpleError(e) => write!(f, "error({e:?})"),
            BulkError(e) => write!(f, "error({e:?})"),
            Integer(i) => write!(f, "{i}"),
            Array(a) => write!(f, "{a:?}"),
            Boolean(b) => write!(f, "{b:?}"),
            Double(d) => write!(f, "{d:?}"),
            BigNumber(b) => write!(f, "{b}"),
            VerbatimString(s) => write!(f, "{}:{:?}", s.0, s.1),
            Map(m) => write!(f, "{m:?}"),
            Attributes(a) => write!(f, "{a:?}"),
            Set(s) => write!(f, "{s:?}"),
            Push(p) => write!(f, "{p:?}"),
            RDB(v) => write!(f, "RDB FILE({} bytes)", v.len()),
            None(_) => write!(f, "None"),
        }
    }
}
