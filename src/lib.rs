//! # syxpack
//!
//! `syxpack` is a collection of helpers for processing MIDI System Exclusive messages.

use std::collections::HashMap;
use std::fmt;
use log::debug;
use bit::BitIndex;
use lazy_static::lazy_static;

/// Manufacturer specific SysEx message initiator.
pub const INITIATOR: u8 = 0xf0;

/// Manufacturer specific SysEx message terminator.
pub const TERMINATOR: u8 = 0xf7;

/// Universal non-real-time SysEx message indicator.
pub const NON_REAL_TIME: u8 = 0x7e;

/// Universal real-time SysEx message indicator.
pub const REAL_TIME: u8 = 0x7f;

/// MIDI manufacturer ID. Either a single byte for standard or three bytes for extended IDs.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub enum ManufacturerId {
    Standard(u8),
    Extended([u8; 3]),
}

impl fmt::Display for ManufacturerId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let hex = match self {
            ManufacturerId::Standard(b) => format!("{:X}", b),
            ManufacturerId::Extended(bs) => format!("{:X} {:X} {:X} ", bs[0], bs[1], bs[2]),
        };
        write!(f, "{}", hex)
    }
}

/// The kind of a Universal System Exclusive message.
pub enum UniversalKind {
    NonRealTime,
    RealTime,
}

/// A MIDI System Exclusive message.
pub enum Message {
    Universal(UniversalKind, u8, u8),
    ManufacturerExclusive(Manufacturer, Vec<u8>),
}

impl Message {
    /// Creates a manufacturer-specific SysEx message.
    pub fn new(manufacturer: Manufacturer, payload: Vec<u8>) -> Self {
        Message::ManufacturerExclusive(manufacturer, payload)
    }

    /// Creates a new universal SysEx message with the given sub-IDs.
    pub fn new_universal(kind: UniversalKind, sub_id1: u8, sub_id2: u8) -> Self {
        Message::Universal(kind, sub_id1, sub_id2)
    }

    /// Converts the message into bytes for MIDI messaging.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::<u8>::new();

        match self {
            Message::Universal(kind, sub_id1, sub_id2) => {
                result.push(match kind {
                    UniversalKind::NonRealTime => NON_REAL_TIME,
                    UniversalKind::RealTime => REAL_TIME,
                });
                result.push(*sub_id1);
                result.push(*sub_id2);
            },
            Message::ManufacturerExclusive(manufacturer, payload) => {
                result.push(INITIATOR);
                result.extend(manufacturer.to_bytes());
                result.extend(payload);
                result.push(TERMINATOR);
            }
        }

        result
    }
}

/// Group of manufacturer.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub enum ManufacturerGroup {
    American,
    EuropeanOrOther,
    Japanese,
    Other,
}

/// MIDI equipment manufacturer.
pub struct Manufacturer {
    id: ManufacturerId,
    display_name: String,
    canonical_name: String,
    group: ManufacturerGroup,
}

impl Manufacturer {
    pub fn from_id(id: ManufacturerId) -> Self {
        if let Some(manufacturer) = crate::MANUFACTURERS.get(&id) {
            Manufacturer {
                id: manufacturer.id,
                display_name: manufacturer.display_name.clone(),
                canonical_name: manufacturer.canonical_name.clone(),
                group: manufacturer.group,
            }
        }
        else {
            panic!("Unknown manufacturer ID");
        }
    }

    /// Converts the manufacturer into bytes for serializing the SysEx message.
    pub fn to_bytes(&self) -> Vec<u8> {
        match self.id {
            ManufacturerId::Standard(b) => vec![b],
            ManufacturerId::Extended(bs) => vec![bs[0], bs[1], bs[2]],
        }
    }
}

impl fmt::Display for Manufacturer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match crate::MANUFACTURERS.get(&self.id) {
            Some(n) => n.display_name.clone(),
            None => "(unknown)".to_string(),
        };
        write!(f, "{}", name)
    }
}

// Store the manufacturers and their information in a hashmap. Not complete yet!
// The complete list can be found at https://www.midi.org/specifications-old/item/manufacturer-id-numbers.
// It might be a good idea to scrape them from the website and auto-generate the hashmap code.
lazy_static! {
    static ref MANUFACTURERS: HashMap<ManufacturerId, Manufacturer> = {
        let mut map = HashMap::new();
        map.insert(ManufacturerId::Standard(0x01), Manufacturer { id: ManufacturerId::Standard(0x01), display_name: "Sequential Circuits".to_string(), canonical_name: "Sequential Circuits".to_string(), group: ManufacturerGroup::American });
        map.insert(ManufacturerId::Extended([0x00, 0x00, 0x01]), Manufacturer { id: ManufacturerId::Extended([0x00, 0x00, 0x01]), display_name: "Time/Warner Interactive".to_string(), canonical_name: "Time/Warner Interactive".to_string(), group: ManufacturerGroup::American });
        map.insert(ManufacturerId::Standard(0x40), Manufacturer { id: ManufacturerId::Standard(0x40), display_name: "Kawai".to_string(), canonical_name: "Kawai Musical Instruments MFG. CO. Ltd".to_string(), group: ManufacturerGroup::Japanese });
        map.insert(ManufacturerId::Standard(0x41), Manufacturer { id: ManufacturerId::Standard(0x41), display_name: "Roland".to_string(), canonical_name: "Roland Corporation".to_string(), group: ManufacturerGroup::Japanese });
        map.insert(ManufacturerId::Standard(0x42), Manufacturer { id: ManufacturerId::Standard(0x42), display_name: "KORG".to_string(), canonical_name: "Korg Inc.".to_string(), group: ManufacturerGroup::Japanese });
        map.insert(ManufacturerId::Standard(0x43), Manufacturer { id: ManufacturerId::Standard(0x43), display_name: "Yamaha".to_string(), canonical_name: "Yamaha Corporation".to_string(), group: ManufacturerGroup::Japanese });
        map
    };
}

