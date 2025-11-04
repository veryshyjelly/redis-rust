use super::{Command, Redis};
use crate::resp::RESP;
use std::fmt::{Display, Formatter};

#[derive(Default)]
pub struct Info {
    pub connected_client: usize,
    pub listening_port: u16,
    pub role: Role,
    pub master_id: String,
    pub offset: isize,
}

pub enum Role {
    Master,
    Slave,
}

impl Redis {
    pub fn info(&mut self, _: Command) -> std::io::Result<RESP> {
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
    pub fn from_role(listening_port: u16, role: Role, master_id: String, offset: isize) -> Self {
        Info {
            listening_port,
            connected_client: 0,
            role,
            master_id,
            offset,
        }
    }

    pub fn new_slave(listening_port: u16) -> Self {
        Info {
            listening_port,
            connected_client: 0,
            role: Role::Slave,
            master_id: "?".into(),
            offset: -1,
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
            Role::Slave => write!(f, "slave"),
        }
    }
}
