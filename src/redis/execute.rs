use super::Redis;
use crate::resp::RESP;
use std::ops::Add;
use std::time::Duration;

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
            _ => self.invalid(cmd),
        }
    }

    fn set(&mut self, mut args: Vec<RESP>) -> std::io::Result<()> {
        let key = args.remove(0).hashable();
        let value = args.remove(0);
        self.store.insert(key.clone(), value);

        if args.len() > 0 {
            let unit = args
                .remove(0)
                .string()
                .ok_or(make_io_error("expected string for unit of time"))?;
            let mut time =
                args.remove(0)
                    .string()
                    .ok_or(make_io_error("expected expiry time"))?.parse().unwrap();
            if unit.to_lowercase() == "ex" {
                time *= 1000;
            }
            let expiry_time = std::time::Instant::now().add(Duration::from_millis(time));
            self.expiry.insert(expiry_time, key);
        }

        let resp: RESP = "OK".into();
        write!(self.io, "{resp}")
    }

    fn get(&mut self, mut args: Vec<RESP>) -> std::io::Result<()> {
        self.remove_expired();

        let key = args.remove(0).hashable();
        if let Some(v) = self.store.get(&key) {
            write!(self.io, "{v}")
        } else {
            write!(self.io, "{}", RESP::null_bulk_string())
        }
    }

    fn remove_expired(&mut self) {
        while !self.expiry.is_empty() {
            let (t, key) = match self.expiry.pop_first() {
                Some(v) => v,
                None => break,
            };
            if t > std::time::Instant::now() {
                self.expiry.insert(t, key);
                break;
            }
            self.store.remove(&key);
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

fn make_io_error(message: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, message)
}
