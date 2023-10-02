//
// driver.rs
//
// @author Natesh Narain <nnaraindev@gmail.com>
// @date Sep 29 2023
//
pub mod socketcan;

use socketcan::{SocketCanDriver, SocketCanDriverError};

use crate::frame::CanFrame;
use crate::DriverOpts;

use async_trait::async_trait;
use thiserror::Error;

/// Driver errors
#[derive(Error, Debug)]
pub enum DriverError {
    #[error("Error initializing socketcan driver: {0}")]
    SocketCanError(#[from] SocketCanDriverError),
}

#[async_trait]
pub trait AsyncCanDriver {
    /// Recieve CAN frame from the driver
    async fn recv(&mut self) -> Option<CanFrame>;
    /// Send CAN frame
    async fn send(&mut self, frame: CanFrame);
}
pub type AsyncCanDriverPtr = Box<dyn AsyncCanDriver + Sync + Send>;

impl TryFrom<DriverOpts> for AsyncCanDriverPtr {
    type Error = DriverError;

    fn try_from(value: DriverOpts) -> Result<Self, Self::Error> {
        match value {
            DriverOpts::SocketCan(can_interface) => SocketCanDriver::new(&can_interface)
                .map(|driver| upcast(Box::new(driver)))
                .map_err(|e| DriverError::SocketCanError(e)),
            DriverOpts::Udp(_, _) => unimplemented!(),
        }
    }
}

fn upcast<T: AsyncCanDriver + Sync + Send + 'static>(a: Box<T>) -> AsyncCanDriverPtr {
    a
}
