//
// frame.rs
//
// @author Natesh Narain <nnaraindev@gmail.com>
// @date Sep 29 2023
//

use embedded_can::{Frame, Id};

/// Independent CAN frame type common to all drivers
#[derive(Debug, Clone)]
pub struct CanFrame {
    id: Id,
    is_extended: bool,
    is_remote: bool,
    dlc: usize,
    data: [u8; 8],
}

impl Frame for CanFrame {
    fn new(id: impl Into<Id>, data: &[u8]) -> Option<Self> {
        if data.len() <= 8 {
            let id: Id = id.into();

            let mut payload = [0u8; 8];
            payload[..data.len()].copy_from_slice(&data[..]);

            Some(CanFrame { id, is_extended: matches!(id, Id::Extended(_)), is_remote: false, dlc: data.len(), data: payload })
        }
        else {
            None
        }
    }

    fn new_remote(id: impl Into<Id>, dlc: usize) -> Option<Self> {
        if dlc <= 8 {
            Some(CanFrame{id: id.into(), is_extended: false, is_remote: true, dlc, data: [0u8; 8]})
        }
        else {
            None
        }
    }

    fn id(&self) -> Id {
        self.id
    }

    fn is_extended(&self) -> bool {
        self.is_extended
    }

    fn is_remote_frame(&self) -> bool {
        self.is_remote
    }

    fn dlc(&self) -> usize {
        self.dlc
    }

    fn data(&self) -> &[u8] {
        &self.data[..self.dlc]
    }
}
