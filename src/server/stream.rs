use crate::frame::TypedNone;
use super::Result;
use super::errors::*;
use super::server::Server;
use crate::HashMap;
use crate::frame::Frame;
use crate::server::Args;
use crate::store::StreamEntry;
use crate::store::StreamEntryID;
use crate::store::Value;

impl Server {
    /// Appends the specified stream entry to the stream at the specified key.
    /// If the key does not exist, XADD will create a new key with the given stream
    /// value as a side effect of running this command.
    /// ```
    /// XADD key [NOMKSTREAM] [KEEPREF | DELREF | ACKED] [<MAXLEN | MINID>
    ///   [= | ~] threshold [LIMIT count]] <* | id> field value [field value ...]
    /// ```
    pub async fn xadd(&mut self, mut args: Args) -> Result {
        let err = || wrong_num_arguments("xadd");

        let key = args.pop_front().ok_or(err())?;
        let mut store = self.store.lock().await;
        let stream = store
            .kv
            .entry(key)
            .or_insert(Value::Stream(vec![]))
            .stream_mut()
            .ok_or(wrong_type())?;
        let id = args.pop_front().ok_or(err())?;
        let mut data = HashMap::new();
        while args.len() > 0 {
            let key = args.pop_front().ok_or(err())?;
            let value = args.pop_front().ok_or(err())?;
            data.insert(key, value);
        }

        let id = if id == "*" {
            StreamEntryID::new()
        } else if id.contains("*") {
            // <milliseconds>-*
            let time: usize = id
                .split("-")
                .nth(0)
                .ok_or(syntax_error())?
                .parse()
                .map_err(|_| syntax_error())?;
            let mut id = StreamEntryID::with_time(time);
            if let Some(x) = stream.last()
                && x.id.time == time
            {
                id.sqn = x.id.sqn + 1;
            }
            id
        } else {
            StreamEntryID::explicit(id)
        };

        let entry = StreamEntry { id, data };
        if id == (StreamEntryID { time: 0, sqn: 0 }) {
            Err("ERR The ID specified in XADD must be greater than 0-0".into())
        } else if stream.is_empty() || &entry > stream.last().unwrap() {
            stream.push(entry);
            Ok(id.to_string().into())
        } else {
            Err(
                "ERR The ID specified in XADD is equal or smaller than the target stream top item"
                    .into(),
            )
        }
    }

    /// Removes the specified entries from a stream, and returns the number of entries
    /// deleted. This number may be less than the number of IDs passed to the command
    /// in the case where some of the specified IDs do not exist in the stream.
    pub async fn xdel(&mut self, mut args: Args) -> Result {
        let mut store = self.store.lock().await;
        let key = args.pop_front().ok_or(wrong_num_arguments("xdel"))?;

        let stream = store
            .kv
            .entry(key)
            .or_insert(Value::Stream(vec![]))
            .stream_mut()
            .ok_or(wrong_type())?;

        let mut res = 0usize;
        for id in args {
            if let Some(idx) = stream.iter().position(|x| x.id.to_string() == id) {
                stream.remove(idx);
                res += 1;
            }
        }
        Ok(res.into())
    }

    /// Returns the number of entries inside a stream. If the specified key does not
    /// exist the command returns zero, as if the stream was empty. However note that
    /// unlike other Redis types, zero-length streams are possible, so you should call
    /// TYPE or EXISTS in order to check if a key exists or not.
    /// ```
    /// XLEN key
    /// ```
    pub async fn xlen(&mut self, mut args: Args) -> Result {
        let store = self.store.lock().await;
        let key = args.pop_front().ok_or(wrong_num_arguments("xlen"))?;

        let n = match store.kv.get(&key).and_then(|v| v.stream()) {
            Some(l) => l.len(),
            None => 0,
        };

        Ok(n.into())
    }

