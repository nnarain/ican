//
// lib.rs
//
// @author Natesh Narain <nnaraindev@gmail.com>
// @date Aug 03 2022
//
#![cfg_attr(not(test), no_std)]
#[deny(missing_docs)]

mod utils;

use embedded_can::Frame;

#[derive(Debug)]
pub enum Error {
    InvalidNmtState,
    HasExtendedId,
    InvalidChannel(u32),
}

/// CANopen Node ID
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeId(u8);

impl From<u8> for NodeId {
    fn from(node: u8) -> Self {
        NodeId(node)
    }
}

impl NodeId {
    pub fn as_raw(&self) -> u8 {
        self.0
    }
}

/// Network Managment State
#[derive(Debug, Clone, Copy)]
pub enum NmtState {
    BootUp,
    Stopped,
    Operational,
    PreOperational,
}

impl TryFrom<u8> for NmtState {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(NmtState::BootUp),
            0x04 => Ok(NmtState::Stopped),
            0x05 => Ok(NmtState::Operational),
            0x7F => Ok(NmtState::PreOperational),
            _ => Err(Error::InvalidNmtState),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pdo {
    Tx1, Rx1,
    Tx2, Rx2,
    Tx3, Rx3,
    Tx4, Rx4,
}

#[derive(Debug, Clone, Copy)]
pub enum Sdo {
    Tx, Rx
}

#[derive(Debug, Clone, Copy)]
pub struct FrameData {
    pub data: [u8; 8],
    pub len: u8,
}

impl FrameData {
    pub fn new(data: &[u8], len: usize) -> Self {
        let mut pdo_data = [0u8; 8];
        let num = pdo_data.len().min(data.len());

        pdo_data[..num].copy_from_slice(&data[..num]);

        Self {
            data: pdo_data,
            len: len as u8,
        }
    }
}

/// Parsed CANopen data
#[derive(Debug, Clone, Copy)]
pub enum CanOpenFrame {
    Sync,
    Heartbeat(NmtState),
    Pdo(Pdo, FrameData),
    Sdo(Sdo, FrameData),
}

impl CanOpenFrame {
    pub fn is_sync(&self) -> bool {
        matches!(*self, CanOpenFrame::Sync)
    }

    pub fn is_heartbeat(&self) -> bool {
        matches!(*self, CanOpenFrame::Heartbeat(_))
    }

    pub fn is_pdo(&self) -> bool {
        matches!(*self, CanOpenFrame::Pdo(_, _))
    }

