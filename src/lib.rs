//! # syxpack
//!
//! `syxpack` is a collection of helpers for processing MIDI System Exclusive messages.

use std::fmt;
use log::debug;
use bit::BitIndex;
use lazy_static::lazy_static;

/// Manufacturer specific SysEx message initiator.
pub const INITIATOR: u8 = 0xf0;

/// Manufacturer specific SysEx message terminator.
pub const TERMINATOR: u8 = 0xf7;

/// Development/non-commercial SysEx manufacturer ID.
pub const DEVELOPMENT: u8 = 0x7d;

/// Universal non-real-time SysEx message indicator.
pub const NON_REAL_TIME: u8 = 0x7e;

/// Universal real-time SysEx message indicator.
pub const REAL_TIME: u8 = 0x7f;

/// MIDI manufacturer ID. Either a single byte for standard IDs,
/// three bytes for extended IDs, or Development (non-commercial).
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum ManufacturerId {
    Standard(u8),
    Extended([u8; 3]),
    Development,
    Unknown,
}

impl fmt::Display for ManufacturerId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let hex = match self {
            ManufacturerId::Standard(b) => format!("{:X}", b),
            ManufacturerId::Extended(bs) => format!("{:X} {:X} {:X} ", bs[0], bs[1], bs[2]),
            ManufacturerId::Development => format!("{:X}", DEVELOPMENT),
            ManufacturerId::Unknown => "?".to_string(),
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
    Universal(UniversalKind, u8, u8, Vec<u8>),
    ManufacturerSpecific(Manufacturer, Vec<u8>),
}

/// Returns the number of System Exclusive messages in this vector,
/// based on the count of terminator bytes.
pub fn message_count(data: Vec<u8>) -> usize {
    data.iter().filter(|&n| *n == TERMINATOR).count()
}

/// Splits the vector by the terminator byte, including it.
pub fn split_messages(data: Vec<u8>) -> Vec<Vec<u8>> {
    let mut parts: Vec<Vec<u8>> = Vec::new();
    for part in data.split_inclusive(|&n| n == TERMINATOR) {
        parts.push(part.to_vec());
    }
    parts
}

impl Message {
    /// Creates a SysEx message based on the initial data bytes.
    pub fn new(data: Vec<u8>) -> Self {
        assert_eq!(data[0], INITIATOR);
        let last_byte_index = data.len() - 1;
        assert_eq!(data[last_byte_index], TERMINATOR);

        match data[1] {
            DEVELOPMENT => Message::new_manufacturer(
                Manufacturer::from_id(ManufacturerId::Development),
                data[2..last_byte_index].to_vec()),
            NON_REAL_TIME => Message::new_universal(
                UniversalKind::NonRealTime,
                data[2], data[3],
                data[4..last_byte_index].to_vec()),
            REAL_TIME => Message::new_universal(
                UniversalKind::RealTime,
                data[2], data[3],
                data[4..last_byte_index].to_vec()),
            0x00 => Message::ManufacturerSpecific(
                Manufacturer::from_id(ManufacturerId::Extended([data[1], data[2], data[3]])),
                data[4..last_byte_index].to_vec()),
            _ => Message::ManufacturerSpecific(
                    Manufacturer::from_id(ManufacturerId::Standard(data[1])),
                    data[2..last_byte_index].to_vec()),
        }
    }

    /// Creates a manufacturer-specific SysEx message.
    pub fn new_manufacturer(manufacturer: Manufacturer, payload: Vec<u8>) -> Self {
        Message::ManufacturerSpecific(manufacturer, payload)
    }

    /// Creates a new universal SysEx message with the given sub-IDs.
    pub fn new_universal(kind: UniversalKind, sub_id1: u8, sub_id2: u8, payload: Vec<u8>) -> Self {
        Message::Universal(kind, sub_id1, sub_id2, payload)
    }

    /// Converts the message into bytes for MIDI messaging.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::<u8>::new();

