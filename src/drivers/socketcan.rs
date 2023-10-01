//
// socketcan.rs
//
// @author Natesh Narain <nnaraindev@gmail.com>
// @date Sep 29 2023
//

use crate::frame::CanFrame;
use crate::drivers::AsyncCanDriver;

use socketcan::{CanFrame as SocketCanFrame, tokio::CanSocket};
use embedded_can::Frame;
use async_trait::async_trait;
use thiserror::Error;
use futures_util::StreamExt;

use std::io;

impl From<SocketCanFrame> for CanFrame {
    fn from(value: SocketCanFrame) -> Self {
        // Using unwrap is fine since the socketcan frame already implements the same trait
        CanFrame::new(value.id(), value.data()).unwrap()
    }
}

impl From<CanFrame> for SocketCanFrame {
    fn from(value: CanFrame) -> Self {
        SocketCanFrame::new(value.id(), value.data()).unwrap()
    }
}


#[derive(Debug, Error)]
pub enum SocketCanDriverError {
    #[error("Failed to open CAN device")]
    OpenError(#[from] io::Error)
}

pub struct SocketCanDriver(CanSocket);

impl SocketCanDriver {
    pub fn new(can_interface: &str) -> Result<SocketCanDriver, SocketCanDriverError> {
        CanSocket::open(can_interface)
            .map(|socket| SocketCanDriver(socket))
            .map_err(|e| SocketCanDriverError::OpenError(e))
    }
}

#[async_trait]
impl AsyncCanDriver for SocketCanDriver {
    async fn recv(&mut self) -> Option<CanFrame> {
        self.0.next().await.map(|frame| frame.ok().map(|frame| frame.into())).flatten()
    }

    async fn send(&mut self, frame: CanFrame) {
        // TODO(nnarain): Error handling
        self.0.write_frame(frame.into()).unwrap().await.unwrap();
    }
}
