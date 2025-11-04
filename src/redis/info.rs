use super::{Command, Redis};
use crate::resp::RESP;
use std::fmt::{Display, Formatter};
use std::net::Ipv4Addr;

#[derive(Default)]
pub struct Info {
    pub connected_client: usize,
    pub role: Role,
    pub master_id: String,
    pub offset: usize,
}

pub enum Role {
    Master,
    Slave((Ipv4Addr, usize)),
}

impl Redis {
    pub fn info(&mut self, mut cmd: Command) -> std::io::Result<RESP> {
        Ok(RESP::BulkString(
            self.store.lock().unwrap().info.to_string(),
        ))
    }
}

impl Display for Info {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "# Clients")?;
        writeln!(f, "connected_clients:{}", self.connected_client)?;
        writeln!(f, "# Replication")?;
        writeln!(f, "role:{}", self.role)?;
        writeln!(f, "master_replid:{}", self.master_id)?;
        writeln!(f, "master_repl_offset:{}", self.offset)?; 
        Ok(())
    }
}

impl Info {
    pub fn from_role(role: Role, master_id: String, offset: usize) -> Self {
        Info {
            connected_client: 0, role,
            master_id,
            offset
        }
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
            Role::Slave(_) => write!(f, "slave"),
        }
    }
}
