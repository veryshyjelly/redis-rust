use super::Redis;
use crate::redis::utils::make_io_error;
use crate::resp::RESP;

impl Redis {
    pub fn rpush(&mut self, mut args: Vec<RESP>) -> std::io::Result<()> {
        let mut store = self.store.lock().unwrap();
        let key = args.remove(0).hashable();
        let e = store.list.entry(key).or_insert(vec![]);
        e.append(&mut args);
        let resp: RESP = e.len().into();
        write!(self.io, "{resp}")
    }

    pub fn lrange(&mut self, mut args: Vec<RESP>) -> std::io::Result<()> {
        let mut store = self.store.lock().unwrap();
        let key = args.remove(0).hashable();
        let list = store.list.entry(key).or_insert(vec![]);
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
        if start == 0 {
            end += 1;
        }

        let resp: RESP = list
            .get(start as usize..end as usize)
            .ok_or(make_io_error("index out of bounds"))?
            .to_vec()
            .into();

        write!(self.io, "{resp}")
    }
}
