use super::Redis;
use crate::redis::errors::{syntax_error, wrong_num_arguments, wrong_type};
use crate::resp::RESP;
use std::collections::VecDeque;
use std::ops::Add;
use std::time::Duration;

impl Redis {
    /// Set key to hold the string value. If key already holds a value,
    /// it is overwritten, regardless of its type. Any previous time to
    /// live associated with the key is discarded on successful SET operation.
    /// ```
    /// SET key value [NX | XX] [GET] [EX seconds | PX milliseconds |
    ///   EXAT unix-time-seconds | PXAT unix-time-milliseconds | KEEPTTL]
    /// ```
    pub fn set(&mut self, mut args: VecDeque<RESP>) -> std::io::Result<()> {
        let err = || wrong_num_arguments("set");
        let mut store = self.store.lock().unwrap();
        let key = args.pop_front().ok_or(err())?.hashable()?;
        let value = args.pop_front().ok_or(wrong_num_arguments("set"))?;
        store.kv.insert(key.clone(), value.into());

        if args.len() > 0 {
            let unit = args
                .pop_front()
                .ok_or(err())?
                .string()
                .ok_or(syntax_error())?;
            let mut time = args
                .pop_front()
                .ok_or(err())?
                .string()
                .ok_or(syntax_error())?
                .parse()
                .map_err(|_| syntax_error())?;
            if unit.to_lowercase() == "ex" {
                time *= 1000;
            }
            let expiry_time = std::time::Instant::now().add(Duration::from_millis(time));
            store.expiry_queue.insert(expiry_time, key.clone());
            store.expiry_time.insert(key, expiry_time);
        } else {
            if let Some(time) = store.expiry_time.remove(&key) {
                store.expiry_queue.remove(&time);
            }
        }

        let resp: RESP = "OK".into();
        write!(self.io, "{resp}")
    }

    /// Get the value of key. If the key does not exist the special value
    /// nil is returned. An error is returned if the value stored at key
    /// is not a string, because GET only handles string values.
    /// ```
    /// GET key
    /// ```
    pub fn get(&mut self, mut args: VecDeque<RESP>) -> std::io::Result<()> {
        self.remove_expired();
        let store = self.store.lock().unwrap();

        let key = args
            .pop_front()
            .ok_or(wrong_num_arguments("get"))?
            .hashable()?;
        if let Some(v) = store.kv.get(&key) {
            let resp = v.string().ok_or(wrong_type())?;
            write!(self.io, "{resp}")
        } else {
            write!(self.io, "{}", RESP::null_bulk_string())
        }
    }

    /// Removes expired keys from the kv store
    /// keys are stored in heap wrt their expiration time
    pub fn remove_expired(&mut self) {
        let mut store = self.store.lock().unwrap();
        while !store.expiry_queue.is_empty() {
            let (t, key) = match store.expiry_queue.pop_first() {
                Some(v) => v,
                None => break,
            };
            if t > std::time::Instant::now() {
                store.expiry_queue.insert(t, key);
                break;
            }
            store.expiry_time.remove(&key);
            store.kv.remove(&key);
        }
    }
}
