mod info;
mod stream;
mod value;

use crate::frame::Frame;
use ordered_float::OrderedFloat;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use tokio::sync::broadcast;

pub struct Store {
    pub info: Info,
    pub broadcast: Option<broadcast::Sender<Frame>>,
    pub get_ack_channel: Option<broadcast::Sender<Frame>>,
    pub kv: HashMap<String, Value>,
    pub channels: HashMap<String, broadcast::Sender<Frame>>,
    pub slave_offsets: HashMap<usize, usize>,
    pub slave_asked_offsets: HashMap<usize, usize>,
    pub expiry_queue: BTreeMap<std::time::SystemTime, String>,
    pub expiry_time: HashMap<String, std::time::SystemTime>,
}

pub enum Value {
    String(String),
    List(VecDeque<Frame>),
    Set(HashSet<Frame>),
    ZSet(BTreeMap<OrderedFloat<f64>, Frame>),
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
    pub send_offset: usize,
    pub recv_offset: usize,
    pub connected_client: usize,
    pub listening_port: u16,
    pub dir: String,
    pub db_filename: String,
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
