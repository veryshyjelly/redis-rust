use super::errors::*;
use super::server::Server;
use super::{Args, Result};
use crate::frame::{Frame, TypedNone};

impl Server {
    /// Returns the string representation of the type of the value stored at key.
    /// The different types that can be returned are:
    /// string, list, set, zset, hash, stream, and vectorset.
    /// ```
    /// TYPE key
    /// ```
    pub async fn redis_type(&mut self, mut args: Args) -> Result {
        let key = args.pop_front().ok_or(wrong_num_arguments("type"))?;
        let store = self.store.lock().await;
        let resp = store
            .kv
            .get(&key)
            .map(|v| v.redis_type())
            .unwrap_or("none".into())
            .as_str()
            .into();
        Ok(resp)
    }

    /// The INFO command returns information and statistics about the server in a format
    /// that is simple to parse by computers and easy to read by humans.
    /// ```
    /// INFO [section [section ...]]
    /// ```
    pub async fn info(&mut self, _: Args) -> Result {
        Ok(Frame::BulkString(
            self.store.lock().await.info.to_string().into(),
        ))
    }

    /// Returns message.
    /// ```
    /// ECHO message
    /// ```
    pub async fn echo(&mut self, mut args: Args) -> Result {
        Ok(args.pop_front().ok_or(wrong_num_arguments("echo"))?.into())
    }

    /// Returns PONG if no argument is provided, otherwise return a copy of the argument as a bulk.
    /// ```
    /// PING [message]
    /// ```
    pub async fn ping(&mut self, _: Args) -> Result {
        Ok("PONG".into())
    }

    pub async fn invalid(&mut self, _: Args) -> Result {
        Ok(Frame::None(TypedNone::Nil))
    }
}
