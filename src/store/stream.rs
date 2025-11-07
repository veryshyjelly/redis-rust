use super::{StreamEntry, StreamEntryID};
use crate::frame::Frame;
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::time::{SystemTime, UNIX_EPOCH};

impl StreamEntryID {
    /// Create a new StreamEntryID for current time
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

impl Display for StreamEntryID {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.time, self.sqn)
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

impl Into<Frame> for StreamEntry {
    fn into(self) -> Frame {
        let mut res: Vec<Frame> = vec![];
        res.push(self.id.to_string().into());
        let mut kvs: Vec<Frame> = vec![];
        for (k, v) in &self.data {
            kvs.push(k.to_owned().into());
            kvs.push(v.to_owned().into());
        }
        res.push(kvs.into());
        res.into()
    }
}
