use super::Args;
use super::errors::*;
use crate::Error;
use crate::frame::{
    encode::AsBytes,
    {Frame, TypedNone},
};
use crate::parser::Parser;
use crate::store::Store;
use bytes::BytesMut;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::{Mutex, mpsc};

pub struct Server {
    pub slave_id: usize,
    pub subscription_count: usize,
    pub store: Arc<Mutex<Store>>,
    pub output: mpsc::Sender<Frame>,
    pub transaction: VecDeque<Args>,
    pub slave_config: Option<SlaveConfig>,
    pub(crate) in_transaction: bool,
}

pub struct SlaveConfig {
    pub port: u16,
    pub capabilities: Vec<String>,
}

impl Server {
    pub fn new(store: Arc<Mutex<Store>>, output: mpsc::Sender<Frame>, slave_id: usize) -> Self {
        Server {
            slave_id,
            store,
            output,
            transaction: VecDeque::new(),
            subscription_count: 0,
            in_transaction: false,
            slave_config: None,
        }
    }

    pub async fn handle(
        store: Arc<Mutex<Store>>,
        stream: TcpStream,
        buffer: BytesMut,
        slave_id: usize,
    ) {
        let (reader, mut writer) = stream.into_split();
        let (tx, mut rx): (mpsc::Sender<Frame>, mpsc::Receiver<Frame>) = mpsc::channel(64);

        tokio::spawn(async move {
            loop {
                if let Some(v) = rx.recv().await {
                    // println!("sending {v:?} to connection");
                    let mut b = BytesMut::new();
                    v.encode_bytes(&mut b);
                    if writer.write_all(b.freeze().as_ref()).await.is_err() {
                        break;
                    }
                } else {
                    break;
                }
            }
        });

        tokio::time::sleep(Duration::from_millis(1)).await;

        tokio::spawn(async move {
            let parser = Parser::new(Box::new(reader), buffer);
            let mut server = Server::new(store, tx, slave_id);
            server.execution_thread(parser).await
        });
    }

    pub async fn execution_thread(&mut self, mut parser: Parser) -> Result<(), Error> {
        loop {
            let command = match parser.read_frame().await? {
                Some(v) => v,
                None => break,
            };

            let method = command
                .clone()
                .array()
                .ok_or("invalid command format!")?
                .remove(0)
                .string()
                .unwrap_or("ping".into());
            
            if self.subscription_count > 0 && !subscriber_mode_command(&method) {
                let resp = Frame::SimpleError(format!("ERR Can't execute '{method}': only (P|S)SUBSCRIBE / (P|S)UNSUBSCRIBE / PING / QUIT / RESET are allowed in this context").into());
                self.output.send(resp).await?;
                continue
            }
            
            if self.slave_id == 0 {
                if is_write_command(&method) {
                    let _ = self
                        .store
                        .lock()
                        .await
                        .broadcast
                        .clone()
                        .expect("broadcast not set properly")
                        .send(command.clone());

                    println!("increasing send_offset in write command");
                    self.store.lock().await.info.send_offset += parser.parsed_bytes;
                }
            }

            let args: Option<VecDeque<String>> = command
                .array()
                .ok_or("invalid command format!")?
                .into_iter()
                .map(|v| v.string())
                .collect();
            let args = args.ok_or("invalid command format!")?;

            #[cfg(debug_assertions)]
            println!("command: {args:?}");
            
            let mut response = if self.in_transaction {
                self.transaction(args).await
            } else {
                self.execute(args).await
            };

            self.store.lock().await.info.recv_offset += parser.parsed_bytes;
            
            if self.subscription_count > 0 {
                response = response.map(|r| {
                    if r.is_array() {
                        r 
                    } else {
                        let pong: Frame = "pong".to_string().into();
                        vec![pong, "".to_string().into()].into()
                    }
                });
            } 
            
            if self.slave_id == 0 {
                let _ = match response {
                    Ok(v) => self.output.send(v).await,
                    Err(e) => {
                        let resp = Frame::SimpleError(format!("{e}"));
                        self.output.send(resp).await
                    }
                };
            }
        }
        Ok(())
    }

