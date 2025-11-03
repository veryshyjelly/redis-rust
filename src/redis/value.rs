use std::cmp::Ordering;
use crate::resp::{Hashable, RESP};
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH};

pub enum Value {
    String(RESP),
    List(VecDeque<RESP>),
    Set(HashSet<RESP>),
    ZSet,
    Hash,
    Stream(Vec<StreamEntry>),
    VectorSet,
}

pub struct StreamEntry {
    pub id: StreamEntryID,
    pub data: HashMap<Hashable, RESP>
}

impl Ord for StreamEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl Eq for StreamEntry {}

impl PartialOrd for StreamEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.id.partial_cmp(&other.id)
    } 
}

impl PartialEq for StreamEntry {
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct StreamEntryID {
    pub time: usize,
    pub sqn: usize
}

impl StreamEntryID {
    pub fn new() -> Self {
        StreamEntryID {
            time: SystemTime::now()
                .duration_since(UNIX_EPOCH).unwrap().as_millis() as usize,
            sqn: 0 
        }
    }
    
    pub fn explicit(s: String) -> Self {
        let tqn: Vec<usize> = s.split("-").map(|v| v.parse().unwrap()).collect();
        StreamEntryID {time: tqn[0], sqn: tqn[1] }
    }
    
    pub fn to_string(&self) -> String {
        format!("{}-{}", self.time, self.sqn)
    }
}

impl Into<Value> for RESP {
    fn into(self) -> Value {
        Value::String(self)
    }
}

impl Value {
    pub fn redis_type(&self) -> String {
        match self {
            Value::String(_) => "string",
            Value::List(_) => "list",
            Value::Set(_) => "set",
            Value::ZSet => "zset",
            Value::Hash => "hash",
            Value::Stream(_) => "stream",
            Value::VectorSet => "vectorset",
        }.into()
    }
    
    pub fn new_list() -> Value {
        Value::List(VecDeque::new())
    }
    
    pub fn new_stream() -> Value {
        Value::Stream(vec![])
    }

    pub fn string(&self) -> Option<&RESP> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn list(&self) -> Option<&VecDeque<RESP>> {
        match self {
            Value::List(l) => Some(l),
            _ => None,
        }
    }

    pub fn list_mut(&mut self) -> Option<&mut VecDeque<RESP>> {
        match self {
            Value::List(v) => Some(v),
            _ => None,
        }
    }
    
    pub fn stream(&self) -> Option<&Vec<StreamEntry>> {
        match self {
            Value::Stream(s) => Some(s),
            _ => None
        } 
    }
    
    pub fn stream_mut(&mut self) -> Option<&mut Vec<StreamEntry>> {
        match self {
            Value::Stream(s) => Some(s),
            _ => None
        }
    }
}
