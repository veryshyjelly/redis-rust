use super::server::Server;
use super::{Args, Result};
use crate::frame::{Frame, TypedNone};
use crate::server::errors::{syntax_error, wrong_num_arguments, wrong_type};
use crate::store::Value;
use ordered_float::OrderedFloat;
use std::collections::BTreeMap;
use std::str::FromStr;

impl Server {
    /// Adds all the specified members with the specified scores to the sorted set
    /// stored at key. It is possible to specify multiple score / member pairs.
    /// If a specified member is already a member of the sorted set, the score is updated
    /// and the element reinserted at the right position to ensure the correct ordering.
    ///
    /// If key does not exist, a new sorted set with the specified members as sole
    /// members is created, like if the sorted set was empty. If the key exists but
    /// does not hold a sorted set, an error is returned.
    ///
    /// The score values should be the string representation of a double precision
    /// floating point number. +inf and -inf values are valid values as well.
    /// ```
    /// ZADD key [NX | XX] [GT | LT] [CH] [INCR] score member [score member ...]
    /// ```
    pub async fn zadd(&mut self, mut args: Args) -> Result {
        let mut store = self.store.lock().await;
        let key = args.pop_front().ok_or(wrong_num_arguments("zadd"))?;
        let set = store
            .kv
            .entry(key)
            .or_insert(Value::ZSet(BTreeMap::new()))
            .zset_mut()
            .ok_or(wrong_type())?;
        let mut res = 0usize;
        while !args.is_empty() {
            let k = args.pop_front().ok_or(syntax_error())?;
            let key = OrderedFloat::from_str(&k).map_err(|_| syntax_error())?;
            let value = args.pop_front().ok_or(syntax_error())?;
            set.insert(key, value.into());
            res += 1;
        }
        Ok(res.into())
    }

    /// Returns the sorted set cardinality (number of elements) of the sorted set stored at key.
    /// ```
    /// ZCARD key
    /// ```
    pub async fn zcard(&mut self, mut args: Args) -> Result {
        let store = self.store.lock().await;
        let key = args.pop_front().ok_or(wrong_num_arguments("zadd"))?;
        let card = store
            .kv
            .get(&key)
            .and_then(|v| v.zset())
            .map(|v| v.len())
            .unwrap_or(0);
        Ok(card.into())
    }

    /// Returns the number of elements in the sorted set at key with a score between min and max.
    ///
    /// The min and max arguments have the same semantic as described for ZRANGEBYSCORE.
    /// ```
    /// ZCOUNT key min max
    /// ```
    pub async fn zcount(&mut self, mut args: Args) -> Result {
        let store = self.store.lock().await;
        let key = args.pop_front().ok_or(wrong_num_arguments("zadd"))?;
        let min = OrderedFloat::from_str(&args.pop_front().ok_or(wrong_num_arguments("zadd"))?)?;
        let max = OrderedFloat::from_str(&args.pop_front().ok_or(wrong_num_arguments("zadd"))?)?;
        let count = store
            .kv
            .get(&key)
            .and_then(|v| v.zset())
            .map(|v| v.range(min..max).count())
            .unwrap_or(0);
        Ok(count.into())
    }

    /// Returns the rank of member in the sorted set stored at key, with the scores ordered
    /// from low to high. The rank (or index) is 0-based, which means that the member with the
    /// lowest score has rank 0.
    /// ```
    /// ZRANK key member [WITHSCORE]
    /// ```
    pub async fn zrank(&mut self, mut args: Args) -> Result {
        let store = self.store.lock().await;
        let key = args.pop_front().ok_or(wrong_num_arguments("zadd"))?;
        let member = args.pop_front().ok_or(wrong_num_arguments("zadd"))?;
        let set = store.kv.get(&key).unwrap().zset().ok_or(wrong_type())?;
        if let Some((k, _)) = set
            .iter()
            .find(|(_, v)| (*v).clone().string().map(|v| v == member).unwrap_or(false))
        {
            Ok(set.range(..k).count().into())
        } else {
            Ok(Frame::None(TypedNone::Nil))
        }
    }

    /// Returns the specified range of elements in the sorted set stored at <key>.
    /// ZRANGE can perform different types of range queries: by index (rank), by the score,
    /// or by lexicographical order.
    /// ```
    /// ZRANGE key start stop [BYSCORE | BYLEX] [REV] [LIMIT offset count] [WITHSCORES]
    /// ```
    pub async fn zrange(&mut self, mut args: Args) -> Result {
        let store = self.store.lock().await;
        let key = args.pop_front().ok_or(wrong_num_arguments("zadd"))?;
        let start = OrderedFloat::from_str(&args.pop_front().ok_or(wrong_num_arguments("zadd"))?)?;
        let stop = OrderedFloat::from_str(&args.pop_front().ok_or(wrong_num_arguments("zadd"))?)?;
        let set = store.kv.get(&key).unwrap().zset().ok_or(wrong_type())?;
        Ok(set
            .range(start..stop)
            .map(|(_, v)| (*v).clone().string().unwrap_or("".into()))
            .collect::<Vec<String>>()
            .into())
    }

    /// Removes the specified members from the sorted set stored at key. Non existing members are ignored.
    ///
    /// An error is returned when key exists and does not hold a sorted set.
    /// ```
    /// ZREM key member [member ...]
    /// ```
    pub async fn zrem(&mut self, mut args: Args) -> Result {
        todo!()
    }

    /// Returns the score of member in the sorted set at key.
    ///
    /// If member does not exist in the sorted set, or key does not exist, nil is returned.
    /// ```
    /// ZSCORE key member
    /// ```
    pub async fn zscore(&mut self, mut args: Args) -> Result {
        todo!()
    }
}
