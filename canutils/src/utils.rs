//
// utils.rs
//
// @author Natesh Narain <nnaraindev@gmail.com>
// @date Jul 19 2022
//

use embedded_hal::can::{Frame, Id, StandardId, ExtendedId};

// impl From<Id> for u32 {
//     fn from(_: Id) -> Self {
//         0
//     }
// }

pub fn id_to_raw<F: Frame>(frame: &F) -> u32 {
    match frame.id() {
        Id::Standard(id) => id.as_raw() as u32,
        Id::Extended(id) => id.as_raw(),
    }
}
