use crate::resp::{RESP};
use std::collections::{HashSet, VecDeque};
use crate::redis::stream::StreamEntry;

pub enum Value {
    String(String),
    List(VecDeque<RESP>),
    Set(HashSet<RESP>),
    ZSet,
    Hash,
    Stream(Vec<StreamEntry>),
    VectorSet,
}


impl Into<Value> for String {
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
        }
        .into()
    }

    pub fn new_list() -> Value {
        Value::List(VecDeque::new())
    }

    pub fn new_stream() -> Value {
        Value::Stream(vec![])
    }

    pub fn string(&self) -> Option<&String> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn string_mut(&mut self) -> Option<&mut String> {
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
            _ => None,
        }
    }

    pub fn stream_mut(&mut self) -> Option<&mut Vec<StreamEntry>> {
        match self {
            Value::Stream(s) => Some(s),
            _ => None,
        }
    }
}
