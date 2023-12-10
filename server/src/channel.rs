use std::{
    net::SocketAddr,
    str,
    sync::mpsc::{self, RecvTimeoutError},
    time::Duration,
};

use anyhow::{bail, Result};
use rusty_enet::Packet;
use serde::{Deserialize, Serialize};

use crate::{EchoChannelStream, TcpChannelStream, UdpChannelStream};

#[derive(Debug, Serialize, Deserialize)]
pub enum ChannelConfig {
    Echo,
    Tcp(SocketAddr),
    Udp(SocketAddr),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelStatus {
    Connecting,
    Connected,
    Disconnected,
}

pub struct Channel {
    sender: mpsc::Sender<Packet>,
    receiver: mpsc::Receiver<Packet>,
}

impl Channel {
    pub fn new(config: ChannelConfig) -> Self {
        let (sender, channel_receiver) = mpsc::channel::<Packet>();
        let (channel_sender, receiver) = mpsc::channel::<Packet>();
        std::thread::spawn(move || {
            let Ok(mut channel) = || -> Result<Box<dyn ChannelStream>> {
                Ok(match config {
                    ChannelConfig::Echo => Box::new(EchoChannelStream::new()),
                    ChannelConfig::Tcp(address) => Box::new(TcpChannelStream::new(address)?),
                    ChannelConfig::Udp(address) => Box::new(UdpChannelStream::new(address)?),
                })
            }() else {
                return;
            };
            let mut connected = false;
            loop {
                if let Err(_) = || -> Result<()> {
                    match channel.status()? {
                        ChannelStatus::Connecting => {}
                        ChannelStatus::Connected => {
                            if !connected {
                                sender.send(Packet::reliable(&[]))?;
                                connected = true;
                            }
                            match receiver.recv_timeout(Duration::ZERO) {
                                Ok(packet) => {
                                    channel.send(packet)?;
                                }
                                Err(RecvTimeoutError::Timeout) => {}
                                Err(_) => bail!("Disconnected"),
                            }
                            while let Some(data) = channel.receive()? {
                                sender.send(data)?;
                            }
                        }
                        ChannelStatus::Disconnected => {
                            bail!("Disconnected");
                        }
                    }
                    std::thread::sleep(Duration::from_millis(10));
                    Ok(())
                }() {
                    break;
                }
            }
        });
        Channel {
            sender: channel_sender,
            receiver: channel_receiver,
        }
    }

    pub fn send(&mut self, packet: Packet) -> Result<()> {
        self.sender.send(packet)?;
        Ok(())
    }

    pub fn receive(&mut self) -> Result<Option<Packet>> {
        match self.receiver.recv_timeout(Duration::ZERO) {
            Ok(packet) => Ok(Some(packet)),
            Err(RecvTimeoutError::Timeout) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }
}

pub trait ChannelStream {
    fn status(&mut self) -> Result<ChannelStatus>;
    fn send(&mut self, data: Packet) -> Result<()>;
    fn receive(&mut self) -> Result<Option<Packet>>;
}
