use std::{
    io::{ErrorKind, Read, Write},
    net::{SocketAddr, TcpStream},
};

use anyhow::{bail, Result};
use rusty_enet::Packet;

use crate::{ChannelStatus, ChannelStream};

pub struct TcpChannelStream(TcpStream);

impl TcpChannelStream {
    pub fn new(address: SocketAddr) -> Result<Self> {
        let stream = TcpStream::connect(address)?;
        stream.set_nonblocking(true)?;
        Ok(Self(stream))
    }
}

impl ChannelStream for TcpChannelStream {
    fn status(&mut self) -> Result<ChannelStatus> {
        Ok(ChannelStatus::Connected)
    }

    fn send(&mut self, packet: Packet) -> Result<()> {
        if self.0.write(packet.data())? == packet.data().len() {
            Ok(())
        } else {
            bail!("Packet too large.");
        }
    }

    fn receive(&mut self) -> Result<Option<Packet>> {
        let mut buffer = [0; 4096];
        match self.0.read(&mut buffer) {
            Ok(received) if received == 0 => bail!("Disconnected."),
            Ok(received) if received == 4096 => bail!("Packet too large."),
            Ok(received) => Ok(Some(Packet::reliable(&buffer[0..received]))),
            Err(err) if err.kind() == ErrorKind::WouldBlock => Ok(None),
            Err(err) => Err(err.into()),
        }
    }
}
