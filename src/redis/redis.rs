use crate::resp::{Hashable, RESP};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

pub trait ReadWrite: Read + Write {}

impl ReadWrite for TcpStream {}

pub struct RedisStore {
    pub kv: HashMap<Hashable, RESP>,
    pub expiry: BTreeMap<std::time::Instant, Hashable>,
    pub list: HashMap<Hashable, VecDeque<RESP>>,
}

pub struct Redis {
    pub io: Box<dyn ReadWrite>,
    pub store: Arc<Mutex<RedisStore>>,
}

impl Redis {
    pub fn new(io: Box<dyn ReadWrite>, store: Arc<Mutex<RedisStore>>) -> Self {
        Redis { io, store }
    }

    pub fn handle(&mut self) -> std::io::Result<()> {
        let mut buf = vec![0; 1024];
        let mut read_bytes = 0;
        #[allow(unused)]
        let mut parsed_bytes = 0;

        loop {
            if read_bytes == buf.len() {
                let mut buuf = vec![0; buf.len() * 2];
                buuf[0..buf.len()].copy_from_slice(&buf);
                buf = buuf;
            }

            let n = self.io.read(&mut buf[read_bytes..])?;
            if n == 0 {
                return Ok(());
            }

            read_bytes += n;

            // println!("read {n} bytes");

            loop {
                let parsed = match self.parse(&buf[..read_bytes]) {
                    Ok(v) => v,
                    Err(_) => break,
                };

                // println!("parsed {parsed} bytes");

                read_bytes -= parsed;

                let v = buf[parsed..read_bytes + parsed].to_vec();
                buf[0..read_bytes].copy_from_slice(&v);

                parsed_bytes += parsed;
            }
        }
    }

    fn parse(&mut self, data: &[u8]) -> std::io::Result<usize> {
        if let Some((parsed, cmd)) = RESP::parse(data) {
            if let Some(cmd) = cmd.array() {
                println!("{cmd:?}");
                if let Some(err) = self.execute(cmd).err() {
                    let e = RESP::SimpleError(format!("{err}"));
                    write!(self.io, "{e}")?;
                }
            } else {
                // self.io.write("+PONG\r\n".as_bytes())?;
            }
            Ok(parsed)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "cannot parse data",
            ))
        }
    }
}
