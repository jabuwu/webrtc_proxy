use std::{
    collections::VecDeque,
    io::{ErrorKind, Read, Write},
    net::{self, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
    time::Duration,
};

use anyhow::{bail, Result};
use enaia_client::EnaiaClient;
use rusty_enet::{crc32, Event, Host, HostSettings, Packet, PeerID, RangeCoder};
use web_time::Instant;

fn unspecified_address(address: SocketAddr) -> SocketAddr {
    if address.is_ipv4() {
        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0))
    } else {
        SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 0, 0, 0))
    }
}

pub struct Proxied {
    address: SocketAddr,
    protocol: &'static str,
    host: Host<EnaiaClient>,
    peer: PeerID,
    connect_time: Instant,
    connected: bool,
    disconnected: bool,
    packets: VecDeque<Vec<u8>>,
}

impl Proxied {
    pub fn connect(address: SocketAddr, proxy: String, protocol: &'static str) -> Result<Self> {
        let mut host = Host::<EnaiaClient>::create(
            EnaiaClient::new(),
            HostSettings {
                peer_limit: 1,
                channel_limit: 1,
                compressor: Some(Box::new(RangeCoder::new())),
                checksum: Some(Box::new(crc32)),
                ..Default::default()
            },
        )?;
        let peer = host.connect(proxy.into(), 1, 0)?.id();
        Ok(Self {
            address,
            protocol,
            host,
            peer,
            connect_time: Instant::now(),
            connected: false,
            disconnected: false,
            packets: VecDeque::new(),
        })
    }

    pub fn connected(&mut self, timeout: Duration) -> Result<bool> {
        self.service()?;
        if !self.connected && self.connect_time.elapsed() > timeout {
            self.disconnect();
            bail!("Connection timeout.");
        } else {
            Ok(self.connected)
        }
    }

    pub fn send(&mut self, data: &[u8]) -> Result<()> {
        self.service()?;
        if self.connected {
            if let Err(_) = self
                .host
                .peer_mut(self.peer)
                .and_then(|peer| peer.send(0, Packet::reliable(data)))
            {
                self.disconnect();
                bail!("Socket not connected.");
            }
        } else {
            bail!("Socket not connected.");
        }
        Ok(())
    }

    pub fn receive(&mut self) -> Result<Option<Vec<u8>>> {
        self.service()?;
        Ok(self.packets.pop_front())
    }

    fn service(&mut self) -> Result<()> {
        if self.disconnected {
            bail!("Disconnected.");
        }
        match self.host.service() {
            Ok(Some(Event::Connect { .. })) => {
                if let Err(_) = self.host.peer_mut(self.peer).and_then(|peer| {
                    peer.send(
                        0,
                        Packet::reliable(
                            format!("{{\"{}\":\"{}\"}}", self.protocol, self.address).as_bytes(),
                        ),
                    )
                }) {
                    self.disconnect();
                    bail!("Disconnected.");
                }
                Ok(())
            }
            Ok(Some(Event::Disconnect { .. })) => {
                self.disconnect();
                bail!("Disconnected.");
            }
            Ok(Some(Event::Receive {
                peer: _,
                channel_id,
                packet,
            })) => {
                if channel_id == 0 {
                    if let Some(first_byte) = packet.data().first() {
                        if *first_byte == 1 {
                            if !self.connected && packet.data().len() == 1 {
                                self.connected = true;
                            } else {
                                self.packets.push_back(packet.data()[1..].to_vec());
                            }
                            Ok(())
                        } else {
                            self.disconnect();
                            bail!("Disconnected.");
                        }
                    } else {
                        self.disconnect();
                        bail!("Disconnected.");
                    }
                } else {
                    Ok(())
                }
            }
            Ok(None) => Ok(()),
            Err(_) => {
                self.disconnect();
                bail!("Disconnected.");
            }
        }
    }

    fn disconnect(&mut self) {
        self.connected = false;
        self.disconnected = true;
        if let Ok(peer) = self.host.peer_mut(self.peer) {
            _ = peer.disconnect(0);
        }
    }
}

pub enum TcpStream {
    Direct(Option<net::TcpStream>),
    Proxied(Proxied),
}

impl TcpStream {
    pub fn connect(address: SocketAddr, proxy: Option<&str>) -> Result<Self> {
        if let Some(proxy) = proxy {
            Ok(Self::Proxied(Proxied::connect(
                address,
                proxy.to_owned(),
                "Tcp",
            )?))
        } else {
            let stream = net::TcpStream::connect(address)?;
            stream.set_nonblocking(true)?;
            Ok(Self::Direct(Some(stream)))
        }
    }

