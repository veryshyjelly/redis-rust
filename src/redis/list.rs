use super::Redis;
use crate::redis::utils::make_io_error;
use crate::redis::value::Value;
use crate::resp::RESP;
use std::collections::VecDeque;
use std::thread::sleep;
use std::time::Duration;

impl Redis {
    pub fn rpush(&mut self, mut args: Vec<RESP>) -> std::io::Result<()> {
        let mut store = self.store.lock().unwrap();
        let key = args.remove(0).hashable();
        let e = store
            .kv
            .entry(key)
            .or_insert(Value::new_list())
            .list_mut()
            .unwrap();
        e.extend(args.into_iter());
        let resp: RESP = e.len().into();
        write!(self.io, "{resp}")
    }

    pub fn lpush(&mut self, mut args: Vec<RESP>) -> std::io::Result<()> {
        let mut store = self.store.lock().unwrap();
        let key = args.remove(0).hashable();
        let e = store
            .kv
            .entry(key)
            .or_insert(Value::new_list())
            .list_mut()
            .unwrap();
        for v in args {
            e.push_front(v);
        }

        let resp: RESP = e.len().into();
        write!(self.io, "{resp}")
    }

    pub fn lpop(&mut self, mut args: Vec<RESP>) -> std::io::Result<()> {
        let key = args.remove(0).hashable();
        let count: usize = args
            .get(0)
            .unwrap_or(&"1".into())
            .clone()
            .string()
            .unwrap()
            .parse()
            .unwrap();

        let mut store = self.store.lock().unwrap();
        if let Some(list) = store.kv.get_mut(&key).and_then(|v| v.list_mut()) {
            let count = count.min(list.len());
            if count == 0 {
                let resp = RESP::null_bulk_string();
                write!(self.io, "{resp}")
            } else if count == 1 {
                write!(self.io, "{}", list.pop_front().unwrap())
            } else {
                let res: Vec<_> = (0..count).map(|_| list.pop_front().unwrap()).collect();
                let resp: RESP = res.into();
                write!(self.io, "{resp}")
            }
        } else {
            let resp = RESP::null_bulk_string();
            write!(self.io, "{resp}")
        }
    }

    pub fn blpop(&mut self, mut args: Vec<RESP>) -> std::io::Result<()> {
        let key = args.remove(0).hashable();
        let now = std::time::Instant::now();
        let mut time_out: f64 = args
            .get(0)
            .unwrap_or(&"0.0".into())
            .clone()
            .string()
            .unwrap()
            .parse()
            .unwrap();

        if time_out == 0.0 {
            time_out = f64::INFINITY;
        }

        while now.elapsed().as_secs_f64() < time_out {
            let mut store = self.store.lock().unwrap();
            let list = match store.kv.get_mut(&key) {
                Some(l) => l.list_mut().unwrap(),
                None => continue,
            };
            if let Some(v) = list.pop_front() {
                let resp: RESP = vec![key.into(), v].into();
                return write!(self.io, "{resp}");
            }
            sleep(Duration::from_millis(1));
        }

        let resp = RESP::null_array();
        write!(self.io, "{resp}")
    }

    pub fn lrange(&mut self, mut args: Vec<RESP>) -> std::io::Result<()> {
        let mut store = self.store.lock().unwrap();
        let key = args.remove(0).hashable();
        let list = store
            .kv
            .entry(key)
            .or_insert(Value::new_list())
            .list_mut()
            .unwrap();
        let n = list.len();

        let mut start: isize = args
            .remove(0)
            .string()
            .ok_or(make_io_error("expected start index"))?
            .parse()
            .unwrap();
        let mut end: isize = args
            .remove(0)
            .string()
            .ok_or(make_io_error("expected end index"))?
            .parse()
            .unwrap();

        if start < 0 {
            start += n as isize;
        }
        if end < 0 {
            end += n as isize;
        }
        let start = 0.max(start) as usize;
        let end = 0.max(end) as usize;
        let end = n.min(end + 1);

        let resp: RESP = list.range(start..end).cloned().collect::<Vec<_>>().into();

        write!(self.io, "{resp}")
    }

    pub fn llen(&mut self, mut args: Vec<RESP>) -> std::io::Result<()> {
        let store = self.store.lock().unwrap();
        let key = args.remove(0).hashable();

        let n: RESP = match store.kv.get(&key) {
            Some(l) => l.list().unwrap().len(),
            None => 0,
        }
        .into();

        write!(self.io, "{n}")
    }
}
