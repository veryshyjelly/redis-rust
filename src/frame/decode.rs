use super::{Error, Frame, TypedNone};
use bytes::{Buf, Bytes};
use std::collections::{HashMap, HashSet};
use std::io::Cursor;
use std::num::{ParseFloatError, TryFromIntError};
use std::string::FromUtf8Error;

impl Frame {
    pub fn parse(src: &mut Cursor<&[u8]>) -> Result<Frame, Error> {
        let err = || -> Error { "protocol error; invalid format".into() };
        match get_u8(src)? {
            b'+' => {
                let line = get_line(src)?.to_vec();
                let string = String::from_utf8(line)?;
                Ok(Frame::SimpleString(string))
            }
            b'-' => {
                let line = get_line(src)?.to_vec();
                let string = String::from_utf8(line)?;
                Ok(Frame::SimpleError(string))
            }
            b':' => Ok(Frame::Integer(get_decimal(src)?)),
            b'$' => {
                let bulk_data = get_bulk(src)?;
                match bulk_data {
                    Some(v) => Ok(Frame::BulkString(v)),
                    None => Ok(Frame::None(TypedNone::String)),
                }
            }
            b'*' => {
                if b'-' == peek_u8(src)? {
                    let line = get_line(src)?;
                    if line != b"-1" {
                        return Err("protocol error; invalid frame format".into());
                    }
                    Ok(Frame::None(TypedNone::Array))
                } else {
                    let len = get_decimal(src)?.try_into()?;
                    let mut out = Vec::with_capacity(len);
                    for _ in 0..len {
                        out.push(Frame::parse(src)?);
                    }
                    Ok(Frame::Array(out))
                }
            }
            b'_' => {
                skip(src, 2)?;
                Ok(Frame::None(TypedNone::Nil))
            }
            b'#' => {
                let line = get_line(src)?;
                Ok(Frame::Boolean(line[0] == b't'))
            }
            b',' => {
                let line = get_line(src)?.to_vec();
                let res: f64 = String::from_utf8(line)?
                    .parse()
                    .map_err(|e| Error::from(e))?;
                Ok(Frame::Double(res))
            }
            b'(' => {
                let line = get_line(src)?.to_vec();
                let string = String::from_utf8(line)?;
                Ok(Frame::BigNumber(string))
            }
            b'!' => {
                let bulk_data = get_bulk(src)?.ok_or(err())?;
                Ok(Frame::BulkError(bulk_data))
            }
            b'=' => {
                unimplemented!()
            }
            b'%' => {
                let count = get_decimal(src)? as usize;
                let mut res = HashMap::new();
                for _ in 0..count {
                    let key = Frame::parse(src)?.string().ok_or(err())?;
                    let value = Frame::parse(src)?;
                    res.insert(key, value);
                }
                Ok(Frame::Map(res))
            }
            b'|' => {
                let count = get_decimal(src)? as usize;
                let mut res = HashMap::new();
                for _ in 0..count {
                    let key = Frame::parse(src)?.string().ok_or(err())?;
                    let value = Frame::parse(src)?;
                    res.insert(key, value);
                }
                Ok(Frame::Attributes(res))
            }
            b'~' => {
                let count = get_decimal(src)? as usize;
                let mut res = HashSet::new();
                for _ in 0..count {
                    let key = Frame::parse(src)?.string().ok_or(err())?;
                    res.insert(key);
                }
                Ok(Frame::Set(res))
            }
            b'>' => {
                if b'-' == peek_u8(src)? {
                    let line = get_line(src)?;
                    if line != b"-1" {
                        return Err("protocol error; invalid frame format".into());
                    }
                    Ok(Frame::None(TypedNone::Array))
                } else {
                    let len = get_decimal(src)?.try_into()?;
                    let mut out = Vec::with_capacity(len);
                    for _ in 0..len {
                        out.push(Frame::parse(src)?);
                    }
                    Ok(Frame::Push(out))
                }
            }
            _ => unimplemented!(),
        }
    }
}

fn get_bulk(src: &mut Cursor<&[u8]>) -> Result<Option<Bytes>, Error> {
    if b'-' == peek_u8(src)? {
        let line = get_line(src)?;

        if line != b"-1" {
            return Err("protocol error; invalid frame format".into());
        }
        Ok(None)
    } else {
        // Read the bulk string
        let len = get_decimal(src)?.try_into()?;
        let n = len + 2;

        if src.remaining() < n {
            return Err(Error::Incomplete);
        }

        let data = Bytes::copy_from_slice(&src.chunk()[..len]);
        skip(src, n)?;
        Ok(Some(data))
    }
}

fn peek_u8(src: &mut Cursor<&[u8]>) -> Result<u8, Error> {
    if !src.has_remaining() {
        return Err(Error::Incomplete);
    }

    Ok(src.chunk()[0])
}

fn get_u8(src: &mut Cursor<&[u8]>) -> Result<u8, Error> {
    if !src.has_remaining() {
        return Err(Error::Incomplete);
    }

    Ok(src.get_u8())
}

fn skip(src: &mut Cursor<&[u8]>, n: usize) -> Result<(), Error> {
    if src.remaining() < n {
        return Err(Error::Incomplete);
    }

    src.advance(n);
    Ok(())
}

pub fn get_decimal(src: &mut Cursor<&[u8]>) -> Result<isize, Error> {
    let line = get_line(src)?;
    atoi::atoi(line).ok_or_else(|| "protocol error; invalid frame format".into())
}

fn get_line<'a>(src: &mut Cursor<&'a [u8]>) -> Result<&'a [u8], Error> {
    let start = src.position() as usize;
    // Scan to the second to last byte
    let end = src.get_ref().len() - 1;

    for i in start..end {
        if src.get_ref()[i] == b'\r' && src.get_ref()[i + 1] == b'\n' {
            // We found a line, update the position to be *after* the \n
            src.set_position((i + 2) as u64);

            // Return the line
            return Ok(&src.get_ref()[start..i]);
        }
    }
    Err(Error::Incomplete)
}

impl From<String> for Error {
    fn from(src: String) -> Error {
        Error::Other(src.into())
    }
}

impl From<&str> for Error {
    fn from(src: &str) -> Error {
        src.to_string().into()
    }
}

macro_rules! impl_error {
    ($tp:ty) => {
        impl From<$tp> for Error {
            fn from(_: $tp) -> Error {
                "protocol error; invalid frame format".into()
            }
        }
    };
}

impl_error!(FromUtf8Error);
impl_error!(ParseFloatError);
impl_error!(TryFromIntError);

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Incomplete => "stream ended early".fmt(fmt),
            Error::Other(err) => err.fmt(fmt),
        }
    }
}
