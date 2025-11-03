use super::Redis;
use crate::resp::RESP;

impl Redis {
    pub fn execute(&mut self, mut cmd: Vec<RESP>) -> std::io::Result<()> {
        let name = cmd.remove(0).string().ok_or(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "expected string for command got something else",
        ))?;
        
        match name.to_lowercase().as_str() {
            "ping" => self.ping(cmd),
            "echo" => self.echo(cmd),
            "set" => self.set(cmd),
            "get" => self.get(cmd),
            _ => self.invalid(cmd),
        }
    }
    
    fn set(&mut self, mut args: Vec<RESP>) -> std::io::Result<()> {
        let key = args.remove(0).hashable();
        let value = args.remove(0);
        self.store.insert(key, value);
        let resp: RESP = "OK".into();
        write!(self.io, "{resp}")
    }
    
    fn get(&mut self, mut args: Vec<RESP>) -> std::io::Result<()> {
        let key = args.remove(0).hashable();
        if let Some(v) = self.store.get(&key) {
            write!(self.io, "{v}") 
        } else {
            write!(self.io, "{}", RESP::null_bulk_string()) 
        }
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
