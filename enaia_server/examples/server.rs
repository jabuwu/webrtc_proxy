use std::time::Duration;

use enaia_server::EnaiaServer;
use naia_server_socket::ServerAddrs;
use rusty_enet::{Event, Host};

fn main() {
    let server_address = ServerAddrs::new(
        "127.0.0.1:14191"
            .parse()
            .expect("could not parse Session address/port"),
        "127.0.0.1:14192"
            .parse()
            .expect("could not parse WebRTC data address/port"),
        "http://127.0.0.1:14192",
    );
    let mut host = Host::<EnaiaServer>::create(server_address, 32, 2, 0, 0).unwrap();
    loop {
        while let Some(event) = host.service().unwrap() {
            match &event {
                Event::Connect { peer, .. } => {
                    println!("Peer {} connected", peer.0);
                }
                Event::Disconnect { peer, .. } => {
                    println!("Peer {} disconnected", peer.0);
                }
                Event::Receive {
                    peer,
                    channel_id,
                    packet,
                    ..
                } => {
                    _ = host.send(*peer, *channel_id, packet.clone());
                }
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
