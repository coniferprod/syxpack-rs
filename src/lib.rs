//! # syxpack
//!
//! `syxpack` is a collection of helpers for processing MIDI System Exclusive messages.

use std::fmt;
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::Path;
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

/// Error type for System Exclusive messages.
#[derive(Debug)]
pub enum SystemExclusiveError {
    InvalidMessage,
    InvalidManufacturer,
}

impl fmt::Display for SystemExclusiveError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match &self {
            SystemExclusiveError::InvalidMessage => "Invalid System Exclusive message",
            SystemExclusiveError::InvalidManufacturer => "Invalid manufacturer identifier"
        })
    }
}

/// MIDI manufacturer. The ID is either a single byte for standard IDs,
/// or three bytes for extended IDs.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum Manufacturer {
    Standard(u8),
    Extended([u8; 3]),
}

impl Manufacturer {
    /// Creates a new manufacturer from System Exclusive bytes.
    pub fn new(data: Vec<u8>) -> Result<Self, SystemExclusiveError> {
        if data.len() != 1 && data.len() != 3 {
            return Err(SystemExclusiveError::InvalidManufacturer);
        }
        if data[0] == 0x00 {
            Ok(Manufacturer::Extended([data[0], data[1], data[2]]))
        }
        else {
            Ok(Manufacturer::Standard(data[0]))
        }
    }

    /// Gets the manufacturer System Exclusive bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Manufacturer::Standard(b) => vec![*b],
            Manufacturer::Extended(bs) => vec![bs[0], bs[1], bs[2]],
        }
    }

    /// Gets the manufacturer SysEx bytes as a hex string.
    pub fn to_hex(&self) -> String {
        hex::encode(self.to_bytes()).to_uppercase()
    }

    /// Returns `true` if this manufacturer represents development / non-commercial.
    pub fn is_development(&self) -> bool {
        match self {
            Manufacturer::Standard(b) => *b == DEVELOPMENT,
            Manufacturer::Extended(_) => false
        }
    }

    /// Gets the name of this manufacturer.
    pub fn name(&self) -> String {
        if self.is_development() {
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
        if self.is_development() {
            return ManufacturerGroup::Development
        }

        match self {
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

/// Finds a manufacturer by initial match of name.
pub fn find_manufacturer(name: &str) -> Result<Manufacturer, SystemExclusiveError> {
    for (key, value) in &*MANUFACTURER_NAMES {
        if value.to_lowercase().starts_with(&name.to_lowercase()) {
            let id_bytes = hex::decode(key).unwrap();
            return Ok(Manufacturer::new(id_bytes).unwrap());
        }
    }
    return Err(SystemExclusiveError::InvalidManufacturer);
}

/// The kind of a Universal System Exclusive message.#[derive(Debug)]
#[derive(Debug)]
pub enum UniversalKind {
    NonRealTime,
    RealTime,
}

impl fmt::Display for UniversalKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            UniversalKind::NonRealTime => "Non-Real-time",
            UniversalKind::RealTime => "Real-time",
        };
        write!(f, "{}", name)
    }
}

/// A MIDI System Exclusive message.
#[derive(Debug)]
pub enum Message {
    Universal { kind: UniversalKind, target: u8, sub_id1: u8, sub_id2: u8, payload: Vec<u8> },
    ManufacturerSpecific { manufacturer: Manufacturer, payload: Vec<u8> },
}

