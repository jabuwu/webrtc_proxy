use std::{
    io::ErrorKind,
    net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6, UdpSocket},
};

use anyhow::{bail, Result};
use rusty_enet::Packet;

use crate::{ChannelStatus, ChannelStream};

pub struct UdpChannelStream(UdpSocket);

impl UdpChannelStream {
    pub fn new(address: SocketAddr) -> Result<Self> {
        let socket = if address.is_ipv4() {
            UdpSocket::bind(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)))
        } else {
            UdpSocket::bind(SocketAddr::V6(SocketAddrV6::new(
                Ipv6Addr::UNSPECIFIED,
                0,
                0,
                0,
            )))
        }?;
        socket.connect(address)?;
        socket.set_nonblocking(true)?;
        Ok(Self(socket))
    }
}

impl ChannelStream for UdpChannelStream {
    fn status(&mut self) -> Result<ChannelStatus> {
        Ok(ChannelStatus::Connected)
    }

    fn send(&mut self, packet: Packet) -> Result<()> {
        if self.0.send(packet.data())? == packet.data().len() {
            Ok(())
        } else {
            bail!("Packet too large.");
        }
    }

    fn receive(&mut self) -> Result<Option<Packet>> {
        let mut buffer = [0; 4096];
        match self.0.recv(&mut buffer) {
            Ok(received) if received == 0 => bail!("Disconnected."),
            Ok(received) if received == 4096 => bail!("Packet too large."),
            Ok(received) => {
                dbg!(received);
                Ok(Some(Packet::unreliable_unsequenced(&buffer[0..received])))
            }
            Err(err) if err.kind() == ErrorKind::WouldBlock => Ok(None),
            Err(err) => Err(err.into()),
        }
    }
}
