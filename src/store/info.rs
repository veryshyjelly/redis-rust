use super::{Info, Role};
use std::fmt::{Display, Formatter};

impl Info {
    pub fn from_role(listening_port: u16, role: Role, master_id: String, offset: usize) -> Self {
        let mut res = Info::default();
        res.listening_port = listening_port;
        res.role = role;
        res.master_id = master_id;
        res.recv_offset = offset;
        res
    }

    pub fn new_slave(listening_port: u16) -> Self {
        let mut res = Info::default();
        res.listening_port = listening_port;
        res.role = Role::Slave;
        res.master_id = "?".into();
        res.recv_offset = 0;
        res
    }
}

impl Display for Info {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "# Clients")?;
        writeln!(f, "connected_clients:{}", self.connected_client)?;
        writeln!(f, "# Replication")?;
        writeln!(f, "role:{}", self.role)?;
        writeln!(f, "master_replid:{}", self.master_id)?;
        writeln!(f, "master_repl_offset:{}", self.recv_offset)?;
        Ok(())
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
