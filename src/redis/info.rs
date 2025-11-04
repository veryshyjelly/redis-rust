use std::fmt::{Display, Formatter};
use super::{Command, Redis};
use crate::resp::RESP;

#[derive(Default)]
pub struct Info {
    pub connected_client: usize,
    pub role: Role
}

pub enum Role {
    Master,
    Slave
}

impl Redis {
    pub fn info(&mut self, mut cmd: Command) -> std::io::Result<RESP> {
        Ok(RESP::BulkString(self.store.lock().unwrap().info.to_string())) 
    }
}

impl Display for Info {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "# Clients")?;
        writeln!(f, "connected_clients:{}", self.connected_client)?;
        writeln!(f, "# Replication")?;
        writeln!(f, "role:{}", self.role)?;
        Ok(())
    }
}

impl Default for Role {
    fn default() -> Self {
        Role::Master
    }
}

impl Display for Role {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Master => write!(f, "master"),
            Role::Slave => write!(f, "slave")
        }
    }
}