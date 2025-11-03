use super::utils::make_io_error;
use super::Redis;
use crate::resp::RESP;

impl Redis {
    pub fn execute(&mut self, mut cmd: Vec<RESP>) -> std::io::Result<()> {
        let name = cmd.remove(0).string().ok_or(make_io_error(
            "expected string for command got something else",
        ))?;

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
            _ => self.invalid(cmd),
        }
    }
    
    fn redis_type(&mut self, mut args: Vec<RESP>) -> std::io::Result<()> {
        let key = args.remove(0).hashable();
        let store = self.store.lock().unwrap();
        let resp: RESP = store.kv.get(&key).map(|v| v.redis_type()).unwrap_or("none".into()).into();
        write!(self.io, "{resp}")
    }

    fn echo(&mut self, args: Vec<RESP>) -> std::io::Result<()> {
        write!(self.io, "{}", args[0])
    }

    fn ping(&mut self, _: Vec<RESP>) -> std::io::Result<()> {
        let resp: RESP = "PONG".into();
        write!(self.io, "{resp}")
    }

    fn invalid(&mut self, _: Vec<RESP>) -> std::io::Result<()> {
        let resp = RESP::None;
        write!(self.io, "{resp}")
    }
}
