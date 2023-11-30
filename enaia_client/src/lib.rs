use naia_client_socket::{NaiaClientSocketError, PacketReceiver, PacketSender, Socket};
use naia_socket_shared::{LinkConditionerConfig, SocketConfig};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnaiaUrl(pub String);

impl From<String> for EnaiaUrl {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for EnaiaUrl {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl rusty_enet::Address for EnaiaUrl {
    fn same_host(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn same(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn is_broadcast(&self) -> bool {
        false
    }
}

pub enum EnaiaClient {
    Disconnected,
    Connected {
        server_address: EnaiaUrl,
        packet_sender: Box<dyn PacketSender>,
        packet_receiver: Box<dyn PacketReceiver>,
    },
}

impl rusty_enet::Socket for EnaiaClient {
    type BindAddress = ();
    type PeerAddress = EnaiaUrl;
    type Error = NaiaClientSocketError;

    fn bind(_: ()) -> Result<Self, NaiaClientSocketError> {
        Ok(EnaiaClient::Disconnected)
    }

    fn set_option(
        &mut self,
        _option: rusty_enet::SocketOption,
        _value: i32,
    ) -> Result<(), NaiaClientSocketError> {
        Ok(())
    }

    fn send(
        &mut self,
        address: Self::PeerAddress,
        buffer: &[u8],
    ) -> Result<usize, NaiaClientSocketError> {
        if matches!(self, EnaiaClient::Disconnected) {
            let (packet_sender, packet_receiver) = Socket::connect(
                &address.0,
                &SocketConfig::new(Some(LinkConditionerConfig::new(0, 0, 0.)), None),
            );
            *self = EnaiaClient::Connected {
                server_address: address,
                packet_sender,
                packet_receiver,
            }
        }

        if let EnaiaClient::Connected {
            server_address: _,
            packet_sender,
            packet_receiver: _,
        } = self
        {
            packet_sender.send(buffer)?;
            Ok(buffer.len())
        } else {
            Err(NaiaClientSocketError::SendError)
        }
    }

    fn receive(
        &mut self,
        _mtu: usize,
    ) -> Result<Option<(Self::PeerAddress, Vec<u8>)>, NaiaClientSocketError> {
        if let EnaiaClient::Connected {
            server_address,
            packet_sender: _,
            packet_receiver,
        } = self
        {
            match packet_receiver.receive() {
                Ok(Some(payload)) => Ok(Some((server_address.clone(), Vec::from(payload)))),
                Ok(None) => Ok(None),
                Err(err) => Err(err.into()),
            }
        } else {
            Ok(None)
        }
    }
}
