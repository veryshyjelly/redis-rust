use tokio::sync::{broadcast, oneshot};
use crate::Error;
use crate::frame::Frame;
use crate::server::errors::wrong_num_arguments;
use super::{Args, server::Server};

impl Server {
    /// Subscribes the client to the specified channels.
    /// 
    /// Once the client enters the subscribed state it is not supposed to 
    /// issue any other commands, except for additional SUBSCRIBE, SSUBSCRIBE, 
    /// PSUBSCRIBE, UNSUBSCRIBE, SUNSUBSCRIBE, PUNSUBSCRIBE, PING, RESET and QUIT 
    /// commands. However, if RESP3 is used (see HELLO) it is possible for a client to
    /// issue any commands while in subscribed state.
    /// ```
    /// SUBSCRIBE channel [channel ...]
    /// ```
    pub async fn subscribe(&mut self, mut args: Args) -> Result<Frame, Error> {
        let key = args.pop_front().ok_or(wrong_num_arguments("subscribe"))?;
        let mut receiver = if let Some(channel) = self.store.lock().await.channels.get(&key) {
            channel.subscribe() 
        } else {
            let (tx, rx) = broadcast::channel(64);
            self.store.lock().await.channels.insert(key.clone(), tx);
            rx
        };
        let (utx, mut urx) = oneshot::channel();
        self.unsubscribe.insert(key.clone(), utx);
        let output = self.output.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    value = receiver.recv() => {
                        if let Ok(value) = value {
                            if let Err(e) = output.send(value).await {
                                println!("stopping subscription channel due to {e}");
                                break
                            }
                        }
                    } 
                    value = &mut urx => {
                        drop(receiver);
                        println!("told to unsubscribe so stopping");
                        break
                    }
                }
            }
        });
        
        self.subscription_count += 1;
        let sub: Frame = "subscribe".to_string().into();
        Ok(vec![sub, key.into(), self.subscription_count.into()].into())
    }

    pub async fn publish(&mut self, mut args: Args) -> Result<Frame, Error> {
        let key = args.pop_front().ok_or(wrong_num_arguments("subscribe"))?;
        let msg = args.pop_front().ok_or(wrong_num_arguments("subscribe"))?;
        if let Some(channel) = self.store.lock().await.channels.get(&key) {
            channel.send(vec!["message".to_string(), key, msg].into())?;
            Ok(channel.receiver_count().into())
        } else {
            Ok(0usize.into())
        }
    }

    pub async fn unsubscribe(&mut self, mut args: Args) -> Result<Frame, Error> {
        let key = args.pop_front().ok_or(wrong_num_arguments("subscribe"))?;
        let _ =   self.unsubscribe.remove(&key).unwrap().send(true);
        self.subscription_count -= 1;
        let sub: Frame = "unsubscribe".to_string().into();
        Ok(vec![sub, key.into(), self.subscription_count.into()].into())
    }
}