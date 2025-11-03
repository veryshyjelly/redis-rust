use super::Redis;
use crate::redis::utils::make_io_error;
use crate::resp::RESP;
use std::collections::VecDeque;
use std::thread::sleep;
use std::time::Duration;

impl Redis {
    pub fn rpush(&mut self, mut args: Vec<RESP>) -> std::io::Result<()> {
        let mut store = self.store.lock().unwrap();
        let key = args.remove(0).hashable();
        let e = store.list.entry(key).or_insert(VecDeque::new());
        e.extend(args.into_iter());
        let resp: RESP = e.len().into();
        write!(self.io, "{resp}")
    }

    pub fn lpush(&mut self, mut args: Vec<RESP>) -> std::io::Result<()> {
        let mut store = self.store.lock().unwrap();
        let key = args.remove(0).hashable();
        let e = store.list.entry(key).or_insert(VecDeque::new());
        for v in args {
            e.push_front(v);
        }

        let resp: RESP = e.len().into();
        write!(self.io, "{resp}")
    }

    pub fn lpop(&mut self, mut args: Vec<RESP>) -> std::io::Result<()> {
        let key = args.remove(0).hashable();

        let mut store = self.store.lock().unwrap();
        let list = store.list.entry(key).or_insert(VecDeque::new());
        if let Some(v) = list.pop_front() {
            write!(self.io, "{v}")
        } else {
            let resp = RESP::null_bulk_string();
            write!(self.io, "{resp}")
        }
    }

    pub fn blpop(&mut self, mut args: Vec<RESP>) -> std::io::Result<()> {
        let key = args.remove(0).hashable();
        let now = std::time::Instant::now();
        let time_out: f64 = args
            .get(0)
            .unwrap_or(&"inf".into())
            .clone()
            .string()
            .unwrap()
            .parse()
            .unwrap();

        while now.elapsed().as_secs_f64() < time_out {
            let mut store = self.store.lock().unwrap();
            let list = match store.list.get_mut(&key) {
                Some(l) => l,
                None => continue,
            };
            if let Some(v) = list.pop_front() {
                return write!(self.io, "{v}");
            }
            sleep(Duration::from_millis(1));
        }

        let resp = RESP::null_bulk_string();
        write!(self.io, "{resp}")
    }

    pub fn lrange(&mut self, mut args: Vec<RESP>) -> std::io::Result<()> {
        let mut store = self.store.lock().unwrap();
        let key = args.remove(0).hashable();
        let list = store.list.entry(key).or_insert(VecDeque::new());
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
        let mut store = self.store.lock().unwrap();
        let key = args.remove(0).hashable();
        let list = store.list.entry(key).or_insert(VecDeque::new());
        let n: RESP = list.len().into();
        write!(self.io, "{n}")
    }
}
