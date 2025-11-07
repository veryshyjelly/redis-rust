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
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::{Mutex, mpsc};

pub struct Server {
    pub is_slave: bool,
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
    pub fn new(store: Arc<Mutex<Store>>, output: mpsc::Sender<Frame>, is_slave: bool) -> Self {
        Server {
            is_slave,
            store,
            output,
            transaction: VecDeque::new(),
            in_transaction: false,
            slave_config: None,
        }
    }

    pub async fn handle(stream: TcpStream, store: Arc<Mutex<Store>>, is_slave: bool) {
        let (reader, mut writer) = stream.into_split();
        let (tx, mut rx): (mpsc::Sender<Frame>, mpsc::Receiver<Frame>) = mpsc::channel(64);
        tokio::spawn(async move {
            let parser = Parser::new(Box::new(reader));
            let mut server = Server::new(store, tx, is_slave);
            server.execution_thread(parser).await
        });
        tokio::spawn(async move {
            loop {
                if let Some(v) = rx.recv().await {
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
    }

    pub async fn execution_thread(&mut self, mut parser: Parser) -> Result<(), Error> {
        loop {
            let command = match parser.read_frame().await? {
                Some(v) => v,
                None => break,
            };

            let args: Option<VecDeque<String>> = command
                .array()
                .ok_or("invalid command format!")?
                .into_iter()
                .map(|v| v.string())
                .collect();
            let args = args.ok_or("invalid command format!")?;

            let response = if self.in_transaction {
                self.transaction(args).await
            } else {
                self.execute(args).await
            };

            if !self.is_slave {
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
        match method.as_str() {
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