    pub async fn execute(&mut self, mut args: Args) -> Result<Frame, Error> {
        let method = args.pop_front().ok_or(syntax_error())?;
        match method.to_lowercase().as_str() {
            // Ping pong commands
            "ping" => self.ping(args),
            "echo" => self.echo(args),
            "info" => self.info(args).await,
            "type" => self.redis_type(args).await,
            // string operations
            "set" => self.set(args).await,
            "get" => self.get(args).await,
            "incr" => self.incr(args).await,
            // list operations
            "rpush" => self.rpush(args).await,
            "lpush" => self.lpush(args).await,
            "lpop" => self.lpop(args).await,
            "blpop" => self.blpop(args).await,
            "lrange" => self.lrange(args).await,
            "llen" => self.llen(args).await,
            // stream operations
            "xadd" => self.xadd(args).await,
            "xdel" => self.xdel(args).await,
            "xlen" => self.xlen(args).await,
            "xrange" => self.xrange(args).await,
            "xread" => self.xread(args).await,
            // transaction operations
            "multi" => self.multi(args),
            "exec" => Err(make_io_error("ERR EXEC without MULTI").into()),
            "discard" => Err(make_io_error("ERR DISCARD without MULTI").into()),
            // replication operations
            "replconf" => self.replconf(args).await,
            "psync" => self.psync(args).await,
            "wait" => self.wait(args).await,
            // config
            "config" => self.config(args).await,
            "keys" => self.keys(args).await,
            // pubsub
            "subscribe" => self.subscribe(args).await,
            "unsubscribe" => self.unsubscribe(args).await,
            _ => self.invalid(args),
        }
    }

    /// Returns the string representation of the type of the value stored at key.
    /// The different types that can be returned are:
    /// string, list, set, zset, hash, stream, and vectorset.
    /// ```
    /// TYPE key
    /// ```
    async fn redis_type(&mut self, mut args: Args) -> Result<Frame, Error> {
        let key = args.pop_front().ok_or(wrong_num_arguments("type"))?;
        let store = self.store.lock().await;
        let resp = store
            .kv
            .get(&key)
            .map(|v| v.redis_type())
            .unwrap_or("none".into())
            .as_str()
            .into();
        Ok(resp)
    }

    /// The INFO command returns information and statistics about the server in a format
    /// that is simple to parse by computers and easy to read by humans.
    /// ```
    /// INFO [section [section ...]]
    /// ```
    pub async fn info(&mut self, _: Args) -> Result<Frame, Error> {
        Ok(Frame::BulkString(
            self.store.lock().await.info.to_string().into(),
        ))
    }

    /// Returns message.
    /// ```
    /// ECHO message
    /// ```
    fn echo(&mut self, mut args: Args) -> Result<Frame, Error> {
        Ok(args.pop_front().ok_or(wrong_num_arguments("echo"))?.into())
    }

    /// Returns PONG if no argument is provided, otherwise return a copy of the argument as a bulk.
    /// ```
    /// PING [message]
    /// ```
    fn ping(&mut self, _: Args) -> Result<Frame, Error> {
        Ok("PONG".into())
    }

    fn invalid(&mut self, _: Args) -> Result<Frame, Error> {
        Ok(Frame::None(TypedNone::Nil))
    }
}

fn is_write_command(cmd: &str) -> bool {
    match cmd.to_lowercase().as_str() {
        "set" | "del" => true,
        _ => false,
    }
}

fn subscriber_mode_command(cmd: &str) -> bool {
    match cmd.to_lowercase().as_str() {
        "subscribe" | "unsubscribe" | "psubscribe" | "punsubscribe" | "ping" | "quit" => true,
        _ => false,
    }
}