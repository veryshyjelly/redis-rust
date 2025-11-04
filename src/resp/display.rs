use super::RESP;
use crate::resp::resp::TypedNone;
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};

impl Display for RESP {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RESP::SimpleString(s) => RESP::fmt_simple_string(s, f),
            RESP::BulkString(s) => RESP::fmt_bulk_string(s, f),
            RESP::SimpleError(e) => RESP::fmt_simple_error(e, f),
            RESP::BulkError(e) => RESP::fmt_bulk_error(e, f),
            RESP::Integer(i) => RESP::fmt_integer(i, f),
            RESP::Array(a) => RESP::fmt_array(a, f),
            RESP::Boolean(b) => RESP::fmt_boolean(b, f),
            RESP::Double(d) => RESP::fmt_double(d, f),
            RESP::BigNumber(n) => RESP::fmt_big_number(n, f),
            RESP::VerbatimString(v) => RESP::fmt_verbatim_string(v, f),
            RESP::Map(m) => RESP::fmt_map(m, f),
            RESP::Attributes(a) => RESP::fmt_attributes(a, f),
            RESP::Set(s) => RESP::fmt_set(s, f),
            RESP::Push(p) => RESP::fmt_push(p, f),
            RESP::RDB(v) => RESP::fmt_rdb(v, f),
            RESP::None(n) => RESP::fmt_none(n, f),
        }
    }
}

impl RESP {
    pub fn as_bytes(&self) -> Vec<u8> {
        match self {
            RESP::RDB(val) => {
                let mut res = format!("${}\r\n", val.len()).as_bytes().to_vec();
                res.extend_from_slice(&val[..]);
                res
            }
            _ => self.to_string().as_bytes().to_vec(),
        }
    }

    pub fn fmt_simple_string(s: &String, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "+{s}\r\n")
    }

    pub fn fmt_bulk_string(s: &String, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "${}\r\n{s}\r\n", s.len())
    }

    pub fn fmt_simple_error(s: &String, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "-{s}\r\n")
    }

    pub fn fmt_bulk_error(s: &String, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "!{}\r\n{s}\r\n", s.len())
    }

    pub fn fmt_integer(i: &isize, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, ":{i}\r\n")
    }

    pub fn fmt_array(arr: &Vec<RESP>, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "*{}\r\n", arr.len())?;
        for a in arr {
            write!(f, "{a}")?;
        }
        Ok(())
    }

    pub fn fmt_boolean(b: &bool, f: &mut Formatter<'_>) -> std::fmt::Result {
        if *b {
            write!(f, "#t\r\n")
        } else {
            write!(f, "#f\r\n")
        }
    }
    pub fn fmt_double(d: &f64, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, ",{d}\r\n")
    }
    pub fn fmt_big_number(b: &String, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "({b}\r\n")
    }
    pub fn fmt_verbatim_string(s: &(String, String), f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "={}\r\n{}:{}\r\n", s.1.len(), s.0, s.1)
    }
    pub fn fmt_map(m: &HashMap<String, RESP>, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "%{}\r\n", m.len())?;
        for (k, v) in m {
            Self::fmt_bulk_string(k, f)?;
            write!(f, "{v}")?;
        }
        Ok(())
    }
    pub fn fmt_attributes(m: &HashMap<String, RESP>, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "|{}\r\n", m.len())?;
        for (k, v) in m {
            Self::fmt_bulk_string(k, f)?;
            write!(f, "{v}")?;
        }
        Ok(())
    }
    pub fn fmt_set(s: &HashSet<String>, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "~{}\r\n", s.len())?;
        for v in s {
            Self::fmt_bulk_string(v, f)?;
        }
        Ok(())
    }
    pub fn fmt_push(p: &Vec<RESP>, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, ">{}\r\n", p.len())?;
        for a in p {
            write!(f, "{a}")?;
        }
        Ok(())
    }

    pub fn fmt_rdb(_: &Vec<u8>, _: &mut Formatter<'_>) -> std::fmt::Result {
        panic!("do not ever call this method again!")
    }

    pub fn fmt_none(n: &TypedNone, f: &mut Formatter<'_>) -> std::fmt::Result {
        match n {
            TypedNone::String => write!(f, "$-1\r\n"),
            TypedNone::Array => write!(f, "*-1\r\n"),
            TypedNone::Nil => write!(f, "_\r\n"),
        }
    }
}
