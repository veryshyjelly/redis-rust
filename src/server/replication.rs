use super::Result;
use super::errors::*;
use super::server::{Server, SlaveConfig};
use crate::frame::{Frame, TypedNone};
use crate::server::Args;
use bytes::Bytes;
use std::mem;
use std::ops::AddAssign;

const EMPTY_RDB: &str = "524544495330303131fa0972656469732d76657205372e322e30fa0a72656469732d62697473c040fa056374696d65c26d08bc65fa08757365642d6d656dc2b0c41000fa08616f662d62617365c000fff06e3bfec0ff5aa2";

impl Server {
    pub async fn replconf(&mut self, mut args: Args) -> Result {
        let key = args
            .pop_front()
            .ok_or(wrong_num_arguments("replconf"))?
            .to_lowercase();
        match key.as_str() {
            "ack" => {
                self.store.lock().await.ack_received.add_assign(1);
                Ok("OK".into())
            }
            "getack" => {
                let offset = self.store.lock().await.info.offset.max(0);
                self.output
                    .send(vec!["REPLCONF".into(), "ACK".into(), offset.to_string()].into())
                    .await?;
                Ok("OK".into())
            }
            "listening-port" => {
                let port: u16 = args
                    .pop_front()
                    .ok_or(wrong_num_arguments("replconf"))?
                    .parse()
                    .unwrap();
                let s = self.slave_config.get_or_insert(SlaveConfig {
                    port: 0,
                    capabilities: vec![],
                });
                s.port = port;
                Ok("OK".into())
            }
            "capa" => {
                let v = args.pop_front().ok_or(wrong_num_arguments("capa"))?;
                self.slave_config.get_or_insert(SlaveConfig {
                    port: 0, capabilities: vec![],
                }).capabilities.push(v);
                Ok("OK".into())
            },
            _ => Ok(Frame::None(TypedNone::Nil)),
        }
    }

    pub async fn psync(&mut self, _: Args) -> Result {
        // tell that we are going to full resync
        let repl_id = self.store.lock().await.info.master_id.clone();
        let sync_status: Frame = format!("FULLRESYNC {repl_id} 0").as_str().into();
        self.output.send(sync_status).await?;

        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        // basically only thing that we will be sending on this connection will be
        // the broadcast commands nothing more, not even the responses will be sent
        let writer = mem::replace(&mut self.output, tx);
        let mut reader = self
            .store
            .lock()
            .await
            .broadcast
            .clone()
            .ok_or("invalid broadcast configuration")?
            .subscribe();

        tokio::spawn(async move {
            loop {
                if let Ok(v) = reader.recv().await {
                    if writer.send(v).await.is_err() {
                        break;
                    }
                } else {
                    break;
                }
            }
        });

        // send the empty rdb file
        let res = hex::decode(EMPTY_RDB).unwrap();
        Ok(Frame::RDB(Bytes::from(res)))
    }

    pub async fn wait(&mut self, mut args: Args) -> Result {
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

        self.store
            .lock()
            .await
            .broadcast
            .clone()
            .ok_or("broadcast not setup please fix")?
            .send(vec!["REPLCONF".to_string(), "GETACK".into(), "*".into()].into())?;

        loop {
            let ack_received = self.store.lock().await.ack_received;
            if ack_received >= count_replicas || now.elapsed().as_millis() > timeout {
                return Ok(ack_received.into());
            }
        }
    }
}
