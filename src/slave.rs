use crate::Error;
use crate::frame::Frame;
use crate::frame::encode::AsBytes;
use crate::server::server::Server;
use crate::store::Store;
use bytes::BytesMut;
use std::io::Cursor;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

pub async fn handle(addr: Ipv4Addr, port: u16, store: Arc<Mutex<Store>>) -> Result<(), Error> {
    let mut tcp = TcpStream::connect(SocketAddrV4::new(addr, port)).await?;
    ping(&mut tcp).await?;
    replconf(store.clone(), &mut tcp).await?;
    psync(store.clone(), &mut tcp).await?;
    Server::handle(tcp, store, true).await;
    Ok(())
}

pub async fn ping(tcp: &mut TcpStream) -> Result<(), Error> {
    let mut b = BytesMut::new();
    let ping: Frame = vec!["PING".to_string()].into();
    ping.encode_bytes(&mut b);
    tcp.write_all(b.as_ref()).await?;
    b.clear();
    tcp.read_buf(&mut b).await?;
    let response = String::from_utf8_lossy(b.as_ref());
    assert_eq!(response.to_lowercase(), "+pong");
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
    assert_eq!(&b.to_ascii_lowercase(), b"+ok\r\n");
    Ok(())
}

pub async fn psync(store: Arc<Mutex<Store>>, tcp: &mut TcpStream) -> Result<(), Error> {
    let mut b = BytesMut::new();
    let repl_id = store.lock().await.info.master_id.clone();
    let offset = store.lock().await.info.offset;

    let message: Frame = vec!["PSYNC".into(), repl_id, offset.to_string()].into();
    message.encode_bytes(&mut b);
    tcp.write_all(b.as_ref()).await?;
    b.clear();

    tcp.read_buf(&mut b).await?;
    let response = String::from_utf8_lossy(b.as_ref());
    #[cfg(debug_assertions)]
    print!("psync-response: {response:?}");
    b.clear();

    let size = loop {
        let mut cursor = Cursor::new(b.as_ref());
        match crate::frame::decode::get_decimal(&mut cursor) {
            Ok(v) => break v,
            Err(crate::frame::Error::Incomplete) => {
                tcp.read_buf(&mut b).await?;
                continue;
            }
            Err(e) => return Err(e.into()),
        };
    };

    b.clear();
    b.resize(size as usize, 0);
    tcp.read_exact(&mut b).await?;
    let rdb_file = b.as_ref();
    #[cfg(debug_assertions)]
    println!("rdb_file: {rdb_file:?}");
    b.clear();

    Ok(())
}