    /// The command returns the stream entries matching a given range of IDs.
    /// The range is specified by a minimum and maximum ID. All the entries having an
    /// ID between the two specified or exactly one of the two IDs specified (closed interval) are returned.
    /// ```
    /// XRANGE key start end [COUNT count]
    /// ```
    pub async fn xrange(&mut self, mut args: Args) -> Result {
        let err = || wrong_num_arguments("xrange");
        let key = args.pop_front().ok_or(err())?;
        let mut store = self.store.lock().await;
        let stream = store
            .kv
            .entry(key)
            .or_insert(Value::Stream(vec![]))
            .stream_mut()
            .ok_or(make_io_error(
                "WRONGTYPE Operation against a key holding the wrong kind of value",
            ))?;

        let start = args.pop_front().ok_or(err())?;
        let end = args.pop_front().ok_or(err())?;

        let start = if start == "-" {
            0
        } else {
            let id = StreamEntryID::implicit(start);
            stream.partition_point(|x| x.id < id)
        };

        let end = if end == "+" {
            stream.len()
        } else {
            let id = StreamEntryID::implicit(end);
            stream.partition_point(|x| x.id <= id)
        };

        let res = stream.get(start..end).unwrap_or_default().to_vec();
        let resp: Frame = res
            .into_iter()
            .map(|v| v.into())
            .collect::<Vec<Frame>>()
            .into();

        Ok(resp)
    }

    /// Read data from one or multiple streams, only returning entries with an ID greater
    /// than the last received ID reported by the caller. This command has an option to block
    /// if items are not available
    /// ```
    /// XREAD [COUNT count] [BLOCK milliseconds] STREAMS key [key ...] id [id ...]
    /// ```
    pub async fn xread(&mut self, mut args: Args) -> Result {
        let err = || wrong_num_arguments("xread");

        let method = args.pop_front().ok_or(err())?.to_lowercase();

        let mut time_out: u128 = if method == "block" {
            let timeout_value = args
                .pop_front()
                .ok_or(err())?
                .parse()
                .map_err(|_| syntax_error())?;
            let _streams = args.pop_front().ok_or(err())?;
            timeout_value
        } else {
            1
        };

        if time_out == 0 {
            time_out = u128::MAX;
        }

        let now = std::time::Instant::now();

        let stream_count = args.len() / 2;
        let keys: Vec<String> = args.drain(0..stream_count).collect();
        let mut starts = vec![];

        for key in &keys {
            let start = args.pop_front().ok_or(err())?;
            let start = if start == "$" {
                let mut store = self.store.lock().await;
                let stream = store
                    .kv
                    .entry(key.clone())
                    .or_insert(Value::Stream(vec![]))
                    .stream_mut()
                    .ok_or(make_io_error(
                        "WRONGTYPE Operation against a key holding the wrong kind of value",
                    ))?;
                stream
                    .last()
                    .map(|v| v.id)
                    .unwrap_or(StreamEntryID { time: 0, sqn: 0 })
            } else if start == "-" {
                StreamEntryID { time: 0, sqn: 0 }
            } else {
                StreamEntryID::implicit(start)
            };
            starts.push(start)
        }

        while now.elapsed().as_millis() < time_out {
            let mut store = self.store.lock().await;

            let mut result: Vec<Frame> = vec![];
            for (key, start) in keys.iter().zip(starts.iter()) {
                let stream = store
                    .kv
                    .entry(key.clone())
                    .or_insert(Value::Stream(vec![]))
                    .stream_mut()
                    .ok_or(make_io_error(
                        "WRONGTYPE Operation against a key holding the wrong kind of value",
                    ))?;
                let start = stream.partition_point(|x| &x.id <= start);
                if stream[start..].len() == 0 {
                    continue;
                }
                let resp: Frame = stream
                    .get(start..)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|v| v.clone().into())
                    .collect::<Vec<Frame>>()
                    .into();
                result.push(vec![key.clone().into(), resp].into())
            }

            if result.len() != 0 {
                return Ok(result.into());
            }
        }

        Ok(Frame::None(TypedNone::Array))
    }
}
