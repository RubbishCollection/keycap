use std::net::SocketAddr;

use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufStream},
    net::{TcpListener, TcpStream},
};

use std::io::Cursor;

async fn handle_connection(stream: TcpStream) {
    let mut stream = BufStream::new(stream);
    let mut vec = Vec::new();

    loop {
        let mut buf = Vec::new();

        stream.read_until(b'\r', &mut buf).await.unwrap();

        let mut check = [0u8; 3];
        stream.read_exact(&mut check).await.unwrap();

        vec.append(&mut buf);
        vec.append(&mut check.to_vec());

        if check == [b'\n', b'\r', b'\n'] {
            break;
        }
    }

    let mut headers = [httparse::EMPTY_HEADER; 16];
    let mut req = httparse::Request::new(&mut headers);

    req.parse(vec.as_mut_slice()).unwrap();

    let remote = match TcpStream::connect(req.path.unwrap()).await {
        Ok(remote) => {
            stream
                .write_all_buf(&mut Cursor::new("HTTP/1.1 200 Connection established\r\n\r\n"))
                .await
                .unwrap();
            stream.flush().await.unwrap();
            remote
        }
        Err(_) => return,
    };

    let (mut remote_read, mut remote_write) = remote.into_split();
    let (mut stream_read, mut stream_write) = stream.into_inner().into_split();


    tokio::spawn(async move {
        loop {
            let mut buf = [0u8; 1024];
            let n = remote_read.read(&mut buf).await.unwrap();
            stream_write.write(&buf[0..n]).await;
        }
    });

    loop {
        let mut buf = [0u8; 1024];
        let n = stream_read.read(&mut buf).await.unwrap();
        remote_write.write(&buf[0..n]).await;
    }
    
}

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([127, 0, 0, 1], 5333));
    let listener = TcpListener::bind(addr)
        .await
        .expect("Fail to bind listener");

    let mut handles = Vec::new();
    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                handles.push(tokio::spawn(handle_connection(stream)));
            }
            Err(e) => println!("Connection Failed: {}", e),
        }
    }
}