/// Packed format of SysEx data used by KORG.
pub trait Packed {
    fn packed(&self) -> Vec<u8>;
    fn unpacked(&self) -> Vec<u8>;
}

impl Packed for Vec<u8> {
    /// Returns this vector in a packed format.
    fn packed(&self) -> Vec<u8> {
        // Split the original vector into 7-byte chunks:
        let chunks = self.chunks(7);
        debug!("chunk count = {}", chunks.len());

        let mut result = Vec::<u8>::new();
        for chunk in chunks {
            let mut high_bits = Vec::<bool>::new();

            // Collect the high bits
            for b in chunk {
                high_bits.push(b.bit(7));
            }

            let mut index_byte = 0u8;
            for (index, value) in high_bits.iter().enumerate() {  // starting from b0
                index_byte.set_bit(index, *value);
            }
            result.push(index_byte);

            for b in chunk {
                result.push(b & 0x7f);  // use only bits 0...6
            }
        }

        result
    }

    /// Unpacks a previously packed byte vector.
    fn unpacked(&self) -> Vec<u8> {
        // Split the original vector into 8-byte chunks:
        let chunks = self.chunks(8);
        debug!("chunk count = {}", chunks.len());

        let mut result = Vec::<u8>::new();
        for chunk in chunks {
            debug!("chunk: {:?}", chunk);

            let index_byte = chunk[0];
            debug!("index byte = 0b{:08b}", index_byte);

            let mut index = 0;
            for b in chunk[1..].iter() {  // process bytes 1..7 of chunk
                debug!("index {}: b = {}", index, b);

                let mut v = *b;
                debug!("v = {}", v);
                debug!("index {}: i. bit = {}", index, index_byte.bit(index));

                // Set the top bit of this byte with the corresponding index bit
                v.set_bit(7, index_byte.bit(index));
                debug!("v = {}", v);
                result.push(v);

                index += 1;
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manufacturer_message() {
        let message = Message::ManufacturerExclusive(
            Manufacturer::from_id(ManufacturerId::Standard(0x40)),  // Kawai ID
            vec![
                0x00, // MIDI channel 1
                0x20, // one block data dump
                0x00, // "synthesizer group"
                0x04, // K4/K4r ID no.
                0x00, // internal patch
                0x3F, // patch slot D-16
            ],
        );
        let message_bytes = message.to_bytes();
        assert_eq!(message_bytes, vec![0xF0, 0x40, 0x00, 0x20, 0x00, 0x04, 0x00, 0x3F, 0xF7]);
    }

    #[test]
    fn standard_manufacturer() {
        let message = Message::ManufacturerExclusive(
            Manufacturer::from_id(ManufacturerId::Standard(0x43)),
            vec![],
        );
        let message_bytes = message.to_bytes();
        assert_eq!(message_bytes, vec![0xF0, 0x43, 0xF7]);
    }

    #[test]
    fn extended_manufacturer() {
        let message = Message::ManufacturerExclusive(
            Manufacturer::from_id(ManufacturerId::Extended([0x00, 0x00, 0x01])),
            vec![],
        );
        let message_bytes = message.to_bytes();
        assert_eq!(message_bytes, vec![0xF0, 0x00, 0x00, 0x01, 0xF7]);
    }

    #[test]
    fn manufacturer_display_name() {
        let manufacturer = Manufacturer::from_id(ManufacturerId::Standard(0x43));
        assert_eq!(format!("{}", manufacturer), "Yamaha");
    }

    fn make_short_unpacked_test() -> Vec<u8> {
        vec![101, 202, 103, 204, 105, 206, 107]
    }

    fn make_short_packed_test() -> Vec<u8> {
        vec![42, 101, 74, 103, 76, 105, 78, 107]
    }

    #[test]
    fn test_short_packed() {
        assert_eq!(make_short_unpacked_test().packed(), make_short_packed_test());
    }

    #[test]
    fn test_short_unpacked() {
        assert_eq!(make_short_packed_test().unpacked(), make_short_unpacked_test());
    }
}
