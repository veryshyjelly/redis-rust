use crate::frame::{Frame, TypedNone};
use bytes::{BufMut, Bytes, BytesMut};
use std::collections::{HashMap, HashSet};

pub trait AsBytes {
    fn encode_bytes(&self, b: &mut BytesMut);
}

impl AsBytes for Frame {
    fn encode_bytes(&self, mut b: &mut BytesMut) {
        use Frame::*;
        match self {
            SimpleString(s) => Frame::encode_simple_string(s, &mut b),
            BulkString(s) => Frame::encode_bulk_string(s, &mut b),
            SimpleError(e) => Frame::encode_simple_error(e, &mut b),
            BulkError(e) => Frame::encode_bulk_error(e, &mut b),
            Integer(i) => Frame::encode_integer(i, &mut b),
            Array(a) => Frame::encode_array(a, &mut b),
            Boolean(bo) => Frame::encode_boolean(bo, &mut b),
            Double(d) => Frame::encode_double(d, &mut b),
            BigNumber(n) => Frame::encode_big_number(n, &mut b),
            VerbatimString(v) => Frame::encode_verbatim_string(v, &mut b),
            Map(m) => Frame::encode_map(m, &mut b),
            Attributes(a) => Frame::encode_attributes(a, &mut b),
            Set(s) => Frame::encode_set(s, &mut b),
            Push(p) => Frame::encode_push(p, &mut b),
            RDB(v) => Frame::encode_rdb(v, &mut b),
            None(n) => Frame::encode_none(n, &mut b),
        }
    }
}

impl Frame {
    fn encode_simple_string(s: &String, b: &mut BytesMut) {
        b.put_u8(b'+');
        b.put_slice(s.as_bytes());
        b.put_slice(b"\r\n");
    }
    fn encode_bulk_string(s: &Bytes, b: &mut BytesMut) {
        b.put_u8(b'$');
        b.put_slice(s.len().to_string().as_bytes());
        b.put_slice(b"\r\n");
        b.extend(s);
        b.put_slice(b"\r\n");
    }
    fn encode_simple_error(e: &String, b: &mut BytesMut) {
        b.put_u8(b'-');
        b.put_slice(e.as_bytes());
        b.put_slice(b"\r\n");
    }
    fn encode_bulk_error(e: &Bytes, b: &mut BytesMut) {
        b.put_u8(b'!');
        b.put_slice(e.len().to_string().as_bytes());
        b.put_slice(b"\r\n");
        b.extend(e);
        b.put_slice(b"\r\n");
    }
    fn encode_integer(i: &isize, b: &mut BytesMut) {
        b.put_u8(b':');
        b.put_slice(i.to_string().as_bytes());
        b.put_slice(b"\r\n");
    }
    fn encode_array(a: &Vec<Frame>, b: &mut BytesMut) {
        b.put_u8(b'*');
        b.put_slice(a.len().to_string().as_bytes());
        b.put_slice(b"\r\n");
        for ai in a {
            ai.encode_bytes(b);
        }
    }
    fn encode_boolean(bo: &bool, b: &mut BytesMut) {
        b.put_u8(b'#');
        if *bo {
            b.put_slice(b"t\r\n")
        } else {
            b.put_slice(b"f\r\n")
        }
    }
    fn encode_double(d: &f64, b: &mut BytesMut) {
        b.put_u8(b',');
        b.put_slice(d.to_string().as_bytes());
        b.put_slice(b"\r\n");
    }
    fn encode_big_number(n: &String, b: &mut BytesMut) {
        b.put_u8(b'(');
        b.put_slice(n.as_bytes());
        b.put_slice(b"\r\n");
    }
    fn encode_verbatim_string(s: &(String, String), b: &mut BytesMut) {
        b.put_slice(format!("={}\r\n{}:{}\r\n", s.1.len(), s.0, s.1).as_bytes())
    }
    fn encode_map(m: &HashMap<String, Frame>, b: &mut BytesMut) {
        b.put_u8(b'%');
        b.put_slice(m.len().to_string().as_bytes());
        b.put_slice(b"\r\n");
        for (k, v) in m {
            Frame::encode_bulk_string(&Bytes::from(k.clone()), b);
            v.encode_bytes(b);
        }
    }

    fn encode_attributes(m: &HashMap<String, Frame>, b: &mut BytesMut) {
        b.put_u8(b'|');
        b.put_slice(m.len().to_string().as_bytes());
        b.put_slice(b"\r\n");
        for (k, v) in m {
            Frame::encode_bulk_string(&Bytes::from(k.clone()), b);
            v.encode_bytes(b);
        }
    }

    fn encode_set(s: &HashSet<String>, b: &mut BytesMut) {
        b.put_u8(b'%');
        b.put_slice(s.len().to_string().as_bytes());
        b.put_slice(b"\r\n");
        for v in s {
            Frame::encode_bulk_string(&Bytes::from(v.clone()), b);
        }
    }

    fn encode_push(p: &Vec<Frame>, b: &mut BytesMut) {
        b.put_u8(b'>');
        b.put_slice(p.len().to_string().as_bytes());
        b.put_slice(b"\r\n");
        for ai in p {
            ai.encode_bytes(b);
        }
    }

    fn encode_rdb(v: &Bytes, b: &mut BytesMut) {
        b.put_u8(b'$');
        b.put_slice(v.len().to_string().as_bytes());
        b.put_slice(b"\r\n");
        b.extend(v);
    }

    fn encode_none(n: &TypedNone, b: &mut BytesMut) {
        match n {
            TypedNone::String => b.put_slice(b"$-1\r\n"),
            TypedNone::Array => b.put_slice(b"*-1\r\n"),
            TypedNone::Nil => b.put_slice(b"_\r\n"),
        }
    }
}
