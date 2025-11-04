use super::Redis;
use super::Command;
use super::errors::{wrong_num_arguments};
use super::utils::make_io_error;
use crate::resp::{TypedNone, RESP};

impl Redis {
    pub fn execute(&mut self, mut cmd: Command) -> std::io::Result<RESP> {
        if self.is_transaction {
            return self.transaction(cmd);
        }
        
        let name = cmd
            .pop_front()
            .ok_or(make_io_error("ERR expected command got nothing"))?;

        match name.to_lowercase().as_str() {
            "ping" => self.ping(cmd),
            "echo" => self.echo(cmd),
            "set" => self.set(cmd),
            "get" => self.get(cmd),
            "rpush" => self.rpush(cmd),
            "lpush" => self.lpush(cmd),
            "lrange" => self.lrange(cmd),
            "llen" => self.llen(cmd),
            "lpop" => self.lpop(cmd),
            "blpop" => self.blpop(cmd),
            "type" => self.redis_type(cmd),
            "xadd" => self.xadd(cmd),
            "xrange" => self.xrange(cmd),
            "xread" => self.xread(cmd),
            "xlen" => self.xlen(cmd),
            "incr" => self.incr(cmd),
            "multi" => self.multi(cmd),
            _ => self.invalid(cmd),
        }
    }

    /// Returns the string representation of the type of the value stored at key.
    /// The different types that can be returned are:
    /// string, list, set, zset, hash, stream, and vectorset.
    /// ```
    /// TYPE key
    /// ```
    fn redis_type(&mut self, mut args: Command) -> std::io::Result<RESP> {
        let key = args
            .pop_front()
            .ok_or(wrong_num_arguments("type"))?;
        let store = self.store.lock().unwrap();
        let resp  = store
            .kv
            .get(&key)
            .map(|v| v.redis_type())
            .unwrap_or("none".into())
            .into();
        Ok(resp)
    }

    /// Returns message.
    /// ```
    /// ECHO message
    /// ```
    fn echo(&mut self, mut args: Command) -> std::io::Result<RESP> {
        Ok(args.pop_front().ok_or(wrong_num_arguments("echo"))?.into())
    }

    /// Returns PONG if no argument is provided, otherwise return a copy of the argument as a bulk.
    /// ```
    /// PING [message]
    /// ```
    fn ping(&mut self, _: Command) -> std::io::Result<RESP> {
        Ok("PONG".into())
    }

    fn invalid(&mut self, _: Command) -> std::io::Result<RESP> {
        Ok(RESP::None(TypedNone::Nil))
    }
}
