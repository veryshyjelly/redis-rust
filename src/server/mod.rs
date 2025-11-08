use crate::Error;
use crate::frame::Frame;
use std::collections::VecDeque;

mod errors;
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
