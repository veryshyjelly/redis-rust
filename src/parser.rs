use crate::frame::Frame;
use bytes::BytesMut;
use std::io::Cursor;
use tokio::io::{AsyncRead, AsyncReadExt};

pub struct Parser {
    input: Box<dyn AsyncRead + Unpin + Send>,
    buffer: BytesMut,
    pub parsed_bytes: usize,
}

impl Parser {
    pub fn new(input: Box<dyn AsyncRead + Unpin + Send>, buffer: BytesMut) -> Self {
        Parser {
            input,
            buffer,
            parsed_bytes: 0,
        }
    }

    /// Read a single `Frame` value from the underlying stream.
    ///
    /// The function waits until it has retrieved enough data to parse a frame.
    /// Any data remaining in the read buffer after the frame has been parsed is
    /// kept there for the next call to `read_frame`.
    ///
    /// # Returns
    ///
    /// On success, the received frame is returned. If the `TcpStream`
    /// is closed in a way that doesn't break a frame in half, it returns
    /// `None`. Otherwise, an error is returned.
    pub async fn read_frame(&mut self) -> Result<Option<Frame>, crate::Error> {
        loop {
            // Attempt to parse a frame from the buffered data. If enough data
            // has been buffered, the frame is returned.
            if let Some(frame) = self.parse_frame()? {
                return Ok(Some(frame));
            }

            // There is not enough buffered data to read a frame. Attempt to
            // read more data from the socket.
            //
            // On success, the number of bytes is returned. `0` indicates "end
            // of stream".
            if 0 == self.input.read_buf(&mut self.buffer).await? {
                // The remote closed the connection. For this to be a clean
                // shutdown, there should be no data in the read buffer. If
                // there is, this means that the peer closed the socket while
                // sending a frame.
                return if self.buffer.is_empty() {
                    Ok(None)
                } else {
                    Err("connection reset by peer".into())
                };
            }
        }
    }

    pub fn parse_frame(&mut self) -> Result<Option<Frame>, crate::Error> {
        use crate::frame::Error::Incomplete;
        let mut buf = Cursor::new(&self.buffer[..]);
        match Frame::parse(&mut buf) {
            Ok(v) => {
                let parsed = buf.position() as usize;
                self.buffer.copy_within(parsed.., 0);
                self.buffer.truncate(self.buffer.len() - parsed);
                self.parsed_bytes = parsed;
                Ok(Some(v))
            }
            Err(Incomplete) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}