        match self {
            Message::Universal(kind, sub_id1, sub_id2, payload) => {
                result.push(INITIATOR);
                result.push(match kind {
                    UniversalKind::NonRealTime => NON_REAL_TIME,
                    UniversalKind::RealTime => REAL_TIME,
                });
                result.push(*sub_id1);
                result.push(*sub_id2);
                result.extend(payload);
                result.push(TERMINATOR);
            },
            Message::ManufacturerSpecific(manufacturer, payload) => {
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
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum ManufacturerGroup {
    American,
    EuropeanOrOther,
    Japanese,
    NotApplicable,  // used for Development/Non-commercial
}

/// MIDI equipment manufacturer.
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct Manufacturer {
    pub id: ManufacturerId,
    pub display_name: String,
    pub canonical_name: String,
    pub group: ManufacturerGroup,
}

impl Manufacturer {
    /// Makes a manufacturer from its identifier.
    ///
    /// # Arguments
    ///
    /// * `id`- a member of the `ManufacturerId` enumeration
    pub fn from_id(id: ManufacturerId) -> Self {
        if let Some(manufacturer) = crate::find_manufacturer(&id) {
            Manufacturer {
                id: manufacturer.id,
                display_name: manufacturer.display_name.clone(),
                canonical_name: manufacturer.canonical_name.clone(),
                group: manufacturer.group,
            }
        }
        else {
            crate::find_manufacturer(&ManufacturerId::Unknown).unwrap().clone()
            //panic!("Unknown manufacturer ID: {}", id);
        }
    }

    /// Converts the manufacturer into bytes for serializing the SysEx message.
    pub fn to_bytes(&self) -> Vec<u8> {
        match self.id {
            ManufacturerId::Development => vec![DEVELOPMENT],
            ManufacturerId::Standard(b) => vec![b],
            ManufacturerId::Extended(bs) => vec![bs[0], bs[1], bs[2]],
            ManufacturerId::Unknown => panic!("Unknown manufacturer ID: {}", self.id),
        }
    }
}

impl fmt::Display for Manufacturer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.display_name)
    }
}

// Storing the manufacturers as a simple vector. There are only a couple of hundred of them,
// so a simple linear search should be OK. The items should be unique, but if they are not,
// it only means that only the first one is used.
// The complete list can be found at https://www.midi.org/specifications-old/item/manufacturer-id-numbers.
// It might be a good idea to scrape them from the website and auto-generate the vectore initialization source code.
lazy_static! {
    static ref ALL_MANUFACTURERS: Vec<Manufacturer> = {
        vec![
            Manufacturer { id: ManufacturerId::Standard(0x01), display_name: "Sequential Circuits".to_string(), canonical_name: "Sequential Circuits".to_string(), group: ManufacturerGroup::American },
            Manufacturer { id: ManufacturerId::Extended([0x00, 0x00, 0x01]), display_name: "Time/Warner Interactive".to_string(), canonical_name: "Time/Warner Interactive".to_string(), group: ManufacturerGroup::American },
            Manufacturer { id: ManufacturerId::Extended([0x00, 0x00, 0x0E]), display_name: "Alesis".to_string(), canonical_name: "Alesis Studio Electronics".to_string(), group: ManufacturerGroup::American },
            Manufacturer { id: ManufacturerId::Extended([0x00, 0x20, 0x29]), display_name: "Novation".to_string(), canonical_name: "Focusrite/Novation".to_string(), group: ManufacturerGroup::EuropeanOrOther },
            Manufacturer { id: ManufacturerId::Standard(0x40), display_name: "Kawai".to_string(), canonical_name: "Kawai Musical Instruments MFG. CO. Ltd".to_string(), group: ManufacturerGroup::Japanese },
            Manufacturer { id: ManufacturerId::Standard(0x41), display_name: "Roland".to_string(), canonical_name: "Roland Corporation".to_string(), group: ManufacturerGroup::Japanese },
            Manufacturer { id: ManufacturerId::Standard(0x42), display_name: "KORG".to_string(), canonical_name: "Korg Inc.".to_string(), group: ManufacturerGroup::Japanese },
            Manufacturer { id: ManufacturerId::Standard(0x43), display_name: "Yamaha".to_string(), canonical_name: "Yamaha Corporation".to_string(), group: ManufacturerGroup::Japanese },
            Manufacturer { id: ManufacturerId::Development, display_name: "Development/Non-commercial".to_string(), canonical_name: "Development/Non-commercial".to_string(), group: ManufacturerGroup::NotApplicable },
            Manufacturer { id: ManufacturerId::Unknown, display_name: "(unknown)".to_string(), canonical_name: "(unknown)".to_string(), group: ManufacturerGroup::NotApplicable },
        ]
    };
}

