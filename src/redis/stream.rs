use std::collections::HashMap;
use crate::redis::Redis;
use crate::redis::utils::make_io_error;
use crate::redis::value::{StreamEntry, StreamEntryID, Value};
use crate::resp::RESP;

impl Redis {
    pub fn xadd(&mut self, mut args: Vec<RESP>) -> std::io::Result<()> {
        let key = args.remove(0).hashable();
        let mut store = self.store.lock().unwrap();
        let stream = store.kv.entry(key).or_insert(Value::new_stream()).stream_mut().unwrap();
        let id = args.remove(0).string().ok_or(make_io_error("expected string id"))?;
        let mut data = HashMap::new();
        while args.len() > 0 {
            let key = args.remove(0).hashable();
            let value = args.remove(0);
            data.insert(key, value);
        }
        
        let id = if id == "*" {
            StreamEntryID::new()
        } else if id.contains("*") {
            let time: usize = id.split("-").nth(0).unwrap().parse().unwrap();
            let mut sqn = if time == 0 {1} else {0};
            if let Some(x) = stream.last() {
                if x.id.time == time {
                    sqn = x.id.sqn + 1;
                }
            }
            StreamEntryID {time, sqn}
        } else {
            StreamEntryID::explicit(id)   
        };
        
        let entry = StreamEntry {id, data};
        if id == (StreamEntryID {time: 0, sqn: 0}) {
            let resp = RESP::SimpleError("ERR The ID specified in XADD must be greater than 0-0".into());
            write!(self.io, "{resp}")
        } else if stream.is_empty() || &entry > stream.last().unwrap() {
            stream.push(entry);
            let resp: RESP = id.to_string().into();
            write!(self.io, "{resp}")
        } else {
            let resp = RESP::SimpleError("ERR The ID specified in XADD is equal or smaller than the target stream top item".into());
            write!(self.io, "{resp}")
        }
    }
}