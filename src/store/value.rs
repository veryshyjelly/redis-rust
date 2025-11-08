use super::{StreamEntry, Value};
use crate::frame::Frame;
use ordered_float::OrderedFloat;
use std::collections::{BTreeMap, HashSet, VecDeque};

macro_rules! impl_getter {
    ($variant:ident, $tp:ty, $name:ident, $name_mut:ident) => {
        pub fn $name(&self) -> Option<&$tp> {
            if let Value::$variant(v) = self {
                Some(v)
            } else {
                None
            }
        }

        pub fn $name_mut(&mut self) -> Option<&mut $tp> {
            if let Value::$variant(v) = self {
                Some(v)
            } else {
                None
            }
        }
    };
}

impl Value {
    pub fn redis_type(&self) -> String {
        match self {
            Value::String(_) => "string",
            Value::List(_) => "list",
            Value::Set(_) => "set",
            Value::ZSet(_) => "zset",
            Value::Hash => "hash",
            Value::Stream(_) => "stream",
            Value::VectorSet => "vectorset",
        }
        .into()
    }

    impl_getter!(String, String, string, string_mut);
    impl_getter!(List, VecDeque<Frame>, list, list_mut);
    impl_getter!(Set, HashSet<Frame>, set, set_mut);
    impl_getter!(ZSet, BTreeMap<OrderedFloat<f64>, Frame>, zset, zset_mut);
    impl_getter!(Stream, Vec<StreamEntry>, stream, stream_mut);
}

macro_rules! impl_into_value {
    ($tp:ty => $variant:ident) => {
        impl Into<Value> for $tp {
            fn into(self) -> Value {
                Value::$variant(self)
            }
        }
    };
}

impl_into_value!(String => String);
impl_into_value!(VecDeque<Frame> => List);
impl_into_value!(HashSet<Frame> => Set);
impl_into_value!(BTreeMap<OrderedFloat<f64>, Frame> => ZSet);
impl_into_value!(Vec<StreamEntry> => Stream);
