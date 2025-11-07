use crate::Error;
use crate::frame::Frame;
use crate::frame::encode::AsBytes;
use crate::server::server::Server;
use crate::store::Store;
use bytes::{Buf, BytesMut};
use rand;
use std::io::Cursor;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

pub async fn handle(addr: Ipv4Addr, port: u16, store: Arc<Mutex<Store>>) -> Result<(), Error> {
    let mut tcp = TcpStream::connect(SocketAddrV4::new(addr, port)).await?;
    ping(&mut tcp).await.unwrap();
    replconf(store.clone(), &mut tcp).await.unwrap();
    let buffer = psync(store.clone(), &mut tcp).await.unwrap();
    Server::handle(store, tcp, buffer, rand::random_range(1..usize::MAX)).await;
    Ok(())
}

pub async fn ping(tcp: &mut TcpStream) -> Result<(), Error> {
    let mut b = BytesMut::new();
    let ping: Frame = vec!["PING".to_string()].into();
    ping.encode_bytes(&mut b);
    tcp.write_all(b.as_ref()).await?;
    b.clear();
    tcp.read_buf(&mut b).await?;
    assert_eq!(b.as_ref().to_ascii_lowercase(), b"+pong\r\n");
    Ok(())
}

pub async fn replconf(store: Arc<Mutex<Store>>, tcp: &mut TcpStream) -> Result<(), Error> {
    let mut b = BytesMut::new();
    // send first message
    let first_message: Frame = vec![
        "REPLCONF".into(),
        "listening-port".into(),
        store.lock().await.info.listening_port.to_string(),
    ]
    .into();
    first_message.encode_bytes(&mut b);
    tcp.write_all(b.as_ref()).await?;
    b.clear();
    // check ok response
    tcp.read_buf(&mut b).await?;
    assert_eq!(&b.to_ascii_lowercase(), b"+ok\r\n");
    b.clear();
    // send second message
    let second_message: Frame = vec!["REPLCONF", "capa", "psync2"].into();
    second_message.encode_bytes(&mut b);
    tcp.write_all(b.as_ref()).await?;
    b.clear();
    tcp.read_buf(&mut b).await?;
    assert_eq!(&b.to_ascii_lowercase(), b"+ok\r\n");
    Ok(())
}

pub async fn psync(store: Arc<Mutex<Store>>, tcp: &mut TcpStream) -> Result<BytesMut, Error> {
    let mut b = BytesMut::new();
    let repl_id = store.lock().await.info.master_id.clone();
    let offset = match store.lock().await.info.recv_offset {
        0 => -1,
        v => v as isize,
    };

    let message: Frame = vec!["PSYNC".into(), repl_id, offset.to_string()].into();
    message.encode_bytes(&mut b);
    tcp.write_all(b.as_ref()).await?;
    b.clear();

    while b.len() == 0 {
        tcp.read_buf(&mut b).await?;
    }
    let mut cursor = Cursor::new(b.as_ref());

    loop {
        assert_eq!(cursor.get_u8(), b'+');
        match crate::frame::decode::get_line(&mut cursor) {
            Ok(v) => {
                #[cfg(debug_assertions)]
                println!("psync-response: {}", String::from_utf8_lossy(v));
                break;
            }
            Err(crate::frame::Error::Incomplete) => {
                tcp.read_buf(&mut b).await?;
                cursor = Cursor::new(b.as_ref());
            }
            Err(e) => panic!("{e}"),
        }
    }

    let parsed = cursor.position() as usize;
    b.copy_within(parsed.., 0);
    b.truncate(b.len() - parsed);
    while b.len() == 0 {
        tcp.read_buf(&mut b).await?;
    }
    let mut cursor = Cursor::new(b.as_ref());

    let size = loop {
        assert_eq!(cursor.get_u8(), b'$');
        match crate::frame::decode::get_decimal(&mut cursor) {
            Ok(v) => break v,
            Err(crate::frame::Error::Incomplete) => {
                cursor = Cursor::new(b.as_ref());
            }
            Err(e) => panic!("{e}"),
        };
    } as usize;

    #[cfg(debug_assertions)]
    println!("size = {size}");

    let parsed = cursor.position() as usize;
    b.copy_within(parsed.., 0);
    b.truncate(b.len() - parsed);

    while b.len() < size {
        tcp.read_buf(&mut b).await?;
    }

    #[cfg(debug_assertions)]
    println!(
        "RDB file: {b:?}, \n hex: {}",
        hex::encode(&b.as_ref()[..size])
    );

    b.copy_within(size.., 0);
    b.truncate(b.len() - size);

    Ok(b)
}
