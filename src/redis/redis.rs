use super::value::Value;
use crate::resp::{RESP};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::ops::{Add, AddAssign, SubAssign};
use std::sync::{Arc, Mutex};
use super::errors::syntax_error;
use super::info::Info;

pub trait ReadWrite: Read + Write {}

impl ReadWrite for TcpStream {}

pub struct RedisStore {
    pub info: Info,
    pub kv: HashMap<String, Value>,
    pub expiry_queue: BTreeMap<std::time::Instant, String>,
    pub expiry_time: HashMap<String, std::time::Instant>,
}

pub type Command = VecDeque<String>;

pub struct Redis {
    pub io: Box<dyn ReadWrite>,
    pub store: Arc<Mutex<RedisStore>>,
    pub is_transaction: bool,
    pub transaction: Vec<Command>
}

impl Redis {
    pub fn new(io: Box<dyn ReadWrite>, store: Arc<Mutex<RedisStore>>) -> Self {
        Redis { io, store, is_transaction: false, transaction: vec![] }
    }

    pub fn handle(&mut self) {
        self.store.lock().unwrap().info.connected_client.add_assign(1);
        
        let mut buf = vec![0; 1024];
        let mut read_bytes = 0;
        #[allow(unused)]
        let mut parsed_bytes = 0;

        loop {
            if read_bytes == buf.len() {
                let mut buf2 = vec![0; buf.len() * 2];
                buf2[0..buf.len()].copy_from_slice(&buf);
                buf = buf2;
            }

            let n = self.io.read(&mut buf[read_bytes..]).unwrap_or(0);
            if n == 0 {
                self.store.lock().unwrap().info.connected_client.sub_assign(1);
                return;
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
                #[cfg(debug_assertions)]
                println!("{cmd:?}");
                
                let mut args = VecDeque::new();
                for c in cmd {
                    match c.string() {
                        Some(v) => args.push_back(v),
                        None => {
                            let e = RESP::SimpleError(syntax_error().to_string());
                            write!(self.io, "{e}")?;
                            return Ok(parsed);
                        }
                    };
                }
                
                match self.execute(args) {
                    Ok(resp) => write!(self.io, "{resp}")?,
                    Err(err) => {
                        let e = RESP::SimpleError(format!("{err}"));
                        write!(self.io, "{e}")?;
                    }
                };
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