    pub fn connected(&mut self, timeout: Duration) -> Result<bool> {
        match self {
            Self::Direct(stream) => {
                if stream.is_some() {
                    Ok(true)
                } else {
                    bail!("Disconnected.");
                }
            }
            Self::Proxied(proxied) => proxied.connected(timeout),
        }
    }

    pub fn send(&mut self, data: &[u8]) -> Result<()> {
        match self {
            Self::Direct(stream) => {
                if let Some(stream) = stream {
                    match stream.write(data) {
                        Ok(sent) if sent == data.len() => Ok(()),
                        Ok(_) => {
                            self.disconnect();
                            bail!("Packet too large.");
                        }
                        Err(_) => {
                            self.disconnect();
                            bail!("Disconnected.");
                        }
                    }
                } else {
                    bail!("Disconnected.");
                }
            }
            Self::Proxied(proxied) => proxied.send(data),
        }
    }

    pub fn receive(&mut self) -> Result<Option<Vec<u8>>> {
        match self {
            Self::Direct(stream) => {
                if let Some(stream) = stream {
                    let mut buffer = [0; 4096];
                    match stream.read(&mut buffer) {
                        Ok(received) if received == 4096 => {
                            self.disconnect();
                            bail!("Packet too large.");
                        }
                        Ok(received) => Ok(Some(buffer[0..received].to_vec())),
                        Err(err) if err.kind() == ErrorKind::WouldBlock => Ok(None),
                        Err(_) => {
                            self.disconnect();
                            bail!("Disconnected.");
                        }
                    }
                } else {
                    bail!("Disconnected.");
                }
            }
            Self::Proxied(proxied) => proxied.receive(),
        }
    }

    fn disconnect(&mut self) {
        match self {
            Self::Direct(stream) => *stream = None,
            Self::Proxied(proxied) => proxied.disconnect(),
        }
    }
}

pub enum UdpSocket {
    Direct(Option<net::UdpSocket>),
    Proxied(Proxied),
}

impl UdpSocket {
    pub fn connect(address: SocketAddr, proxy: Option<&str>) -> Result<Self> {
        if let Some(proxy) = proxy {
            Ok(Self::Proxied(Proxied::connect(
                address,
                proxy.to_owned(),
                "Udp",
            )?))
        } else {
            let socket = net::UdpSocket::bind(unspecified_address(address))?;
            socket.connect(address)?;
            socket.set_nonblocking(true)?;
            Ok(Self::Direct(Some(socket)))
        }
    }

    pub fn connected(&mut self, timeout: Duration) -> Result<bool> {
        match self {
            Self::Direct(socket) => {
                if socket.is_some() {
                    Ok(true)
                } else {
                    bail!("Disconnected.");
                }
            }
            Self::Proxied(proxied) => proxied.connected(timeout),
        }
    }

    pub fn send(&mut self, data: &[u8]) -> Result<()> {
        match self {
            Self::Direct(socket) => {
                if let Some(socket) = socket {
                    match socket.send(data) {
                        Ok(sent) if sent == data.len() => Ok(()),
                        Ok(_) => {
                            self.disconnect();
                            bail!("Packet too large.");
                        }
                        Err(_) => {
                            self.disconnect();
                            bail!("Disconnected.");
                        }
                    }
                } else {
                    bail!("Disconnected.");
                }
            }
            Self::Proxied(proxied) => proxied.send(data),
        }
    }

    pub fn receive(&mut self) -> Result<Option<Vec<u8>>> {
        match self {
            Self::Direct(socket) => {
                if let Some(socket) = socket {
                    let mut buffer = [0; 4096];
                    match socket.recv(&mut buffer) {
                        Ok(received) if received == 4096 => {
                            self.disconnect();
                            bail!("Packet too large.");
                        }
                        Ok(received) => Ok(Some(buffer[0..received].to_vec())),
                        Err(err) if err.kind() == ErrorKind::WouldBlock => Ok(None),
                        Err(_) => {
                            self.disconnect();
                            bail!("Disconnected.");
                        }
                    }
                } else {
                    bail!("Disconnected.");
                }
            }
            Self::Proxied(proxied) => proxied.receive(),
        }
    }

    fn disconnect(&mut self) {
        match self {
            Self::Direct(socket) => *socket = None,
            Self::Proxied(proxied) => proxied.disconnect(),
        }
    }
}
