//
// utils.rs
//
// @author Natesh Narain <nnaraindev@gmail.com>
// @date Jul 19 2022
//

use embedded_hal::can::Id;


pub fn id_to_raw(id: &Id) -> u32 {
    match id {
        Id::Standard(id) => id.as_raw() as u32,
        Id::Extended(id) => id.as_raw(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_id_standard() {
        let id: Id = StandardId::new(0x1F1).expect("Failed to created ID").into();
        assert_eq!(id_to_raw(&id), 0x1F1u32)
    }

    #[test]
    fn check_id_extended() {
        let id: Id = ExtendedId::new(0x1F1).expect("Failed to created ID").into();
        assert_eq!(id_to_raw(&id), 0x1F1u32)
    }
}
