use std::net::SocketAddr;

use naia_server_socket::{NaiaServerSocketError, PacketReceiver, PacketSender, Socket};
use naia_socket_shared::{LinkConditionerConfig, SocketConfig};

pub use naia_server_socket::ServerAddrs;

pub struct EnaiaServer {
    packet_sender: Box<dyn PacketSender>,
    packet_receiver: Box<dyn PacketReceiver>,
}

impl rusty_enet::Socket for EnaiaServer {
    type BindAddress = ServerAddrs;
    type PeerAddress = SocketAddr;
    type Error = NaiaServerSocketError;

    fn bind(server_address: ServerAddrs) -> Result<Self, NaiaServerSocketError> {
        let (packet_sender, packet_receiver) = Socket::listen(
            &server_address,
            &SocketConfig::new(Some(LinkConditionerConfig::new(0, 0, 0.)), None),
        );
        Ok(EnaiaServer {
            packet_sender,
            packet_receiver,
        })
    }

    fn set_option(
        &mut self,
        _option: rusty_enet::SocketOption,
        _value: i32,
    ) -> Result<(), NaiaServerSocketError> {
        Ok(())
    }

    fn send(
        &mut self,
        address: Self::PeerAddress,
        buffer: &[u8],
    ) -> Result<usize, NaiaServerSocketError> {
        self.packet_sender.send(&address, buffer)?;
        Ok(buffer.len())
    }

    fn receive(
        &mut self,
        _mtu: usize,
    ) -> Result<Option<(Self::PeerAddress, Vec<u8>)>, NaiaServerSocketError> {
        match self.packet_receiver.receive() {
            Ok(Some((address, payload))) => Ok(Some((address, Vec::from(payload)))),
            Ok(None) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }
}
