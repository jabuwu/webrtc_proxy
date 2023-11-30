use std::{net::SocketAddr, str, str::FromStr, time::Duration};

use webrtc_proxy_client::TcpStream;

fn main() {
    let mut socket = TcpStream::connect(
        SocketAddr::from_str("34.238.34.226:80").unwrap(),
        Some("http://127.0.0.1:14191"),
    )
    .unwrap();
    while !socket.connected(Duration::from_secs(3)).unwrap() {
        std::thread::sleep(Duration::from_millis(100));
    }
    socket
        .send(
            "GET / HTTP/1.1\r\nHost: checkip.amazonaws.com\r\nUser-Agent: curl/7.79.1\r\nAccept: */*\r\n\r\n"
                .as_bytes(),
        )
        .unwrap();
    loop {
        if let Some(packet) = socket.receive().unwrap() {
            println!("{}", str::from_utf8(&packet).unwrap());
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
