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

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args: Vec<_> = std::env::args().collect();
    let port = if let Some(idx) = args.iter().position(|v| v == "--port") {
        args[idx + 1].parse().unwrap()
    } else {
        6379
    };

    let redis_store = Arc::new(Mutex::new(Store {
        kv: Default::default(),
        expiry_queue: Default::default(),
        expiry_time: Default::default(),
        info: Info::new_slave(port),
        broadcast: None,
        get_ack_channel: None,
        slave_offsets: HashMap::new(),
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
        let store = redis_store.clone();
        tokio::spawn(async move {
            slave::handle(ip, port, store).await.unwrap();
        });
    } else {
        let (tx, _rx) = tokio::sync::broadcast::channel(64);
        let _ = redis_store.lock().await.broadcast.insert(tx);
        let (tx, _rx) = tokio::sync::broadcast::channel(64);
        let _ = redis_store.lock().await.get_ack_channel.insert(tx);
        let master_id: String = rand::rng()
            .sample_iter(&Alphanumeric)
            .take(40)
            .map(char::from)
            .collect();
        let info = Info::from_role(port, Role::Master, master_id.to_lowercase(), 0);
        redis_store.lock().await.info = info;
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
