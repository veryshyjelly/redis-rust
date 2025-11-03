use super::Redis;
use crate::redis::errors::{syntax_error, wrong_num_arguments, wrong_type};
use crate::redis::value::Value;
use crate::resp::RESP;
use std::collections::VecDeque;
use std::thread::sleep;
use std::time::Duration;

impl Redis {
    /// Insert all the specified values at the tail of the list stored at key.
    /// If key does not exist, it is created as empty list before performing
    /// the push operation. When key holds a value that is not a list, an error is returned.
    ///
    /// It is possible to push multiple elements using a single command call just
    /// specifying multiple arguments at the end of the command. Elements are
    /// inserted one after the other to the tail of the list, from the leftmost
    /// element to the rightmost element. So for instance the command
    /// RPUSH mylist a b c will result into a list containing a as first element,
    /// b as second element and c as third element.
    /// ```
    /// RPUSH key element [element ...]
    /// ```
    pub fn rpush(&mut self, mut args: VecDeque<RESP>) -> std::io::Result<()> {
        let mut store = self.store.lock().unwrap();
        let key = args
            .pop_front()
            .ok_or(wrong_num_arguments("rpush"))?
            .hashable()?;
        let e = store
            .kv
            .entry(key)
            .or_insert(Value::new_list())
            .list_mut()
            .ok_or(wrong_type())?;
        e.append(&mut args);
        let resp: RESP = e.len().into();
        write!(self.io, "{resp}")
    }

    /// Insert all the specified values at the head of the list stored at key.
    /// If key does not exist, it is created as empty list before performing the
    /// push operations. When key holds a value that is not a list, an error is returned.
    ///
    /// It is possible to push multiple elements using a single command call just
    /// specifying multiple arguments at the end of the command. Elements are inserted
    /// one after the other to the head of the list, from the leftmost element to the
    /// rightmost element. So for instance the command LPUSH mylist a b c will result into a
    /// list containing c as first element, b as second element and a as third element.
    /// ```
    /// LPUSH key element [element ...]
    /// ```
    pub fn lpush(&mut self, mut args: VecDeque<RESP>) -> std::io::Result<()> {
        let mut store = self.store.lock().unwrap();
        let key = args
            .pop_front()
            .ok_or(wrong_num_arguments("lpush"))?
            .hashable()?;
        let e = store
            .kv
            .entry(key)
            .or_insert(Value::new_list())
            .list_mut()
            .unwrap();
        e.append(&mut args);
        for v in args {
            e.push_front(v);
        }

        let resp: RESP = e.len().into();
        write!(self.io, "{resp}")
    }

    /// Removes and returns the first elements of the list stored at key.
    ///
    /// By default, the command pops a single element from the beginning of the list.
    /// When provided with the optional count argument, the reply will consist of up
    /// to count elements, depending on the list's length.
    /// ```
    /// LPOP key [count]
    /// ```
    pub fn lpop(&mut self, mut args: VecDeque<RESP>) -> std::io::Result<()> {
        let key = args
            .pop_front()
            .ok_or(wrong_num_arguments("lpop"))?
            .hashable()?;
        let count: usize = args
            .pop_front()
            .unwrap_or("1".into())
            .string()
            .ok_or(syntax_error())?
            .parse()
            .map_err(|_| syntax_error())?;

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

    /// BLPOP is a blocking list pop primitive. It is the blocking version of LPOP
    /// because it blocks the connection when there are no elements to pop from any
    /// of the given lists. An element is popped from the head of the first list that is
    /// non-empty, with the given keys being checked in the order that they are given.
    /// ```
    /// BLPOP key [key ...] timeout
    /// ```
    pub fn blpop(&mut self, mut args: VecDeque<RESP>) -> std::io::Result<()> {
        let err = || wrong_num_arguments("blpop");

        let key = args.pop_front().ok_or(err())?.hashable()?;
        let now = std::time::Instant::now();
        let mut time_out: f64 = args
            .pop_front()
            .unwrap_or("0.0".into())
            .string()
            .ok_or(syntax_error())?
            .parse()
            .map_err(|_| syntax_error())?;

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
            drop(store);
            sleep(Duration::from_millis(1));
        }

        let resp = RESP::null_array();
        write!(self.io, "{resp}")
    }

    /// Returns the specified elements of the list stored at key. The offsets start
    /// and stop are zero-based indexes, with 0 being the first element of the list
    /// (the head of the list), 1 being the next element and so on.
    ///
    /// These offsets can also be negative numbers indicating offsets starting at
    /// the end of the list. For example, -1 is the last element of the list, -2 the
    /// penultimate, and so on.
    /// ```
    /// LRANGE key start stop
    /// ```
    pub fn lrange(&mut self, mut args: VecDeque<RESP>) -> std::io::Result<()> {
        let err = || wrong_num_arguments("lrange");

        let mut store = self.store.lock().unwrap();
        let key = args.pop_front().ok_or(err())?.hashable()?;
        let list = store
            .kv
            .entry(key)
            .or_insert(Value::new_list())
            .list_mut()
            .unwrap();
        let n = list.len();

        let mut start: isize = args
            .pop_front()
            .ok_or(err())?
            .string()
            .ok_or(syntax_error())?
            .parse()
            .unwrap();
        let mut end: isize = args
            .pop_front()
            .ok_or(err())?
            .string()
            .ok_or(syntax_error())?
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

    /// Returns the length of the list stored at key. If key does not exist,
    /// it is interpreted as an empty list and 0 is returned. An error is
    /// returned when the value stored at key is not a list.
    /// ```
    /// LLEN key
    /// ```
    pub fn llen(&mut self, mut args: VecDeque<RESP>) -> std::io::Result<()> {
        let store = self.store.lock().unwrap();
        let key = args
            .pop_front()
            .ok_or(wrong_num_arguments("llen"))?
            .hashable()?;

        let n: RESP = match store.kv.get(&key).and_then(|v| v.list()) {
            Some(l) => l.len(),
            None => 0,
        }
        .into();

        write!(self.io, "{n}")
    }
}
