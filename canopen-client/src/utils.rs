//
// utils.rs
//
// @author Natesh Narain <nnaraindev@gmail.com>
// @date Aug 03 2022
//
use embedded_can::Id;

pub fn id_to_raw(id: &Id) -> u32 {
    match id {
        Id::Standard(id) => id.as_raw() as u32,
        Id::Extended(id) => id.as_raw(),
    }
}