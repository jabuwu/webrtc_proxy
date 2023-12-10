use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use anyhow::Result;
use rusty_enet::Packet;

use crate::{ChannelStatus, ChannelStream};

pub struct EchoChannelStream {
    instant: Instant,
    packets: VecDeque<Packet>,
}

impl EchoChannelStream {
    pub fn new() -> Self {
        Self {
            instant: Instant::now(),
            packets: VecDeque::new(),
        }
    }
}

impl ChannelStream for EchoChannelStream {
    fn status(&mut self) -> Result<ChannelStatus> {
        if self.instant.elapsed() > Duration::from_secs(3) {
            Ok(ChannelStatus::Disconnected)
        } else {
            Ok(ChannelStatus::Connected)
        }
    }

    fn send(&mut self, packet: Packet) -> Result<()> {
        self.packets.push_back(packet);
        Ok(())
    }

    fn receive(&mut self) -> Result<Option<Packet>> {
        Ok(self.packets.pop_front())
    }
}