/// Returns the number of System Exclusive messages in this vector,
/// based on the count of terminator bytes.
pub fn message_count(data: &Vec<u8>) -> usize {
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
    pub fn from_bytes(data: &[u8]) -> Result<Self, SystemExclusiveError> {
        if data[0] != INITIATOR {
            return Err(SystemExclusiveError::InvalidMessage);
        }

        let last_byte_index = data.len() - 1;
        if data[last_byte_index] != TERMINATOR {
            return Err(SystemExclusiveError::InvalidMessage);
        }

        if data.len() < 5 {   // too short
            return Err(SystemExclusiveError::InvalidMessage);
        }

        match data[1] {
            DEVELOPMENT => Ok(Message::ManufacturerSpecific {
                manufacturer: Manufacturer::Standard(data[1]),
                payload: data[2..last_byte_index].to_vec()
            }),
            NON_REAL_TIME => Ok(Message::Universal {
                kind: UniversalKind::NonRealTime,
                target: data[2],
                sub_id1: data[3],
                sub_id2: data[4],
                payload: data[5..last_byte_index].to_vec()
            }),
            REAL_TIME => Ok(Message::Universal {
                kind: UniversalKind::RealTime,
                target: data[2],
                sub_id1: data[3],
                sub_id2: data[4],
                payload: data[5..last_byte_index].to_vec()
            }),
            0x00 => Ok(Message::ManufacturerSpecific {
                manufacturer: Manufacturer::Extended([data[1], data[2], data[3]]),
                payload: data[4..last_byte_index].to_vec()
            }),
            _ => Ok(Message::ManufacturerSpecific {
                manufacturer: Manufacturer::Standard(data[1]),
                payload: data[2..last_byte_index].to_vec()
            }),
        }
    }

    /// Converts the message into bytes for MIDI messaging.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::<u8>::new();

        match self {
            Message::Universal { kind, target, sub_id1, sub_id2, payload } => {
                result.push(INITIATOR);
                result.push(match kind {
                    UniversalKind::NonRealTime => NON_REAL_TIME,
                    UniversalKind::RealTime => REAL_TIME,
                });
                result.push(*target);
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

    pub fn digest(&self) -> md5::Digest {
        md5::compute(self.to_bytes())
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

impl fmt::Display for ManufacturerGroup {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            ManufacturerGroup::Development => "Development",
            ManufacturerGroup::EuropeanAndOther => "European & Other",
            ManufacturerGroup::Japanese => "Japanese",
            ManufacturerGroup::NorthAmerican => "North American",
        };
        write!(f, "{}", name)
    }
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
            ("000016", "Opcode Systems"),
            ("000017", "Rane Corporation"),
            ("000018", "Anadi Electronique"),
            ("000019", "KMX"),
            ("00001A", "Allen & Heath Brenell"),
            ("00001B", "Peavey Electronics"),
            ("00001C", "360 Systems"),
            ("00001D", "Spectrum Design and Development"),
            ("00001E", "Marquis Music"),
            ("00001F", "Zeta Systems"),
            ("000020", "Axxes (Brian Parsonett)"),
            ("000021", "Orban"),
            ("000022", "Indian Valley Mfg."),
            ("000023", "Triton"),
            ("000024", "KTI"),
            ("000025", "Breakway Technologies"),
            ("000026", "Leprecon / CAE Inc."),
            ("000027", "Harrison Systems Inc."),
            ("000028", "Future Lab/Mark Kuo"),
            ("000029", "Rocktron Corporation"),
            ("00002A", "PianoDisc"),
            ("00002B", "Cannon Research Group"),
            ("00002C", "Reserved"),
            ("00002D", "Rodgers Instrument LLC"),
            ("00002E", "Blue Sky Logic"),
            ("00002F", "Encore Electronics"),
            ("000030", "Uptown"),
            ("000031", "Voce"),
            ("000032", "CTI Audio, Inc. (Musically Intel. Devs.)"),
            ("000033", "S3 Incorporated"),
            ("000034", "Broderbund / Red Orb"),
            ("000035", "Allen Organ Co."),
            ("000036", "Reserved"),
            ("000037", "Music Quest"),
            ("000038", "Aphex"),
            ("000039", "Gallien Krueger"),
            ("00003A", "IBM"),
            ("00003B", "Mark Of The Unicorn"),
            ("00003C", "Hotz Corporation"),
            ("00003D", "ETA Lighting"),
            ("00003E", "NSI Corporation"),
            ("00003F", "Ad Lib, Inc."),
            ("000040", "Richmond Sound Design"),
            ("000041", "Microsoft"),
            ("000042", "Mindscape (Software Toolworks)"),
            ("000043", "Russ Jones Marketing / Niche"),
            ("000044", "Intone"),
            ("000045", "Advanced Remote Technologies"),
            ("000046", "White Instruments"),
            ("000047", "GT Electronics/Groove Tubes"),
            ("000048", "Pacific Research & Engineering"),
            ("000049", "Timeline Vista, Inc."),
            ("00004A", "Mesa Boogie Ltd."),
            ("00004B", "FSLI"),
            ("00004C", "Sequoia Development Group"),
            ("00004D", "Studio Electronics"),
            ("00004E", "Euphonix, Inc"),
            ("00004F", "InterMIDI, Inc."),

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
            ("00200D", "MidiTemp GmbH"),
            ("00200E", "LA Audio (Larking Audio)"),
            ("00200F", "Zero 88 Lighting Limited"),
            ("002010", "Micon Audio Electronics GmbH"),
            ("002011", "Forefront Technology"),
            ("002012", "Studio Audio and Video Ltd."),
            ("002013", "Kenton Electronics"),

            ("00201F", "TC Electronics"),
            ("002020", "Doepfer Musikelektronik GmbH"),
            ("002021", "Creative ATC / E-mu"),

            ("002029", "Focusrite/Novation"),

            ("002032", "Behringer GmbH"),
            ("002033", "Access Music Electronics"),

            ("00203A", "Propellerhead Software"),

            ("00206B", "Arturia"),
            ("002076", "Teenage Engineering"),

            ("002103", "PreSonus Software Ltd"),

            ("002109", "Native Instruments"),

            ("002110", "ROLI Ltd"),

            ("00211A", "IK Multimedia"),

            ("00211D", "Ableton"),

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

pub fn read_file(name: &Path) -> Option<Vec<u8>> {
    match fs::File::open(&name) {
        Ok(mut f) => {
            let mut buffer = Vec::new();
            match f.read_to_end(&mut buffer) {
                Ok(_) => Some(buffer),
                Err(_) => None
            }
        },
        Err(_) => {
            eprintln!("Unable to open file {}", &name.display());
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_message_manufacturer_standard() {
        let data = vec![0xF0, 0x40, 0x00, 0x20, 0x00, 0x04, 0x00, 0x3F, 0xF7];
        let message = Message::from_bytes(&data);
        if let Ok(Message::ManufacturerSpecific { manufacturer, .. }) = message {
            assert_eq!(manufacturer, Manufacturer::Standard(0x40));
        }
        else {
            panic!("Expected a manufacturer-specific message with standard identifier");
        }
    }

    #[test]
    fn new_message_manufacturer_extended() {
        let data = vec![0xF0, 0x00, 0x00, 0x0E, 0x00, 0x41, 0x63, 0x00, 0x5D, 0xF7];
        let message = Message::from_bytes(&data);
        if let Ok(Message::ManufacturerSpecific { manufacturer, .. }) = message {
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
    fn universal() {
        let message = Message::Universal {
            kind: UniversalKind::NonRealTime,
            target: 0x00,
            sub_id1: 0x06,  // General information
            sub_id2: 0x01,  // - Identity request
            payload: vec![]
        };
        let message_bytes = message.to_bytes();
        assert_eq!(message_bytes, vec![INITIATOR, NON_REAL_TIME, 0x00, 0x06, 0x01, TERMINATOR]);
    }

    #[test]
    fn development_manufacturer() {
        let message = Message::ManufacturerSpecific {
            manufacturer: Manufacturer::Standard(DEVELOPMENT),
            payload: vec![],
        };
        let message_bytes = message.to_bytes();
        assert_eq!(message_bytes, vec![INITIATOR, DEVELOPMENT, TERMINATOR]);
    }

    #[test]
    fn manufacturer_display_name() {
        let manufacturer = Manufacturer::Standard(0x43);
        assert_eq!(format!("{}", manufacturer), "Yamaha");
    }

    #[test]
    fn find_manufacturer_name_success() {
        let manuf = find_manufacturer("yama").unwrap();
        assert_eq!(manuf.name(), "Yamaha");
    }

    #[test]
    fn find_manufacturer_name_failure() {
        assert!(find_manufacturer("humppaurku").is_err());
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