/// Find a manufacturer based on its identifier.
///
/// # Arguments
///
/// * `id`- a member of the `ManufacturerId` enumeration
pub fn find_manufacturer(id: &ManufacturerId) -> Option<&'static Manufacturer> {
    ALL_MANUFACTURERS.iter().find(|x| x.id == *id)
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

fn high_nybble(b: u8) -> u8 {
    (b & 0xf0) >> 4
}

fn low_nybble(b: u8) -> u8 {
    b & 0x0f
}

fn nybbles_from_byte(b: u8) -> (u8, u8) {
    (high_nybble(b), low_nybble(b))
}

fn byte_from_nybbles(high: u8, low: u8) -> u8 {
    high << 4 | low
}

/// Make a new byte array from `data` with the bytes split into
/// high and low nybbles.
pub fn nybblify(data: Vec<u8>) -> Vec<u8> {
    let mut result = Vec::<u8>::new();

    for b in data {
        let n = nybbles_from_byte(b);
        result.push(n.0);
        result.push(n.1);
    }

    result
}

/// Make a new byte array from `data` by combining adjacent bytes
/// representing the high and low nybbles of each byte.
pub fn denybblify(data: Vec<u8>) -> Vec<u8> {
    assert_eq!(data.len() % 2, 0);  // length must be even

    let mut result = Vec::<u8>::new();

    let mut index = 0;
    let mut offset = 0;
    let count = data.len() / 2;

    while index < count {
        let b = byte_from_nybbles(data[offset], data[offset + 1]);
        result.push(b);
        index += 1;
        offset += 2;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_message_manufacturer_standard() {
        let data = vec![0xF0, 0x40, 0x00, 0x20, 0x00, 0x04, 0x00, 0x3F, 0xF7];
        let message = Message::new(data);
        if let Message::ManufacturerSpecific(manufacturer, _) = message {
            assert_eq!(manufacturer.id, ManufacturerId::Standard(0x40));
        }
        else {
            panic!("Expected a manufacturer-specific message with standard identifier");
        }
    }

    #[test]
    fn new_message_manufacturer_extended() {
        let data = vec![0xF0, 0x00, 0x00, 0x0E, 0x00, 0x41, 0x63, 0x00, 0x5D, 0xF7];
        let message = Message::new(data);
        if let Message::ManufacturerSpecific(manufacturer, _) = message {
            assert_eq!(manufacturer.id, ManufacturerId::Extended([0x00, 0x00, 0x0E]));
        }
        else {
            panic!("Expected a manufacturer-specific message with extended identifier");
        }
    }

    #[test]
    fn manufacturer_message() {
        let message = Message::ManufacturerSpecific(
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
        let message = Message::ManufacturerSpecific(
            Manufacturer::from_id(ManufacturerId::Standard(0x43)),
            vec![],
        );
        let message_bytes = message.to_bytes();
        assert_eq!(message_bytes, vec![0xF0, 0x43, 0xF7]);
    }

    #[test]
    fn extended_manufacturer() {
        let message = Message::ManufacturerSpecific(
            Manufacturer::from_id(ManufacturerId::Extended([0x00, 0x00, 0x01])),
            vec![],
        );
        let message_bytes = message.to_bytes();
        assert_eq!(message_bytes, vec![0xF0, 0x00, 0x00, 0x01, 0xF7]);
    }

    #[test]
    fn development_manufacturer() {
        let message = Message::ManufacturerSpecific(
            Manufacturer::from_id(ManufacturerId::Development),
            vec![],
        );
        let message_bytes = message.to_bytes();
        assert_eq!(message_bytes, vec![0xF0, 0x7D, 0xF7]);
    }

    #[test]
    fn manufacturer_display_name() {
        let manufacturer = Manufacturer::from_id(ManufacturerId::Standard(0x43));
        assert_eq!(format!("{}", manufacturer), "Yamaha");
    }

    #[test]
    fn test_find_manufacturer() {
        match find_manufacturer(&ManufacturerId::Standard(0x40)) {
            Some(manufacturer) => assert_eq!(manufacturer.id, ManufacturerId::Standard(0x40)),
            None => panic!("Manufacturer not found")
        };
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

    #[test]
    fn test_nybblify() {
        let b = vec![0x01, 0x23, 0x45];
        let nb = nybblify(b);
        assert_eq!(nb, vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05]);
    }

    #[test]
    fn test_denybblify() {
        let b = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05];
        let nb = denybblify(b);
        assert_eq!(nb, vec![0x01, 0x23, 0x45]);
    }
}
