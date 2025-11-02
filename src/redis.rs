use std::io::{Read, Write};
use std::net::TcpStream;

pub trait ReadWrite: Read + Write {}

impl ReadWrite for TcpStream {}

pub struct Redis {
    io: Box<dyn ReadWrite>
}

impl Redis {
    pub fn new(io: Box<dyn ReadWrite>) -> Self {
        Redis { io }
    }
    
    pub fn handle(&mut self) -> std::io::Result<()> {
        let mut buf = vec![0; 1024];
        let mut read_bytes = 0;
        let mut parsed_bytes = 0;
        
        loop {
            if read_bytes == buf.len() {
                let mut buuf = vec![0; buf.len() * 2];
                buuf[0..buf.len()].copy_from_slice(&buf);
                buf = buuf;
            }
            
            let n = self.io.read(&mut buf[read_bytes..])?;
            read_bytes += n;

            let parsed = self.parse(&buf[..read_bytes])?;
            read_bytes -= parsed;
            
            let v = buf[parsed..read_bytes+parsed].to_vec(); 
            buf[0..read_bytes].copy_from_slice(&v);
            
            parsed_bytes += parsed;
        }
    }
    
    fn parse(&mut self, data: &[u8]) -> std::io::Result<usize> {
        self.io.write("+PONG\r\n".as_bytes())?;
        Ok(data.len())
    }
}