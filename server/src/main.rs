use std::{
    collections::HashMap,
    str::{self},
    time::Duration,
};

use anyhow::Result;
use enaia_server::{EnaiaServer, ServerAddrs};
use rusty_enet::{Event, Host, Packet, PeerID};

mod channel;
mod echo;
mod tcp;
mod udp;

pub use channel::*;
pub use echo::*;
pub use tcp::*;
pub use udp::*;

struct Peer {
    channels: HashMap<u8, Channel>,
}

fn main() {
    let address = ServerAddrs::new(
        "0.0.0.0:14191"
            .parse()
            .expect("could not parse Session address/port"),
        "0.0.0.0:14192"
            .parse()
            .expect("could not parse WebRTC data address/port"),
        "http://127.0.0.1:14192",
    );
    let mut network = Host::<EnaiaServer>::create(address, 4095, 255, 0, 0).unwrap();
    let mut peers = HashMap::<PeerID, Peer>::new();
    loop {
        while let Some(event) = network.service().unwrap() {
            match &event {
                Event::Connect { peer, .. } => {
                    peers.insert(
                        *peer,
                        Peer {
                            channels: HashMap::default(),
                        },
                    );
                }
                Event::Disconnect { peer, .. } => {
                    peers.remove(peer);
                }
                Event::Receive {
                    peer: peer_id,
                    channel_id,
                    packet,
                } => {
                    if let Some(peer) = peers.get_mut(peer_id) {
                        if let Some(channel) = peer.channels.get_mut(channel_id) {
                            if let Err(_) = channel.send(packet.data()) {
                                if let Err(_) =
                                    network.send(*peer_id, *channel_id, Packet::reliable(&[0]))
                                {
                                    _ = network.disconnect(*peer_id, 0);
                                }
                            }
                        } else {
                            match str::from_utf8(packet.data())
                                .ok()
                                .and_then(|str| serde_json::from_str::<ChannelConfig>(str).ok())
                            {
                                Some(channel_config) => {
                                    peer.channels
                                        .insert(*channel_id, Channel::new(channel_config));
                                }
                                None => {
                                    if let Err(_) =
                                        network.send(*peer_id, *channel_id, Packet::reliable(&[0]))
                                    {
                                        _ = network.disconnect(*peer_id, 0);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        for (peer_id, peer) in peers.iter_mut() {
            let mut disconnected_channels = vec![];
            for (channel_id, channel) in peer.channels.iter_mut() {
                if let Err(_) = || -> Result<()> {
                    while let Some(mut data) = channel.receive()? {
                        let mut packet_data = vec![1];
                        packet_data.append(&mut data);
                        network.send(*peer_id, *channel_id, Packet::reliable(&packet_data))?;
                    }
                    Ok(())
                }() {
                    if let Err(_) = network.send(*peer_id, *channel_id, Packet::reliable(&[0])) {
                        _ = network.disconnect(*peer_id, 0);
                    }
                    disconnected_channels.push(*channel_id);
                }
            }
            peer.channels
                .retain(|channel_id, _| !disconnected_channels.contains(channel_id));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
