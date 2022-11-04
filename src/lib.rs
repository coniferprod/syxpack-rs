//! # syxpack
//!
//! `syxpack` is a collection of helpers for processing MIDI System Exclusive messages.

use std::fmt;
use std::collections::HashMap;
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

/// MIDI manufacturer. The ID is either a single byte for standard IDs,
/// three bytes for extended IDs, or Development (non-commercial).
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum Manufacturer {
    Standard(u8),
    Extended([u8; 3]),
    Development,
}

impl Manufacturer {
    /// Creates a new manufacturer from System Exclusive bytes.
    pub fn new(data: Vec<u8>) -> Self {
        if data[0] == DEVELOPMENT {
            Manufacturer::Development
        }
        else {
            if data[0] == 0x00 {
                Manufacturer::Extended([data[0], data[1], data[2]])
            }
            else {
                Manufacturer::Standard(data[0])
            }
        }
    }

    /// Gets the manufacturer System Exclusive bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Manufacturer::Development => vec![DEVELOPMENT],
            Manufacturer::Standard(b) => vec![*b],
            Manufacturer::Extended(bs) => vec![bs[0], bs[1], bs[2]],
        }
    }

    /// Gets the manufacturer SysEx bytes as a hex string.
    pub fn to_hex(&self) -> String {
        hex::encode(self.to_bytes()).to_uppercase()
    }

    /// Gets the name of this manufacturer.
    pub fn name(&self) -> String {
        if *self == Manufacturer::Development {
            return "Development / Non-commercial".to_string()
        }

        let hex_id = self.to_hex();
        if let Some(n) = MANUFACTURER_NAMES.get(&*hex_id) {
            n.to_string()
        }
        else {
            "Unknown manufacturer".to_string()
        }
    }

    /// Gets the group of this manufacturer based on the identifier.
    pub fn group(&self) -> ManufacturerGroup {
        match self {
            Manufacturer::Development => ManufacturerGroup::Development,
            Manufacturer::Standard(b) => {
                if (0x01..0x40).contains(b) {
                    ManufacturerGroup::NorthAmerican
                }
                else if (0x40..0x60).contains(b) {
                    ManufacturerGroup::Japanese
                }
                else {
                    ManufacturerGroup::EuropeanAndOther
                }
            },
            Manufacturer::Extended(bs) => {
                if (bs[1] & (1 << 6)) != 0 {  // 0x4x
                    ManufacturerGroup::Japanese
                }
                else if (bs[1] & (1 << 5)) != 0 { // 0x2x
                    ManufacturerGroup::EuropeanAndOther
                }
                else {
                    ManufacturerGroup::NorthAmerican
                }
            }
        }
    }
}

impl fmt::Display for Manufacturer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// The kind of a Universal System Exclusive message.
pub enum UniversalKind {
    NonRealTime,
    RealTime,
}

/// A MIDI System Exclusive message.
pub enum Message {
    Universal { kind: UniversalKind, sub_id1: u8, sub_id2: u8, payload: Vec<u8> },
    ManufacturerSpecific { manufacturer: Manufacturer, payload: Vec<u8> },
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
    /// Creates a new SysEx message based on the initial data bytes.
    pub fn new(data: Vec<u8>) -> Self {
        assert_eq!(data[0], INITIATOR);
        let last_byte_index = data.len() - 1;
        assert_eq!(data[last_byte_index], TERMINATOR);

        match data[1] {
            DEVELOPMENT => Message::ManufacturerSpecific {
                manufacturer: Manufacturer::Development,
                payload: data[2..last_byte_index].to_vec()
            },
            NON_REAL_TIME => Message::Universal {
                kind: UniversalKind::NonRealTime,
                sub_id1: data[2],
                sub_id2: data[3],
                payload: data[4..last_byte_index].to_vec()
            },
            REAL_TIME => Message::Universal {
                kind: UniversalKind::RealTime,
                sub_id1: data[2],
                sub_id2: data[3],
                payload: data[4..last_byte_index].to_vec()
            },
            0x00 => Message::ManufacturerSpecific {
                manufacturer: Manufacturer::Extended([data[1], data[2], data[3]]),
                payload: data[4..last_byte_index].to_vec()
            },
            _ => Message::ManufacturerSpecific {
                manufacturer: Manufacturer::Standard(data[1]),
                payload: data[2..last_byte_index].to_vec()
            },
        }
    }

