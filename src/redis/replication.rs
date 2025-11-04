use super::{Command, Redis};
use crate::redis::errors::wrong_num_arguments;
use crate::redis::redis::SlaveConfig;
use crate::resp::{TypedNone, RESP};


const EMPTY_RDB: &str = "524544495330303131fa0972656469732d76657205372e322e30fa0a72656469732d62697473c040fa056374696d65c26d08bc65fa08757365642d6d656dc2b0c41000fa08616f662d62617365c000fff06e3bfec0ff5aa2";

impl Redis {
    pub fn replconf(&mut self, mut args: Command) -> std::io::Result<RESP> {
        let key = args.pop_front().ok_or(wrong_num_arguments("replconf"))?;
        if key == "listening-port" {
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

    pub fn psync(&mut self, mut args: Command) -> std::io::Result<RESP> {
        let repl_id = self.store.lock().unwrap().info.master_id.clone();
        let sync_status: RESP = format!("FULLRESYNC {repl_id} 0").as_str().into();
        self.resp.send(sync_status)?;
        let res = hex::decode(EMPTY_RDB).unwrap(); 
        Ok(RESP::RDB(res))
    }
}
