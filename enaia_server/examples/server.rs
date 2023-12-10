use std::time::Duration;

use enaia_server::EnaiaServer;
use naia_server_socket::ServerAddrs;
use rusty_enet::{Event, Host, HostSettings};

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
    let mut host = Host::create(
        EnaiaServer::new(server_address).unwrap(),
        HostSettings {
            peer_limit: 32,
            channel_limit: 2,
            ..Default::default()
        },
    )
    .unwrap();
    loop {
        while let Some(event) = host.service().unwrap() {
            match event {
                Event::Connect { peer, .. } => {
                    println!("Peer {} connected", peer.id().0);
                }
                Event::Disconnect { peer, .. } => {
                    println!("Peer {} disconnected", peer.id().0);
                }
                Event::Receive {
                    peer,
                    channel_id,
                    packet,
                    ..
                } => {
                    _ = peer.send(channel_id, packet.clone());
                }
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
