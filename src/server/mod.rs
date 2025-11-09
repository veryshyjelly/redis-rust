use crate::frame::Frame;
use crate::Error;
use std::collections::VecDeque;

mod errors;
mod geospatial;
mod list;
mod misc;
mod persistence;
mod pubsub;
mod replication;
pub mod server;
mod stream;
mod string;
mod transaction;
mod zset;

type Result = std::result::Result<Frame, Error>;

pub type Args = VecDeque<String>;