    // new_xxx variants for constructing messages are not needed,
    // because you can just create a struct variant like an ordinary struct.

    /// Converts the message into bytes for MIDI messaging.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::<u8>::new();

        match self {
            Message::Universal { kind, sub_id1, sub_id2, payload } => {
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
            Message::ManufacturerSpecific { manufacturer, payload } => {
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
    Development,
    NorthAmerican,
    EuropeanAndOther,
    Japanese,
}

lazy_static! {
    static ref MANUFACTURER_NAMES: HashMap<&'static str, &'static str> = {
        HashMap::from([
            ("01", "Sequential Circuits"),
            ("02", "IDP"),
            ("03", "Voyetra Turtle Beach, Inc."),
            ("04", "Moog Music"),
            ("05", "Passport Designs"),
            ("06", "Lexicon Inc."),
            ("07", "Kurzweil / Young Chang"),
            ("08", "Fender"),
            ("09", "MIDI9"),
            ("0A", "AKG Acoustics"),
            ("0B", "Voyce Music"),
            ("0C", "WaveFrame (Timeline)"),
            ("0D", "ADA Signal Processors, Inc."),
            ("0E", "Garfield Electronics"),
            ("0F", "Ensoniq"),
            ("10", "Oberheim / Gibson Labs"),
            ("11", "Apple"),
            ("12", "Grey Matter Response"),
            ("13", "Digidesign Inc."),
            ("14", "Palmtree Instruments"),
            ("15", "JLCooper Electronics"),
            ("16", "Lowrey Organ Company"),
            ("17", "Adams-Smith"),
            ("18", "E-mu"),
            ("19", "Harmony Systems"),
            ("1A", "ART"),
            ("1B", "Baldwin"),
            ("1C", "Eventide"),
            ("1D", "Inventronics"),
            ("1E", "Key Concepts"),
            ("1F", "Clarity"),
            ("20", "Passac"),
            ("21", "Proel Labs (SIEL)"),
            ("22", "Synthaxe (UK)"),
            ("23", "Stepp"),
            ("24", "Hohner"),
            ("25", "Twister"),
            ("26", "Ketron s.r.l."),
            ("27", "Jellinghaus MS"),
            ("28", "Southworth Music Systems"),
            ("29", "PPG (Germany)"),
            ("2A", "JEN"),
            ("2B", "Solid State Logic Organ Systems"),
            ("2C", "Audio Veritrieb-P. Struven"),
            ("2D", "Neve"),
            ("2E", "Soundtracs Ltd."),
            ("2F", "Elka"),
            ("30", "Dynacord"),
            ("31", "Viscount International Spa (Intercontinental Electronics)"),
            ("32", "Drawmer"),
            ("33", "Clavia Digital Instruments"),
            ("34", "Audio Architecture"),
            ("35", "Generalmusic Corp SpA"),
            ("36", "Cheetah Marketing"),
            ("37", "C.T.M."),
            ("38", "Simmons UK"),
            ("39", "Soundcraft Electronics"),
            ("3A", "Steinberg Media Technologies GmbH"),
            ("3B", "Wersi Gmbh"),
            ("3C", "AVAB Niethammer AB"),
            ("3D", "Digigram"),
            ("3E", "Waldorf Electronics GmbH"),
            ("3F", "Quasimidi"),

            ("000001", "Time/Warner Interactive"),
            ("000002", "Advanced Gravis Comp. Tech Ltd."),
            ("000003", "Media Vision"),
            ("000004", "Dornes Research Group"),
            ("000005", "K-Muse"),
            ("000006", "Stypher"),
            ("000007", "Digital Music Corp."),
            ("000008", "IOTA Systems"),
            ("000009", "New England Digital"),
            ("00000A", "Artisyn"),
            ("00000B", "IVL Technologies Ltd."),
            ("00000C", "Southern Music Systems"),
            ("00000D", "Lake Butler Sound Company"),
            ("00000E", "Alesis Studio Electronics"),
            ("00000F", "Sound Creation"),
            ("000010", "DOD Electronics Corp."),
            ("000011", "Studer-Editech"),
            ("000012", "Sonus"),
            ("000013", "Temporal Acuity Products"),
            ("000014", "Perfect Fretworks"),
            ("000015", "KAT Inc."),

            // European & Other Group
            ("002000", "Dream SAS"),
            ("002001", "Strand Lighting"),
            ("002002", "Amek Div of Harman Industries"),
            ("002003", "Casa Di Risparmio Di Loreto"),
            ("002004", "Böhm electronic GmbH"),
            ("002005", "Syntec Digital Audio"),
            ("002006", "Trident Audio Developments"),
            ("002007", "Real World Studio"),
            ("002008", "Evolution Synthesis, Ltd"),
            ("002009", "Yes Technology"),
            ("00200A", "Audiomatica"),
            ("00200B", "Bontempi SpA (Sigma)"),
            ("00200C", "F.B.T. Elettronica SpA"),

            ("002029", "Focusrite/Novation"),

            ("40", "Kawai Musical Instruments MFG. CO. Ltd"),
            ("41", "Roland Corporation"),
            ("42", "Korg Inc."),
            ("43", "Yamaha"),
            ("44", "Casio Computer Co. Ltd"),
            // 0x45 is not assigned
            ("46", "Kamiya Studio Co. Ltd"),
            ("47", "Akai Electric Co. Ltd."),
            ("48", "Victor Company of Japan, Ltd."),
            ("4B", "Fujitsu Limited"),
            ("4C", "Sony Corporation"),
            ("4E", "Teac Corporation"),
            ("50", "Matsushita Electric Industrial Co. , Ltd"),
            ("51", "Fostex Corporation"),
            ("52", "Zoom Corporation"),
            ("54", "Matsushita Communication Industrial Co., Ltd."),
            ("55", "Suzuki Musical Instruments MFG. Co., Ltd."),
            ("56", "Fuji Sound Corporation Ltd."),
            ("57", "Acoustic Technical Laboratory, Inc."),
            // 58h is not assigned
            ("59", "Faith, Inc."),
            ("5A", "Internet Corporation"),
            // 5Bh is not assigned
            ("5C", "Seekers Co. Ltd."),
            // 5Dh and 5Eh are not assigned
            ("5F", "SD Card Association"),

            ("004000", "Crimson Technology Inc."),
            ("004001", "Softbank Mobile Corp"),
            ("004003", "D&M Holdings Inc."),
            ("004004", "Xing Inc."),
            ("004005", "Alpha Theta Corporation"),
            ("004006", "Pioneer Corporation"),
            ("004007", "Slik Corporation"),
        ])
    };
}

/// Packed format of SysEx data used by KORG.
pub trait Packed {
    fn packed(&self) -> Vec<u8>;
    fn unpacked(&self) -> Vec<u8>;
}

impl Packed for Vec<u8> {
    /// Returns this byte vector in a packed format.
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

/// The order of nybbles in a byte.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum NybbleOrder {
    HighFirst,
    LowFirst
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
/// high and low nybbles. The `order` argument determines
/// which one comes first.
pub fn nybblify(data: Vec<u8>, order: NybbleOrder) -> Vec<u8> {
    let mut result = Vec::<u8>::new();

    for b in data {
        let n = nybbles_from_byte(b);
        if order == NybbleOrder::HighFirst {
            result.push(n.0);
            result.push(n.1);
        } else {
            result.push(n.1);
            result.push(n.0);
        }
    }

    result
}

/// Make a new byte array from `data` by combining adjacent bytes
/// representing the high and low nybbles of each byte.
/// The `order` argument determines which one comes first.
pub fn denybblify(data: Vec<u8>, order: NybbleOrder) -> Vec<u8> {
    assert_eq!(data.len() % 2, 0);  // length must be even

    let mut result = Vec::<u8>::new();

    let mut index = 0;
    let mut offset = 0;
    let count = data.len() / 2;

    while index < count {
        let high = data[offset];
        let low = data[offset + 1];
        let b = if order == NybbleOrder::HighFirst {
            byte_from_nybbles(high, low)
        } else {
            byte_from_nybbles(low, high)
        };
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
        if let Message::ManufacturerSpecific { manufacturer, .. } = message {
            assert_eq!(manufacturer, Manufacturer::Standard(0x40));
        }
        else {
            panic!("Expected a manufacturer-specific message with standard identifier");
        }
    }

    #[test]
    fn new_message_manufacturer_extended() {
        let data = vec![0xF0, 0x00, 0x00, 0x0E, 0x00, 0x41, 0x63, 0x00, 0x5D, 0xF7];
        let message = Message::new(data);
        if let Message::ManufacturerSpecific { manufacturer, .. } = message {
            assert_eq!(manufacturer, Manufacturer::Extended([0x00, 0x00, 0x0E]));
        }
        else {
            panic!("Expected a manufacturer-specific message with extended identifier");
        }
    }

    #[test]
    fn manufacturer_message() {
        let message = Message::ManufacturerSpecific {
            manufacturer: Manufacturer::Standard(0x40),  // Kawai ID
            payload: vec![
                0x00, // MIDI channel 1
                0x20, // one block data dump
                0x00, // "synthesizer group"
                0x04, // K4/K4r ID no.
                0x00, // internal patch
                0x3F, // patch slot D-16
            ],
        };
        let message_bytes = message.to_bytes();
        assert_eq!(message_bytes, vec![0xF0, 0x40, 0x00, 0x20, 0x00, 0x04, 0x00, 0x3F, 0xF7]);
    }

    #[test]
    fn standard_manufacturer() {
        let message = Message::ManufacturerSpecific {
            manufacturer: Manufacturer::Standard(0x43),
            payload: vec![],
        };
        let message_bytes = message.to_bytes();
        assert_eq!(message_bytes, vec![0xF0, 0x43, 0xF7]);
    }

    #[test]
    fn extended_manufacturer() {
        let message = Message::ManufacturerSpecific {
            manufacturer: Manufacturer::Extended([0x00, 0x00, 0x01]),
            payload: vec![],
        };
        let message_bytes = message.to_bytes();
        assert_eq!(message_bytes, vec![0xF0, 0x00, 0x00, 0x01, 0xF7]);
    }

    #[test]
    fn development_manufacturer() {
        let message = Message::ManufacturerSpecific {
            manufacturer: Manufacturer::Development,
            payload: vec![],
        };
        let message_bytes = message.to_bytes();
        assert_eq!(message_bytes, vec![0xF0, 0x7D, 0xF7]);
    }

    #[test]
    fn manufacturer_display_name() {
        let manufacturer = Manufacturer::Standard(0x43);
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

    #[test]
    fn test_nybblify() {
        let b = vec![0x01, 0x23, 0x45];
        let nb = nybblify(b, NybbleOrder::HighFirst);
        assert_eq!(nb, vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05]);
    }

    #[test]
    fn test_nybblify_flipped() {
        let b = vec![0x57, 0x61, 0x76];
        let nb = nybblify(b, NybbleOrder::LowFirst);
        assert_eq!(nb, vec![0x07, 0x05, 0x01, 0x06, 0x06, 0x07]);
    }

    #[test]
    fn test_denybblify() {
        let b = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05];
        let nb = denybblify(b, NybbleOrder::HighFirst);
        assert_eq!(nb, vec![0x01, 0x23, 0x45]);
    }

    #[test]
    fn test_denybblify_flipped() {
        let b = vec![0x07, 0x05, 0x01, 0x06, 0x06, 0x07];
        let nb = denybblify(b, NybbleOrder::LowFirst);
        assert_eq!(nb, vec![0x57, 0x61, 0x76]);
    }
}
