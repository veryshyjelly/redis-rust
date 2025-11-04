mod redis;
mod resp;

use crate::redis::{Info, RedisStore, Role};
use rand::{distr::Alphanumeric, Rng};
use redis::Redis;
use std::net::{Ipv4Addr, TcpListener};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::thread;

fn main() -> std::io::Result<()> {
    let args: Vec<_> = std::env::args().collect();

    let port = if let Some(idx) = args.iter().position(|v| v == "--port") {
        args[idx + 1].parse().unwrap()
    } else {
        6379
    };

    let role = if let Some(idx) = args.iter().position(|v| v == "--replicaof") {
        let mut addr = args[idx + 1].split(" ");
        let ip_str = addr.next().unwrap();
        let ip = if ip_str == "localhost" {
            Ipv4Addr::from_str("127.0.0.1").unwrap()
        } else {
            Ipv4Addr::from_str(ip_str).unwrap()
        };
        let port: usize = addr.next().unwrap().parse().unwrap();
        Role::Slave((ip, port))
    } else {
        Role::Master
    };

    let master_id: String = rand::rng()
        .sample_iter(&Alphanumeric)
        .take(40)
        .map(char::from)
        .collect();

    let listener = TcpListener::bind(format!("127.0.0.1:{port}"))?;

    let redis_store = Arc::new(Mutex::new(RedisStore {
        kv: Default::default(),
        expiry_queue: Default::default(),
        expiry_time: Default::default(),
        info: Info::from_role(role, master_id.to_lowercase(), 0),
    }));

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let store = redis_store.clone();
                thread::spawn(|| {
                    let mut redis = Redis::new(Box::new(stream), store);
                    redis.handle()
                });
                println!("accepted new connection");
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }

    Ok(())
}
