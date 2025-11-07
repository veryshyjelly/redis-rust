mod info;
mod stream;
mod value;

use crate::frame::Frame;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use tokio::sync::broadcast;

pub struct Store {
    pub info: Info,
    pub broadcast: Option<broadcast::Sender<Frame>>,
    pub kv: HashMap<String, Value>,
    pub ack_received: usize,
    pub expiry_queue: BTreeMap<std::time::Instant, String>,
    pub expiry_time: HashMap<String, std::time::Instant>,
}

pub enum Value {
    String(String),
    List(VecDeque<Frame>),
    Set(HashSet<Frame>),
    ZSet(BTreeSet<Frame>),
    Hash,
    Stream(Vec<StreamEntry>),
    VectorSet,
}

#[derive(Clone)]
pub struct StreamEntry {
    pub id: StreamEntryID,
    pub data: HashMap<String, String>,
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct StreamEntryID {
    pub time: usize,
    pub sqn: usize,
}

#[derive(Default)]
pub struct Info {
    pub role: Role,
    pub master_id: String,
    pub offset: isize,
    pub connected_client: usize,
    pub listening_port: u16,
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Role {
    Master,
    Slave,
}

impl Default for Role {
    fn default() -> Self {
        Role::Master
    }
}
