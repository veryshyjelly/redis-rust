use super::Args;
use super::errors::*;
use crate::Error;
use crate::frame::{Frame, encode::AsBytes};
use crate::parser::Parser;
use crate::store::Store;
use bytes::BytesMut;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::{Mutex, mpsc, oneshot};

pub struct Server {
    pub(crate) slave_id: usize,
    pub(crate) subscription_count: usize,
    pub(crate) store: Arc<Mutex<Store>>,
    pub(crate) output: mpsc::Sender<Frame>,
    pub(crate) transaction: VecDeque<Args>,
    pub(crate) unsubscribe: HashMap<String, oneshot::Sender<bool>>,
    pub(crate) slave_config: Option<SlaveConfig>,
    pub(crate) in_transaction: bool,
}

pub struct SlaveConfig {
    pub port: u16,
    pub capabilities: Vec<String>,
}

macro_rules! dispatch {
    ($self:ident, $method:expr, $args:expr, {
        $($cmd:ident),* $(,)?
        ; $($custom:literal => $expr:expr),* $(,)?
    }) => {{
        match $method.to_lowercase().as_str() {
            $(
                stringify!($cmd) => $self.$cmd($args).await,
            )*
            $(
                $custom => $expr,
            )*
            _ => $self.invalid($args).await,
        }
    }};
}

impl Server {
    pub fn new(store: Arc<Mutex<Store>>, output: mpsc::Sender<Frame>, slave_id: usize) -> Self {
        Server {
            slave_id,
            store,
            output,
            transaction: VecDeque::new(),
            unsubscribe: HashMap::new(),
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

    async fn execution_thread(&mut self, mut parser: Parser) -> Result<(), Error> {
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
                continue;
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

    pub(crate) async fn execute(&mut self, mut args: Args) -> Result<Frame, Error> {
        let method = args.pop_front().ok_or(syntax_error())?;
        dispatch!(self, method, args, {
            // Ping pong commands
            ping, echo, info,
            // string operations
            set, get, incr,
            // list operations
            rpush, lpush, lpop, blpop, lrange, llen,
            // sream operations
            xadd, xdel, xlen, xrange, xread,
            // transaction operations
            multi,
            // replication operations
            replconf, psync, wait,
            // config
            config, keys,
            // pubsub
            subscribe, unsubscribe, publish,
            // zset
            zadd, zcard, zcount, zrank, zrange, zrem, zscore ;
            "type" => self.redis_type(args).await,
            "exec" => Err(make_io_error("ERR EXEC without MULTI").into()),
            "discard" => Err(make_io_error("ERR DISCARD without MULTI").into()),
        })
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
