use super::errors::syntax_error;
use super::info::Info;
use super::value::Value;
use crate::resp::parse::ReadWrite;
use crate::resp::{RESP, RESPHandler};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::net::TcpStream;
use std::ops::{AddAssign, SubAssign};
use std::sync::{Arc, Mutex};

impl ReadWrite for TcpStream {}

pub struct RedisStore {
    pub kv: HashMap<String, Value>,
    pub info: Info,
    pub expiry_queue: BTreeMap<std::time::Instant, String>,
    pub expiry_time: HashMap<String, std::time::Instant>,
}

pub type Command = VecDeque<String>;

pub struct Redis {
    pub resp: RESPHandler,
    pub store: Arc<Mutex<RedisStore>>,
    pub is_transaction: bool,
    pub transaction: Vec<Command>,
}

impl Redis {
    pub fn new(io: Box<dyn ReadWrite>, store: Arc<Mutex<RedisStore>>) -> Self {
        Redis {
            resp: RESPHandler::new(io),
            store,
            is_transaction: false,
            transaction: vec![],
        }
    }

    pub fn handle(mut self) -> std::io::Result<()> {
        self.store
            .lock()
            .unwrap()
            .info
            .connected_client
            .add_assign(1);

        loop {
            let cmd = match self.resp.next() {
                Some(v) => v,
                None => break,
            };

            if let Some(cmd) = cmd.array() {
                #[cfg(debug_assertions)]
                println!("{cmd:?}");

                let mut args = VecDeque::new();
                for c in cmd {
                    match c.string() {
                        Some(v) => args.push_back(v),
                        None => {
                            let e = RESP::SimpleError(syntax_error().to_string());
                            self.resp.send(e)?;
                            return Ok(());
                        }
                    };
                }

                match self.execute(args) {
                    Ok(res) => self.resp.send(res),
                    Err(err) => {
                        let e = RESP::SimpleError(format!("{err}"));
                        self.resp.send(e)
                    }
                }?;
            }
        }

        self.store
            .lock()
            .unwrap()
            .info
            .connected_client
            .sub_assign(1);

        Ok(())
    }
}
