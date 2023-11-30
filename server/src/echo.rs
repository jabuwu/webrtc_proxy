use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use anyhow::Result;

use crate::{ChannelStatus, ChannelStream};

pub struct EchoChannelStream {
    instant: Instant,
    packets: VecDeque<Vec<u8>>,
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

    fn send(&mut self, data: &[u8]) -> Result<()> {
        self.packets.push_back(data.to_vec());
        Ok(())
    }

    fn receive(&mut self) -> Result<Option<Vec<u8>>> {
        Ok(self.packets.pop_front())
    }
}
