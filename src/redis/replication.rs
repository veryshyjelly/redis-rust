use super::{Command, Redis, syntax_error};
use crate::redis::errors::wrong_num_arguments;
use crate::redis::redis::SlaveConfig;
use crate::resp::{RESP, TypedNone};
use rand::Rng;
use std::io::{Read, pipe};

const EMPTY_RDB: &str = "524544495330303131fa0972656469732d76657205372e322e30fa0a72656469732d62697473c040fa056374696d65c26d08bc65fa08757365642d6d656dc2b0c41000fa08616f662d62617365c000fff06e3bfec0ff5aa2";

impl Redis {
    pub fn replconf(&mut self, mut args: Command) -> std::io::Result<RESP> {
        let key = args
            .pop_front()
            .ok_or(wrong_num_arguments("replconf"))?
            .to_lowercase();
        if key == "getack" {
            let offset = self.store.lock().unwrap().info.offset.max(0);
            Ok(RESP::Push(vec![
                "REPLCONF".into(),
                "ACK".into(),
                offset.to_string().into(),
            ]))
        } else if key == "listening-port" {
            let port: u16 = args
                .pop_front()
                .ok_or(wrong_num_arguments("replconf"))?
                .parse()
                .unwrap();
            let s = self.slave_conf.get_or_insert(SlaveConfig {
                port: 0,
                capabilities: vec![],
            });
            s.port = port;
            Ok("OK".into())
        } else if key == "capa" {
            Ok("OK".into())
        } else {
            Ok(RESP::None(TypedNone::Nil))
        }
    }

    pub fn psync(&mut self, _: Command) -> std::io::Result<RESP> {
        let repl_id = self.store.lock().unwrap().info.master_id.clone();
        let sync_status: RESP = format!("FULLRESYNC {repl_id} 0").as_str().into();
        self.resp.send(sync_status)?;

        let res = hex::decode(EMPTY_RDB).unwrap();
        self.resp.send(RESP::RDB(res))?;

        let (mut pi, po) = pipe()?;
        self.store
            .lock()
            .unwrap()
            .slaves
            .insert(rand::rng().random_range(..usize::MAX), po);
        let mut buf = vec![0u8; 1024];

        loop {
            let n = pi.read(&mut buf)?;
            self.resp.send_bytes(&buf[..n])?;
        }
    }

    pub fn wait(&mut self, mut args: Command) -> std::io::Result<RESP> {
        let count_replicas: usize = args
            .pop_front()
            .ok_or(wrong_num_arguments("wait"))?
            .parse()
            .map_err(|_| syntax_error())?;
        let timeout: u128 = args
            .pop_front()
            .ok_or(wrong_num_arguments("wait"))?
            .parse()
            .map_err(|_| syntax_error())?;
        let now = std::time::Instant::now();
        
        loop {
            let current_replicas = self.store.lock().unwrap().slaves.len();
            if current_replicas >= count_replicas || now.elapsed().as_millis() > timeout {
                return Ok(current_replicas.into()) 
            }
        }
    }
}
