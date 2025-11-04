use super::utils::DevNull;
use crate::redis::{Redis, RedisStore, syntax_error};
use crate::resp::{RESP, RESPHandler};
use std::collections::VecDeque;
use std::io::pipe;
use std::net::{Ipv4Addr, SocketAddrV4, TcpStream};
use std::sync::{Arc, Mutex};

pub struct Slave {
    pub master: (Ipv4Addr, u16),
    pub io: RESPHandler,
    pub store: Arc<Mutex<RedisStore>>,
}

impl Slave {
    pub fn new(addr: Ipv4Addr, port: u16, store: Arc<Mutex<RedisStore>>) -> std::io::Result<Self> {
        let tcp = TcpStream::connect(SocketAddrV4::new(addr, port))?;
        Ok(Slave {
            master: (addr, port),
            io: RESPHandler::new(Box::new(tcp)),
            store,
        })
    }

    pub fn handle(&mut self) -> std::io::Result<()> {
        self.handshake()
    }

    pub fn handshake(&mut self) -> std::io::Result<()> {
        self.ping()?;
        self.replconf()?;
        self.psync()?;
        Ok(())
    }

    pub fn ping(&mut self) -> std::io::Result<()> {
        let ping: RESP = RESP::from(vec!["PING"]);
        self.io.send(ping)?;
        let response = self.io.next().unwrap().string().unwrap();
        assert_eq!(response.to_lowercase(), "pong");
        Ok(())
    }

    pub fn replconf(&mut self) -> std::io::Result<()> {
        let first_message = RESP::from(vec![
            "REPLCONF".into(),
            "listening-port".into(),
            self.store.lock().unwrap().info.listening_port.to_string(),
        ]);
        self.io.send(first_message)?;
        let response = self.io.next().unwrap().string().unwrap();
        assert_eq!(response.to_lowercase(), "ok");

        let second_message: RESP = RESP::from(vec!["REPLCONF", "capa", "psync2"]);
        self.io.send(second_message)?;
        let response = self.io.next().unwrap().string().unwrap();
        assert_eq!(response.to_lowercase(), "ok");

        Ok(())
    }

    pub fn psync(&mut self) -> std::io::Result<()> {
        let repl_id = self.store.lock().unwrap().info.master_id.clone();
        let offset = self.store.lock().unwrap().info.offset;

        let message = RESP::from(vec!["PSYNC".into(), repl_id, offset.to_string()]);
        self.io.send(message)?;

        let response = self.io.next().unwrap();
        #[cfg(debug_assertions)]
        print!("psync-response: {response}");

        let rdb_file = self.io.next_rdb();
        #[cfg(debug_assertions)]
        println!("rdb_file: {rdb_file:?}");

        let mut redis = Redis::new(Box::new(DevNull), self.store.clone());

        self.io.parsed_bytes = 0;

        loop {
            let cmd = match self.io.next() {
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
                            // redis.resp.send(e)?;
                            return Ok(());
                        }
                    };
                }

                match redis.execute(args) {
                    Ok(res) => {
                        if let RESP::Push(v) = res {
                            self.io.send(v.into())?;
                        }
                        // redis.resp.send(res)
                    }
                    Err(err) => {
                        // let e = RESP::SimpleError(format!("{err}"));
                        // redis.resp.send(e)
                    }
                };
            }

            self.store.lock().unwrap().info.offset = self.io.parsed_bytes as isize;
        }
        // assert_eq!(response.to_lowercase(), "ok");
        Ok(())
    }
}
