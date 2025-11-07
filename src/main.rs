use crate::server::server::Server;
use crate::store::{Info, Role, Store};
use bytes::BytesMut;
use rand::Rng;
use rand::distr::Alphanumeric;
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::str::FromStr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

mod frame;
mod parser;
mod server;
mod slave;
mod store;

pub type Error = Box<dyn std::error::Error + Send + Sync>;

fn get_arg_value(args: &[String], key: &str) -> Option<String> {
    args.iter()
        .position(|v| v == key)
        .and_then(|idx| args.get(idx + 1))
        .cloned()
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args: Vec<_> = std::env::args().collect();
    
    let mut info = Info::default();
    let port = get_arg_value(&args, "--port")
        .and_then(|v| v.parse().ok())
        .unwrap_or(6379);
    info.listening_port = port;
    info.dir = get_arg_value(&args, "--dir").unwrap_or_default();
    info.db_filename = get_arg_value(&args, "--dbfilename").unwrap_or_default();

    let redis_store = Arc::new(Mutex::new(Store {
        kv: Default::default(),
        expiry_queue: Default::default(),
        expiry_time: Default::default(),
        info,
        broadcast: None,
        get_ack_channel: None,
        slave_offsets: HashMap::new(),
        slave_asked_offsets: HashMap::new(),
    }));

    if let Some(idx) = args.iter().position(|v| v == "--replicaof") {
        let mut addr = args[idx + 1].split(" ");
        let ip_str = addr.next().unwrap();
        let ip = if ip_str == "localhost" {
            Ipv4Addr::from_str("127.0.0.1").unwrap()
        } else {
            Ipv4Addr::from_str(ip_str).unwrap()
        };
        let port: u16 = addr.next().unwrap().parse().unwrap();
        redis_store.lock().await.info.master_id = "?".into();
        redis_store.lock().await.info.role = Role::Slave;
        let store = redis_store.clone();
        tokio::spawn(async move {
            slave::handle(ip, port, store).await.unwrap();
        });
    } else {
        let (tx, _rx) = tokio::sync::broadcast::channel(64);
        drop(_rx);
        let _ = redis_store.lock().await.broadcast.insert(tx);
        let (tx, _rx) = tokio::sync::broadcast::channel(64);
        drop(_rx);
        let _ = redis_store.lock().await.get_ack_channel.insert(tx);
        let master_id: String = rand::rng()
            .sample_iter(&Alphanumeric)
            .take(40)
            .map(char::from)
            .collect();
        let mut store = redis_store.lock().await;
        store.info.role = Role::Master;
        store.info.master_id = master_id.to_lowercase();
    };

    let listener = TcpListener::bind(format!("127.0.0.1:{port}")).await?;
    while let Ok((stream, _)) = listener.accept().await {
        let store = redis_store.clone();
        tokio::spawn(async move {
            Server::handle(store, stream, BytesMut::with_capacity(4 * 1024), 0).await;
        });
        println!("accepted new connection");
    }

    Ok(())
}
