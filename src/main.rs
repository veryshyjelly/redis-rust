mod redis;
mod resp;
mod utils;

use crate::redis::RedisStore;
use redis::Redis;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6379")?;

    let redis_store = Arc::new(Mutex::new(RedisStore {
        kv: Default::default(),
        expiry_queue: Default::default(),
        expiry_time: Default::default()
    }));

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let store = redis_store.clone();
                thread::spawn(|| -> std::io::Result<()> {
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
