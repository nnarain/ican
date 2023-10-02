//
// format.rs
//
// @author Natesh Narain <nnaraindev@gmail.com>
// @date Oct 01 2023
//

use crate::{frame::CanFrame, utils};

use std::fmt;

use embedded_can::{Frame, Id};

#[derive(Debug, Clone, Copy)]
pub enum DataFormatMode {
    Hex,
    Binary,
}

/// Data for formatting a CAN frame
pub struct CanFrameFormatter {
    frame: CanFrame,
    data_format_mode: DataFormatMode,
}

impl From<(CanFrame, DataFormatMode)> for CanFrameFormatter {
    fn from(value: (CanFrame, DataFormatMode)) -> Self {
        CanFrameFormatter {
            frame: value.0,
            data_format_mode: value.1,
        }
    }
}

impl fmt::Display for CanFrameFormatter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let id = utils::id_to_raw(&self.frame.id());
        let id = match self.frame.id() {
            Id::Standard(_) => format!("{:03X}", id),
            Id::Extended(_) => format!("{:08X}", id),
        };

        let dlc = self.frame.dlc();
        let data_string =
            self.frame
                .data()
                .iter()
                .fold(String::from(""), |a, b| match self.data_format_mode {
                    DataFormatMode::Hex => format!("{} {:02X}", a, b),
                    DataFormatMode::Binary => format!("{} {:08b}", a, b),
                });

        write!(f, "{} [{}] {}", id, dlc, data_string)
    }
}