    pub fn is_sdo(&self) -> bool {
        matches!(*self, CanOpenFrame::Sdo(_, _))
    }
}

pub fn parse<F: Frame>(frame: F) -> Result<(Option<NodeId>, CanOpenFrame), Error> {
    let id = utils::id_to_raw(&frame.id());

    let channel = id & !(0x7F);
    let node_id = NodeId((id & 0x7F) as u8);

    match channel {
        0x080 => Ok((None, CanOpenFrame::Sync)),
        0x700 => Ok((Some(node_id), CanOpenFrame::Heartbeat(NmtState::try_from(frame.data()[0])?))),
        0x180 => Ok((Some(node_id), CanOpenFrame::Pdo(Pdo::Tx1, FrameData::new(frame.data(), frame.dlc())))),
        0x200 => Ok((Some(node_id), CanOpenFrame::Pdo(Pdo::Rx1, FrameData::new(frame.data(), frame.dlc())))),
        0x280 => Ok((Some(node_id), CanOpenFrame::Pdo(Pdo::Tx2, FrameData::new(frame.data(), frame.dlc())))),
        0x300 => Ok((Some(node_id), CanOpenFrame::Pdo(Pdo::Rx2, FrameData::new(frame.data(), frame.dlc())))),
        0x380 => Ok((Some(node_id), CanOpenFrame::Pdo(Pdo::Tx3, FrameData::new(frame.data(), frame.dlc())))),
        0x400 => Ok((Some(node_id), CanOpenFrame::Pdo(Pdo::Rx3, FrameData::new(frame.data(), frame.dlc())))),
        0x480 => Ok((Some(node_id), CanOpenFrame::Pdo(Pdo::Tx4, FrameData::new(frame.data(), frame.dlc())))),
        0x500 => Ok((Some(node_id), CanOpenFrame::Pdo(Pdo::Rx4, FrameData::new(frame.data(), frame.dlc())))),
        0x580 => Ok((Some(node_id), CanOpenFrame::Sdo(Sdo::Tx, FrameData::new(frame.data(), frame.dlc())))),
        0x600 => Ok((Some(node_id), CanOpenFrame::Sdo(Sdo::Rx, FrameData::new(frame.data(), frame.dlc())))),
        _ => Err(Error::InvalidChannel(channel)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use embedded_hal::can::{Id, StandardId};

    struct TestFrame {
        id: Id,
        data: [u8; 8],
        dlc: usize,
    }

    impl Frame for TestFrame {
        fn data(&self) -> &[u8] {
            &self.data[..]
        }

        fn dlc(&self) -> usize {
            self.dlc
        }

        fn id(&self) -> embedded_hal::can::Id {
            self.id
        }

        fn is_remote_frame(&self) -> bool {
            false
        }

        fn is_extended(&self) -> bool {
            false
        }

        fn new(id: impl Into<embedded_hal::can::Id>, data: &[u8]) -> Option<Self> {
            let mut frame_data: [u8; 8] = [0; 8];
            frame_data[0..data.len()].copy_from_slice(data);

            Some(
                TestFrame {
                    id: id.into(),
                    data: frame_data,
                    dlc: data.len(),
                }
            )
        }

        fn new_remote(_id: impl Into<Id>, _dlc: usize) -> Option<Self> {
            None
        }
    }

    #[test]
    fn convert_nmt_state() {
        matches!(NmtState::try_from(0x00), Ok(NmtState::BootUp));
        matches!(NmtState::try_from(0x04), Ok(NmtState::Stopped));
        matches!(NmtState::try_from(0x05), Ok(NmtState::Operational));
        matches!(NmtState::try_from(0x7F), Ok(NmtState::PreOperational));
    }

    #[test]
    #[should_panic]
    fn convert_nmt_state_fail() {
        NmtState::try_from(0x11).unwrap();
    }

    #[test]
    fn parse_sync() {
        let frame = TestFrame::new(StandardId::new(0x080).unwrap(), &[]).unwrap();
        let (node, frame) = parse(frame).unwrap();
        
        assert!(node.is_none());
        assert!(frame.is_sync());
    }

    #[test]
    fn parse_nmt() {
        let frame = TestFrame::new(StandardId::new(0x705).unwrap(), &[0x05]).unwrap();
        let (node, frame) = parse(frame).unwrap();

        assert!(matches!(node, Some(NodeId(0x05))));
        assert!(matches!(frame, CanOpenFrame::Heartbeat(NmtState::Operational)));
    }

    #[test]
    fn parse_tpdo1() {
        let frame = TestFrame::new(StandardId::new(0x181).unwrap(), &[0x01, 0x02, 0x03, 0x04]).unwrap();
        let (node, frame) = parse(frame).unwrap();

        assert!(matches!(node, Some(NodeId(0x01))));
        assert!(
            matches!(
                frame,
                CanOpenFrame::Pdo(Pdo::Tx1, FrameData { data: [0x01, 0x02, 0x03, 0x04, 0x00, 0x00, 0x00, 0x00], len: 4 })
            )
        );
    }

    #[test]
    fn parse_tpdo2() {
        let frame = TestFrame::new(StandardId::new(0x281).unwrap(), &[0x01, 0x02, 0x03, 0x04]).unwrap();
        let (node, frame) = parse(frame).unwrap();

        assert!(matches!(node, Some(NodeId(0x01))));
        assert!(
            matches!(
                frame,
                CanOpenFrame::Pdo(Pdo::Tx2, FrameData { data: [0x01, 0x02, 0x03, 0x04, 0x00, 0x00, 0x00, 0x00], len: 4 })
            )
        );
    }

    #[test]
    fn parse_tpdo3() {
        let frame = TestFrame::new(StandardId::new(0x381).unwrap(), &[0x01, 0x02, 0x03, 0x04]).unwrap();
        let (node, frame) = parse(frame).unwrap();

        assert!(matches!(node, Some(NodeId(0x01))));
        assert!(
            matches!(
                frame,
                CanOpenFrame::Pdo(Pdo::Tx3, FrameData { data: [0x01, 0x02, 0x03, 0x04, 0x00, 0x00, 0x00, 0x00], len: 4 })
            )
        );
    }

    #[test]
    fn parse_tpdo4() {
        let frame = TestFrame::new(StandardId::new(0x481).unwrap(), &[0x01, 0x02, 0x03, 0x04]).unwrap();
        let (node, frame) = parse(frame).unwrap();

        assert!(matches!(node, Some(NodeId(0x01))));
        assert!(
            matches!(
                frame,
                CanOpenFrame::Pdo(Pdo::Tx4, FrameData { data: [0x01, 0x02, 0x03, 0x04, 0x00, 0x00, 0x00, 0x00], len: 4 })
            )
        );
    }

    #[test]
    fn parse_rpdo1() {
        let frame = TestFrame::new(StandardId::new(0x201).unwrap(), &[0x01, 0x02, 0x03, 0x04]).unwrap();
        let (node, frame) = parse(frame).unwrap();

        assert!(matches!(node, Some(NodeId(0x01))));
        assert!(
            matches!(
                frame,
                CanOpenFrame::Pdo(Pdo::Rx1, FrameData { data: [0x01, 0x02, 0x03, 0x04, 0x00, 0x00, 0x00, 0x00], len: 4 })
            )
        );
    }

    #[test]
    fn parse_rpdo2() {
        let frame = TestFrame::new(StandardId::new(0x301).unwrap(), &[0x01, 0x02, 0x03, 0x04]).unwrap();
        let (node, frame) = parse(frame).unwrap();

        assert!(matches!(node, Some(NodeId(0x01))));
        assert!(
            matches!(
                frame,
                CanOpenFrame::Pdo(Pdo::Rx2, FrameData { data: [0x01, 0x02, 0x03, 0x04, 0x00, 0x00, 0x00, 0x00], len: 4 })
            )
        );
    }

    #[test]
    fn parse_rpdo3() {
        let frame = TestFrame::new(StandardId::new(0x401).unwrap(), &[0x01, 0x02, 0x03, 0x04]).unwrap();
        let (node, frame) = parse(frame).unwrap();

        assert!(matches!(node, Some(NodeId(0x01))));
        assert!(
            matches!(
                frame,
                CanOpenFrame::Pdo(Pdo::Rx3, FrameData { data: [0x01, 0x02, 0x03, 0x04, 0x00, 0x00, 0x00, 0x00], len: 4 })
            )
        );
    }

    #[test]
    fn parse_rpdo4() {
        let frame = TestFrame::new(StandardId::new(0x501).unwrap(), &[0x01, 0x02, 0x03, 0x04]).unwrap();
        let (node, frame) = parse(frame).unwrap();

        assert!(matches!(node, Some(NodeId(0x01))));
        assert!(
            matches!(
                frame,
                CanOpenFrame::Pdo(Pdo::Rx4, FrameData { data: [0x01, 0x02, 0x03, 0x04, 0x00, 0x00, 0x00, 0x00], len: 4 })
            )
        );
    }

    #[test]
    fn frame_data_variable_length() {
        let raw_data: [u8; 2] = [0x01, 0x02];
        let frame_data = FrameData::new(&raw_data[..], 2);

        assert_eq!(frame_data.data, [0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    }
}