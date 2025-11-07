#![allow(unused)]

use std::io::Error;

pub fn make_io_error(message: &str) -> Error {
    Error::new(std::io::ErrorKind::InvalidData, message)
}

pub fn wrong_type() -> Error {
    make_io_error("WRONGTYPE Operation against a key holding the wrong kind of value")
}

pub fn syntax_error() -> Error {
    make_io_error("ERR syntax error")
}

pub fn not_integer() -> Error {
    make_io_error("ERR value is not an integer")
}

pub fn out_of_range() -> Error {
    make_io_error("ERR value is not an integer or out of range")
}

pub fn wrong_num_arguments(cmd: &str) -> Error {
    make_io_error(&format!(
        "ERR wrong number of arguments for '{cmd}' command"
    ))
}
