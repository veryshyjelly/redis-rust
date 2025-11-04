use super::Redis;
use super::Command;
use super::errors::{out_of_range, syntax_error, wrong_num_arguments, wrong_type};
use super::value::Value;
use crate::resp::{TypedNone, RESP};
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
    pub fn set(&mut self, mut args: Command) -> std::io::Result<RESP> {
        let err = || wrong_num_arguments("set");
        let mut store = self.store.lock().unwrap();
        let key  = args.pop_front().ok_or(err())?;
        let value = args.pop_front().ok_or(wrong_num_arguments("set"))?;
        store.kv.insert(key.clone(), value.into());

        if args.len() > 0 {
            let unit = args
                .pop_front()
                .ok_or(err())?;
            let mut time = args
                .pop_front()
                .ok_or(err())?
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

        Ok("OK".into())
    }

    /// Get the value of key. If the key does not exist the special value
    /// nil is returned. An error is returned if the value stored at key
    /// is not a string, because GET only handles string values.
    /// ```
    /// GET key
    /// ```
    pub fn get(&mut self, mut args: Command) -> std::io::Result<RESP> {
        self.remove_expired();
        let store = self.store.lock().unwrap();

        let key = args
            .pop_front()
            .ok_or(wrong_num_arguments("get"))?;
        
        let resp = if let Some(v) = store.kv.get(&key) {
             v.string().ok_or(wrong_type())?.clone().into()
        } else {
            RESP::None(TypedNone::String)
        };
        
        Ok(resp)
    }

    /// Increments the number stored at key by one. If the key does not exist, 
    /// it is set to 0 before performing the operation. An error is returned if 
    /// the key contains a value of the wrong type or contains a string that can 
    /// not be represented as integer. This operation is limited to 64 bit signed integers.
    /// ```
    /// INCR key
    /// ```
    pub fn incr(&mut self, mut args: Command) -> std::io::Result<RESP> {
        let key = args
            .pop_front()
            .ok_or(wrong_num_arguments("incr"))?;
        let mut store = self.store.lock().unwrap();
        let val = store
            .kv
            .entry(key)
            .or_insert(Value::String("0".into()))
            .string_mut()
            .ok_or(out_of_range())?;
        let mut result_value: isize = val.parse().ok()
            .ok_or(out_of_range())?;
        result_value += 1;
        
        // let v = .string().ok_or(out_of_range())?
        *val = result_value.to_string();
        Ok(result_value.into())
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
