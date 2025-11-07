use super::Result;
use super::errors::*;
use super::server::{Server, SlaveConfig};
use crate::frame::{Frame, TypedNone};
use crate::server::Args;
use bytes::{Bytes, BytesMut};
use std::mem;
use std::thread::sleep;
use std::time::Duration;
use rand::random_range;
use crate::frame::encode::AsBytes;

const EMPTY_RDB: &str = "524544495330303131fa0972656469732d76657205372e322e30fa0a72656469732d62697473c040fa056374696d65c26d08bc65fa08757365642d6d656dc2b0c41000fa08616f662d62617365c000fff06e3bfec0ff5aa2";

impl Server {
    pub async fn replconf(&mut self, mut args: Args) -> Result {
        let key = args
            .pop_front()
            .ok_or(wrong_num_arguments("replconf"))?
            .to_lowercase();
        match key.as_str() {
            "ack" => {
                // let offset = args.pop_front().ok_or(wrong_num_arguments("replconf"))?;
                let offset = self.store.lock().await.info.send_offset;
                let slave_id = self.slave_id;
                self.store
                    .lock()
                    .await
                    .slave_offsets
                    .insert(slave_id, offset);
                Ok("OK".into())
            }
            "getack" => {
                let offset = self.store.lock().await.info.recv_offset.max(0);
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
                self.slave_config
                    .get_or_insert(SlaveConfig {
                        port: 0,
                        capabilities: vec![],
                    })
                    .capabilities
                    .push(v);
                Ok("OK".into())
            }
            _ => Ok(Frame::None(TypedNone::Nil)),
        }
    }

    pub async fn psync(&mut self, _: Args) -> Result {
        // tell that we are going to full resync
        let repl_id = self.store.lock().await.info.master_id.clone();
        let sync_status: Frame = format!("FULLRESYNC {repl_id} 0").as_str().into();
        self.output.send(sync_status).await?;

        // send the empty rdb file
        let res = hex::decode(EMPTY_RDB).unwrap();
        self.output.send(Frame::RDB(Bytes::from(res))).await?;

        let (mut tx, _rx) = tokio::sync::mpsc::channel(1);
        // basically only thing that we will be sending on this connection will be
        // the broadcast commands nothing more, not even the responses will be sent
        mem::swap(&mut self.output, &mut tx);
        let mut reader = self
            .store
            .lock()
            .await
            .broadcast
            .clone()
            .ok_or("invalid broadcast configuration")?
            .subscribe();

        let px = tx.clone();
        tokio::spawn(async move {
            loop {
                if let Ok(v) = reader.recv().await {
                    if let Err(e) = tx.send(v).await {
                        println!("stopping sending because {e}");
                        break;
                    }
                } else {
                    break;
                }
            }
        });

        let mut ack_reader = self
            .store
            .lock()
            .await
            .get_ack_channel
            .clone()
            .ok_or("invalid get_ack configuration")?
            .subscribe();

        let slave_id = random_range(1..usize::MAX);
        self.slave_id = slave_id;
        self.store.lock().await.slave_offsets.insert(slave_id, 0);

        let store = self.store.clone();
        let mut asked_offset = self.asked_offset;
        
        tokio::spawn(async move {
            loop {
                if let Ok(v) = ack_reader.recv().await {
                    // println!("got instructions to send get ack");
                    sleep(Duration::from_millis(1));
                    let store = store.lock().await;
                    let send_offset = store.info.send_offset;
                    if send_offset > asked_offset {
                        println!("send ack but don't know if she got :(");
                        if let Err(e) = px.send(v).await {
                            println!("stopping sending because {e}");
                            break;
                        }
                    } else {
                        // println!("but send offset was {send_offset} and ask_offset was {asked_offset}");
                    }
                    asked_offset = send_offset;
                } else {
                    break;
                }
            }
        });

        Ok("OK".into())
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

        let message: Frame = vec!["REPLCONF".to_string(), "GETACK".into(), "*".into()].into();
        let _ = self
            .store
            .lock()
            .await
            .get_ack_channel
            .clone()
            .ok_or("broadcast not setup please fix")?
            .send(message.clone());
        let mut b = BytesMut::new();
        message.encode_bytes(&mut b);
        
        let now = std::time::Instant::now();
        loop {
            let send_offset = self.store.lock().await.info.send_offset;
            let ack_received = self
                .store
                .lock()
                .await
                .slave_offsets
                .values()
                .filter(|&&x| {
                    // println!("recv_offset = {x}, send_offset = {send_offset}");
                    x == send_offset
                })
                .count();
            
            if ack_received >= count_replicas || now.elapsed().as_millis() > timeout {
                return Ok(ack_received.into());
            }
            
            let _ = self
                .store
                .lock()
                .await
                .get_ack_channel
                .clone()
                .ok_or("broadcast not setup please fix")?
                .send(message.clone());
        }
    }
}
