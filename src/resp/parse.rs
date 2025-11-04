use super::resp::Result;
use super::resp::{RESP, TypedNone};
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};

pub struct RESPHandler {
    io: Box<dyn ReadWrite>,
    read_bytes: usize,
    buf: Vec<u8>,
}

pub trait ReadWrite: Read + Write {}

impl RESPHandler {
    pub fn new(io: Box<dyn ReadWrite>) -> Self {
        RESPHandler {
            io,
            read_bytes: 0,
            buf: vec![0; 1024],
        }
    }

    pub fn send(&mut self, val: RESP) -> std::io::Result<()> {
        write!(self.io, "{val}")
    }
}

impl Iterator for RESPHandler {
    type Item = RESP;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.buf.len() > 16 * 1024 * 1024 {
                eprintln!("RESPParser: buffer too large, aborting");
                return None;
            }

            if let Some((parsed, cmd)) = RESP::parse(&self.buf[..self.read_bytes]) {
                self.buf.copy_within(parsed..self.read_bytes, 0);
                self.read_bytes -= parsed;
                return Some(cmd);
            }

            if self.read_bytes == self.buf.len() {
                self.buf.resize(self.buf.len() * 2, 0);
            }

            let n = match self.io.read(&mut self.buf[self.read_bytes..]) {
                Ok(0) => return None, // EOF
                Ok(n) => n,
                Err(_) => return None, // TODO: maybe handle differently
            };

            self.read_bytes += n;
        }
    }
}

impl RESP {
    pub fn parse(mut data: &[u8]) -> Result {
        if data.is_empty() {
            return None;
        }

        let format = data[0] as char;
        data = &data[1..];
        let (l, r) = match format {
            '+' => Self::parse_simple_string(data),
            '-' => Self::parse_simple_error(data),
            ':' => Self::parse_integer(data),
            '$' => Self::parse_bulk_string(data),
            '*' => Self::parse_array(data),
            '_' => Self::parse_null(data),
            '#' => Self::parse_boolean(data),
            ',' => Self::parse_double(data),
            '(' => Self::parse_big_number(data),
            '!' => Self::parse_bulk_error(data),
            '=' => Self::parse_verbatim_string(data),
            '%' => Self::parse_map(data),
            '|' => Self::parse_attribute(data),
            '~' => Self::parse_set(data),
            '>' => Self::parse_push(data),
            _ => None,
        }?;
        Some((l + 1, r))
    }

    fn parse_simple_string(data: &[u8]) -> Result {
        if let Some(idx) = find_crlf(data) {
            let res = RESP::SimpleString(String::from_utf8_lossy(&data[..idx]).to_string());
            return Some((idx + 2, res));
        }
        None
    }

    fn parse_simple_error(data: &[u8]) -> Result {
        let (parsed, s) = Self::parse_simple_string(data)?;
        Some((parsed, RESP::SimpleError(s.string()?)))
    }

    fn parse_integer(data: &[u8]) -> Result {
        let (parsed, s) = Self::parse_simple_string(data)?;
        let s = s.string()?;
        Some((parsed, RESP::Integer(s.parse().ok()?)))
    }

    fn parse_bulk_string(data: &[u8]) -> Result {
        let (mut parsed, length) = Self::parse_integer(data)?;
        let length = length.int()?;
        if length == -1 {
            return Some((parsed, RESP::None(TypedNone::String)));
        }
        if data[parsed..].len() < length as usize + 2 {
            return None;
        }
        let (n, string) = Self::parse_simple_string(&data[parsed..])?;
        parsed += n;

        Some((parsed, RESP::BulkString(string.string()?)))
    }

    fn parse_array(data: &[u8]) -> Result {
        let (mut parsed, n) = Self::parse_integer(data)?;
        let n = n.int()?;
        if n == -1 {
            return Some((parsed, RESP::None(TypedNone::Array)));
        }

        let mut res = vec![];

        for _ in 0..n {
            let (p, r) = Self::parse(&data[parsed..])?;
            parsed += p;
            res.push(r);
        }

        Some((parsed, RESP::Array(res)))
    }

    fn parse_null(data: &[u8]) -> Result {
        if let Some(idx) = find_crlf(data) {
            return Some((idx + 2, RESP::None(TypedNone::Nil)));
        }
        None
    }

    fn parse_boolean(data: &[u8]) -> Result {
        if let Some(idx) = find_crlf(data) {
            return Some((idx + 2, RESP::Boolean(data[0] as char == 't')));
        }
        None
    }

    fn parse_double(data: &[u8]) -> Result {
        let (parsed, s) = Self::parse_simple_string(data)?;
        let s = s.string()?;
        Some((parsed, RESP::Double(s.parse().ok()?)))
    }

    fn parse_big_number(data: &[u8]) -> Result {
        let (parsed, s) = Self::parse_simple_string(data)?;
        Some((parsed, RESP::BigNumber(s.string()?)))
    }

    fn parse_bulk_error(data: &[u8]) -> Result {
        let (parsed, s) = Self::parse_bulk_string(data)?;
        Some((parsed, RESP::BulkError(s.string()?)))
    }

    fn parse_verbatim_string(_data: &[u8]) -> Result {
        todo!()
    }

    fn parse_map(data: &[u8]) -> Result {
        let (mut parsed, count) = Self::parse_integer(data)?;
        let count = count.int()? as usize;
        let mut res = HashMap::new();

        for _ in 1..count {
            let (p, key) = Self::parse(&data[parsed..])?;
            parsed += p;
            let (p, value) = Self::parse(&data[parsed..])?;
            parsed += p;
            res.insert(key.string()?, value);
        }

        Some((parsed, RESP::Map(res)))
    }

    fn parse_attribute(data: &[u8]) -> Result {
        let (parsed, res) = Self::parse_map(data)?;
        Some((parsed, RESP::Attributes(res.map()?)))
    }

    fn parse_set(data: &[u8]) -> Result {
        let (mut parsed, count) = Self::parse_integer(data)?;
        let count = count.int()? as usize;
        let mut res = HashSet::new();

        for _ in 1..count {
            let (p, value) = Self::parse(&data[parsed..])?;
            parsed += p;
            res.insert(value.string()?);
        }

        Some((parsed, RESP::Set(res)))
    }

    fn parse_push(data: &[u8]) -> Result {
        let (parsed, res) = Self::parse_array(data)?;
        Some((parsed, RESP::Push(res.array()?)))
    }
}

fn find_crlf(data: &[u8]) -> Option<usize> {
    data.windows(2).position(|w| w.eq("\r\n".as_bytes()))
}
