use super::RDB;
use crate::Error;
use crate::store::Value;
use bytes::{Buf, Bytes};
use std::io::Cursor;
use std::path::PathBuf;
use std::time::{Duration, UNIX_EPOCH};

pub struct RDBParser<'a> {
    bytes: Cursor<&'a [u8]>,
}

impl<'a> RDBParser<'a> {
    pub fn parse_file(path: PathBuf) -> Result<RDB, Error> {
        let data = Bytes::from(std::fs::read(path)?);

        let mut parser = RDBParser {
            bytes: Cursor::new(data.as_ref()),
        };
        let mut rdb_file = RDB::default();
        rdb_file.header = parser.parse_header();
        let mut key_len = 0;
        let mut expiry_len = 0;

        while parser.bytes.has_remaining() {
            let section = parser.bytes.get_u8();
            match section {
                0xFA => {
                    let (k, v) = parser.parse_metadata();
                    rdb_file.metadata.insert(k, v);
                }
                0xFE => {
                    let idx = parser.bytes.get_u8();
                    if parser.bytes.get_ref()[parser.bytes.position() as usize] == 0xFB {
                        parser.bytes.advance(1);
                        key_len = parser.parse_encoded_length().unwrap();
                        expiry_len = parser.parse_encoded_length().unwrap();
                    }
                    for _ in 0..key_len {
                        let kv_type = parser.bytes.get_ref()[parser.bytes.position() as usize];
                        match kv_type {
                            0xFD => {
                                parser.bytes.advance(1);
                                // expiry time in seconds
                                let time = parser.bytes.get_u32_le();
                                let expiry = UNIX_EPOCH + Duration::from_secs(time as u64);
                                let (k, v) = parser.parse_key_value();
                                rdb_file.database.insert(k.clone(), v);
                                rdb_file.expiry_time.insert(k, expiry);
                            }
                            0xFC => {
                                parser.bytes.advance(1);
                                // expiry time in milliseconds
                                let time = parser.bytes.get_u64_le();
                                let expiry = UNIX_EPOCH + Duration::from_millis(time);
                                let (k, v) = parser.parse_key_value();
                                rdb_file.database.insert(k.clone(), v);
                                rdb_file.expiry_time.insert(k, expiry);
                            }
                            0x00 => {
                                let (k, v) = parser.parse_key_value();
                                rdb_file.database.insert(k.clone(), v);
                            }
                            _ => panic!("invalid or unimplemented"),
                        }
                    }
                }
                _ => continue,
            }
        }
        // println!("database: {:?}", rdb_file.database.keys());
        Ok(rdb_file)
    }

    fn parse_header(&mut self) -> String {
        let header = &self.bytes.get_ref()[..9];
        self.bytes.advance(9);
        String::from_utf8_lossy(header).into()
    }

    fn parse_metadata(&mut self) -> (String, String) {
        let key = self.parse_encoded_string();
        let value = self.parse_encoded_string();
        (key, value)
    }

    fn parse_db_fields(&mut self) {
        let idx = self.bytes.get_u8();
        // todo!()
    }

    fn parse_key_value(&mut self) -> (String, Value) {
        let value_type = self.bytes.get_u8();
        let key = self.parse_encoded_string();
        let value = match value_type {
            0x00 => self.parse_encoded_string().into(),
            _ => unimplemented!(),
        };
        (key, value)
    }

    fn parse_encoded_string(&mut self) -> String {
        let length = self.parse_encoded_length();
        match length {
            Ok(length) => {
                let position = self.bytes.position() as usize;
                let length = length.min(self.bytes.get_ref().len() - position);
                let data =
                    String::from_utf8_lossy(&self.bytes.get_ref()[position..position + length]);
                self.bytes.advance(length);
                data.into()
            }
            Err(encoded) => match encoded {
                0xC0 => self.bytes.get_u8().to_string(),
                0xC1 => self.bytes.get_u16().to_string(),
                0xC2 => self.bytes.get_u32().to_string(),
                _ => unimplemented!(),
            },
        }
    }

    fn parse_encoded_length(&mut self) -> Result<usize, usize> {
        let starting = self.bytes.get_u8();
        match starting >> 6 {
            0b00 => Ok(starting as usize),
            0b01 => {
                let first = (starting & 0x3F) as usize;
                let second = self.bytes.get_u8() as usize;
                Ok(first << 8 | second)
            }
            0b10 => Ok(self.bytes.get_u32() as usize),
            0b11 => Err(starting as usize),
            _ => panic!("not possible"),
        }
    }
}
