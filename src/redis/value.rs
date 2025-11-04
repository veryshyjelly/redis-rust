use crate::resp::{RESP};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::{Display, Formatter};
use std::time::{SystemTime, UNIX_EPOCH};

pub enum Value {
    String(String),
    List(VecDeque<RESP>),
    Set(HashSet<RESP>),
    ZSet,
    Hash,
    Stream(Vec<StreamEntry>),
    VectorSet,
}

#[derive(Clone)]
pub struct StreamEntry {
    pub id: StreamEntryID,
    pub data: HashMap<String, String>,
}

impl Into<RESP> for StreamEntry {
    fn into(self) -> RESP {
        let mut res: Vec<RESP> = vec![];
        res.push(self.id.to_string().into());
        let mut kvs: Vec<RESP> = vec![];
        for (k, v) in &self.data {
            kvs.push(k.to_owned().into());
            kvs.push(v.to_owned().into());
        }
        res.push(kvs.into());
        res.into()
    }
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
    pub sqn: usize,
}

impl Display for StreamEntryID {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.time, self.sqn)
    }
}

impl StreamEntryID {
    pub fn new() -> Self {
        StreamEntryID {
            time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as usize,
            sqn: 0,
        }
    }

    /// Converts string from either specified only time
    /// or both time and sqn to `StreamEntryID`
    pub fn implicit(s: String) -> Self {
        if s.contains("-") {
            StreamEntryID::explicit(s)
        } else {
            let time = s.parse().unwrap();
            StreamEntryID::with_time(time)
        }
    }

    /// Creates a `StreamEntryID` with the given time
    /// if time is zero then sequence starts from 1
    pub fn with_time(time: usize) -> Self {
        let sqn = if time == 0 { 1 } else { 0 };
        StreamEntryID { time, sqn }
    }

    /// Create `StreamEntryID` from explicit string
    /// of the form <time_in_milliseconds>-<sequence_number>
    pub fn explicit(s: String) -> Self {
        let tqn: Vec<usize> = s.split("-").map(|v| v.parse().unwrap()).collect();
        StreamEntryID {
            time: tqn[0],
            sqn: tqn[1],
        }
    }
}

// impl Into<Value> for RESP {
//     fn into(self) -> Value {
//         let res = match self {
//             RESP::SimpleString(s) => s,
//             RESP::BulkString(s) => s,
//             RESP::SimpleError(e) => e,
//             RESP::BulkError(e) => e,
//             RESP::Integer(i) => i.to_string(),
//             RESP::Boolean(b) => b.to_string(),
//             RESP::Double(d) => d.to_string(),
//             RESP::BigNumber(b) => b,
//             _ => panic!("")
//         };
//         Value::String(res)
//     }
// }

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
