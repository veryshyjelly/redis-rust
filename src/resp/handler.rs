use super::resp::RESP;
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
