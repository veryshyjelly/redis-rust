mod redis;
mod resp;

use crate::redis::RedisStore;
use redis::Redis;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

fn main() -> std::io::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    
    let mut port = 6379;
    if let Some(idx) = args.iter().position(|v| v=="--port") {
        port = args[idx + 1].parse().unwrap();
    }
    
    let listener = TcpListener::bind(format!("127.0.0.1:{port}"))?;

    let redis_store = Arc::new(Mutex::new(RedisStore {
        kv: Default::default(),
        expiry_queue: Default::default(),
        expiry_time: Default::default(),
        info: Default::default()
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
