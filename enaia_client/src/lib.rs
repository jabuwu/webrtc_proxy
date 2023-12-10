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

impl EnaiaClient {
    pub fn new() -> Self {
        Self::Disconnected
    }
}

impl rusty_enet::Socket for EnaiaClient {
    type PeerAddress = EnaiaUrl;
    type Error = NaiaClientSocketError;

    fn init(&mut self, _options: rusty_enet::SocketOptions) -> Result<(), NaiaClientSocketError> {
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
    ) -> Result<Option<(Self::PeerAddress, rusty_enet::PacketReceived)>, NaiaClientSocketError>
    {
        if let EnaiaClient::Connected {
            server_address,
            packet_sender: _,
            packet_receiver,
        } = self
        {
            match packet_receiver.receive() {
                Ok(Some(payload)) => Ok(Some((
                    server_address.clone(),
                    rusty_enet::PacketReceived::Complete(Vec::from(payload)),
                ))),
                Ok(None) => Ok(None),
                Err(err) => Err(err.into()),
            }
        } else {
            Ok(None)
        }
    }
}
