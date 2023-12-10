use std::{
    collections::HashMap,
    str::{self},
    time::Duration,
};

use anyhow::Result;
use enaia_server::{EnaiaServer, ServerAddrs};
use rusty_enet::{Event, Host, HostSettings, Packet, PeerID};

mod channel;
mod echo;
mod tcp;
mod udp;

pub use channel::*;
pub use echo::*;
pub use tcp::*;
pub use udp::*;

struct Tunnel {
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
    let mut network = Host::create(
        EnaiaServer::new(address).unwrap(),
        HostSettings {
            peer_limit: 4095,
            channel_limit: 255,
            ..Default::default()
        },
    )
    .unwrap();
    let mut tunnels = HashMap::<PeerID, Tunnel>::new();
    loop {
        while let Some(event) = network.service().unwrap() {
            match event {
                Event::Connect { peer, .. } => {
                    tunnels.insert(
                        peer.id(),
                        Tunnel {
                            channels: HashMap::default(),
                        },
                    );
                }
                Event::Disconnect { peer, .. } => {
                    tunnels.remove(&peer.id());
                }
                Event::Receive {
                    peer,
                    channel_id,
                    packet,
                } => {
                    if let Some(tunnel) = tunnels.get_mut(&peer.id()) {
                        if let Some(channel) = tunnel.channels.get_mut(&channel_id) {
                            if let Err(_) = channel.send(packet.data()) {
                                if let Err(_) = peer.send(channel_id, Packet::reliable(&[0])) {
                                    peer.disconnect(0);
                                }
                            }
                        } else {
                            match str::from_utf8(packet.data())
                                .ok()
                                .and_then(|str| serde_json::from_str::<ChannelConfig>(str).ok())
                            {
                                Some(channel_config) => {
                                    tunnel
                                        .channels
                                        .insert(channel_id, Channel::new(channel_config));
                                }
                                None => {
                                    if let Err(_) = peer.send(channel_id, Packet::reliable(&[0])) {
                                        peer.disconnect(0);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        for (peer_id, tunnel) in tunnels.iter_mut() {
            if let Ok(peer) = network.peer_mut(*peer_id) {
                let mut disconnected_channels = vec![];
                for (channel_id, channel) in tunnel.channels.iter_mut() {
                    if let Err(_) = || -> Result<()> {
                        while let Some(mut data) = channel.receive()? {
                            let mut packet_data = vec![1];
                            packet_data.append(&mut data);
                            peer.send(*channel_id, Packet::reliable(&packet_data))?;
                        }
                        Ok(())
                    }() {
                        if let Err(_) = peer.send(*channel_id, Packet::reliable(&[0])) {
                            peer.disconnect(0);
                        }
                        disconnected_channels.push(*channel_id);
                    }
                }
                tunnel
                    .channels
                    .retain(|channel_id, _| !disconnected_channels.contains(channel_id));
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
