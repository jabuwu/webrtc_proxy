use std::{
    net::SocketAddr,
    str,
    sync::mpsc::{self, RecvTimeoutError},
    time::Duration,
};

use anyhow::{bail, Result};
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
    sender: mpsc::Sender<Vec<u8>>,
    receiver: mpsc::Receiver<Vec<u8>>,
}

impl Channel {
    pub fn new(config: ChannelConfig) -> Self {
        let (sender, channel_receiver) = mpsc::channel::<Vec<u8>>();
        let (channel_sender, receiver) = mpsc::channel::<Vec<u8>>();
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
                                sender.send(vec![])?;
                                connected = true;
                            }
                            match receiver.recv_timeout(Duration::ZERO) {
                                Ok(data) => {
                                    channel.send(&data)?;
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

    pub fn send(&mut self, data: &[u8]) -> Result<()> {
        self.sender.send(data.to_vec())?;
        Ok(())
    }

    pub fn receive(&mut self) -> Result<Option<Vec<u8>>> {
        match self.receiver.recv_timeout(Duration::ZERO) {
            Ok(data) => Ok(Some(data)),
            Err(RecvTimeoutError::Timeout) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }
}

pub trait ChannelStream {
    fn status(&mut self) -> Result<ChannelStatus>;
    fn send(&mut self, data: &[u8]) -> Result<()>;
    fn receive(&mut self) -> Result<Option<Vec<u8>>>;
}
