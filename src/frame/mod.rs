mod debug;
pub mod decode;
pub(crate) mod encode;
mod frame;

use bytes::Bytes;
use std::collections::{HashMap, HashSet};

#[derive(Clone)]
pub enum Frame {
    SimpleString(String),               // +
    BulkString(Bytes),                  // $
    SimpleError(String),                // -
    BulkError(Bytes),                   // !
    Integer(isize),                     // :
    Array(Vec<Frame>),                  // *
    Boolean(bool),                      // #
    Double(f64),                        // ,
    BigNumber(String),                  // (
    VerbatimString((String, String)),   // =
    Map(HashMap<String, Frame>),        // %
    Attributes(HashMap<String, Frame>), // |
    Set(HashSet<String>),               // ~
    Push(Vec<Frame>),                   // >
    RDB(Bytes),                         // $
    None(TypedNone),                    // _
}

#[derive(Copy, Clone)]
pub enum TypedNone {
    String,
    Array,
    Nil,
}

#[derive(Debug)]
pub enum Error {
    /// Not enough data is available to parse a message
    Incomplete,

    /// Invalid message encoding
    Other(crate::Error),
}
