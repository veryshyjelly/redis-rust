use super::Redis;
use crate::redis::errors::{syntax_error, wrong_num_arguments};
use crate::resp::RESP;
use crate::utils::make_io_error;
use std::collections::VecDeque;

impl Redis {
    pub fn execute(&mut self, mut cmd: VecDeque<RESP>) -> std::io::Result<()> {
        let name = cmd
            .pop_front()
            .ok_or(make_io_error("ERR expected command got nothing"))?
            .string()
            .ok_or(syntax_error())?;

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
            _ => self.invalid(cmd),
        }
    }

    /// Returns the string representation of the type of the value stored at key.
    /// The different types that can be returned are:
    /// string, list, set, zset, hash, stream, and vectorset.
    /// ```
    /// TYPE key
    /// ```
    fn redis_type(&mut self, mut args: VecDeque<RESP>) -> std::io::Result<()> {
        let key = args
            .pop_front()
            .ok_or(wrong_num_arguments("type"))?
            .hashable()?;
        let store = self.store.lock().unwrap();
        let resp: RESP = store
            .kv
            .get(&key)
            .map(|v| v.redis_type())
            .unwrap_or("none".into())
            .into();
        write!(self.io, "{resp}")
    }

    /// Returns message.
    /// ```
    /// ECHO message
    /// ```
    fn echo(&mut self, args: VecDeque<RESP>) -> std::io::Result<()> {
        write!(self.io, "{}", args[0])
    }

    /// Returns PONG if no argument is provided, otherwise return a copy of the argument as a bulk.
    /// ```
    /// PING [message]
    /// ```
    fn ping(&mut self, _: VecDeque<RESP>) -> std::io::Result<()> {
        let resp: RESP = "PONG".into();
        write!(self.io, "{resp}")
    }

    fn invalid(&mut self, _: VecDeque<RESP>) -> std::io::Result<()> {
        let resp = RESP::None;
        write!(self.io, "{resp}")
    }
}
