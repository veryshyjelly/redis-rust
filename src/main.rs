mod redis;
mod resp;
mod slave;

use crate::redis::{Info, RedisStore, Role};
use crate::slave::Slave;
use rand::{Rng, distr::Alphanumeric};
use redis::Redis;
use std::collections::HashMap;
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

    let redis_store = Arc::new(Mutex::new(RedisStore {
        slaves: HashMap::new(),
        kv: Default::default(),
        expiry_queue: Default::default(),
        expiry_time: Default::default(),
        info: Info::new_slave(port),
    }));

    let role = if let Some(idx) = args.iter().position(|v| v == "--replicaof") {
        let mut addr = args[idx + 1].split(" ");
        let ip_str = addr.next().unwrap();
        let ip = if ip_str == "localhost" {
            Ipv4Addr::from_str("127.0.0.1").unwrap()
        } else {
            Ipv4Addr::from_str(ip_str).unwrap()
        };
        let port: u16 = addr.next().unwrap().parse().unwrap();
        let store = redis_store.clone();
        thread::spawn(move || -> std::io::Result<()> {
            let mut slave = Slave::new(ip, port, store)?;
            slave.handle()
        });
        Role::Slave
    } else {
        Role::Master
    };

    match role {
        Role::Master => {
            let master_id: String = rand::rng()
                .sample_iter(&Alphanumeric)
                .take(40)
                .map(char::from)
                .collect();
            let info = Info::from_role(port, role, master_id.to_lowercase(), 0);
            redis_store.lock().unwrap().info = info;
        }
        Role::Slave => {}
    };

    let listener = TcpListener::bind(format!("127.0.0.1:{port}"))?;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let store = redis_store.clone();
                thread::spawn(|| -> std::io::Result<()> {
                    Redis::new(Box::new(stream), store).handle()
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
