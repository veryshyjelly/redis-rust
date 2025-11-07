use tokio::sync::broadcast;
use crate::Error;
use crate::frame::Frame;
use crate::server::errors::wrong_num_arguments;
use super::{Args, server::Server};

impl Server {
    pub async fn subscribe(&mut self, mut args: Args) -> Result<Frame, Error> {
        let key = args.pop_front().ok_or(wrong_num_arguments("subscribe"))?;
        let mut receiver = if let Some(channel) = self.store.lock().await.channels.get(&key) {
            channel.subscribe() 
        } else {
            let (tx, rx) = broadcast::channel(64);
            self.store.lock().await.channels.insert(key.clone(), tx);
            rx
        };
        let output = self.output.clone();
        tokio::spawn(async move {
            loop {
                if let Ok(err) = receiver.recv().await {
                    if let Err(e) = output.send(err).await {
                        println!("stopping subscription channel due to {e}");
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
        todo!()
    }
}