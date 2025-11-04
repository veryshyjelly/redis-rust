use crate::resp::ReadWrite;
use std::io::{Read, Write};

pub struct DevNull;

impl Read for DevNull {
    fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
        Ok(0) // EOF immediately
    }
}

impl Write for DevNull {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(buf.len()) // pretend all bytes written
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl ReadWrite for DevNull {}
