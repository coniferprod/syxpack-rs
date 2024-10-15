//! # syxpack
//!
//! `syxpack` is a collection of helpers for processing MIDI System Exclusive messages.

use std::fmt;
use std::collections::HashMap;
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
    pub fn from_bytes(data: &[u8]) -> Result<Self, SystemExclusiveError> {
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

    /// Creates a new manufacturer with default ID.
    pub fn new() -> Self {
        Manufacturer::Standard(0x40)
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
            return Ok(Manufacturer::from_bytes(&id_bytes).unwrap());
        }
    }
    return Err(SystemExclusiveError::InvalidManufacturer);
}

/// The kind of a Universal System Exclusive message.
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

    /// Compute the MD5 digest for this message.
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
            ("000050", "MIDI Solutions Inc."),
            ("000051", "3DO Company"),
            ("000052", "Lightwave Research / High End Systems"),
            ("000053", "Micro-W Corporation"),
            ("000054", "Spectral Synthesis, Inc."),
            ("000055", "Lone Wolf"),
            ("000056", "Studio Technologies Inc."),
            ("000057", "Peterson Electro-Musical Product, Inc."),
            ("000058", "Atari Corporation"),
            ("000059", "Marion Systems Corporation"),
            ("00005A", "Design Event"),
            ("00005B", "Winjammer Software Ltd."),
            ("00005C", "AT&T Bell Laboratories"),
            ("00005D", "Reserved"),
            ("00005E", "Symetrix"),
            ("00005F", "MIDI the World"),
            ("000060", "Spatializer"),
            ("000061", "Micros ‘N MIDI"),
            ("000062", "Accordians International"),
            ("000063", "EuPhonics (now 3Com)"),
            ("000064", "Musonix"),
            ("000065", "Turtle Beach Systems (Voyetra)"),
            ("000066", "Loud Technologies / Mackie"),
            ("000067", "Compuserve"),
            ("000068", "BEC Technologies"),
            ("000069", "QRS Music Inc"),
            ("00006A", "P.G. Music"),
            ("00006B", "Sierra Semiconductor"),
            ("00006C", "EpiGraf"),
            ("00006D", "Electronics Diversified Inc"),
            ("00006E", "Tune 1000"),
            ("00006F", "Advanced Micro Devices"),
            ("000070", "Mediamation"),
            ("000071", "Sabine Musical Mfg. Co. Inc."),
            ("000072", "Woog Labs"),
            ("000073", "Micropolis Corp"),
            ("000074", "Ta Horng Musical Instrument"),
            ("000075", "e-Tek Labs (Forte Tech)"),
            ("000076", "Electro-Voice"),
            ("000077", "Midisoft Corporation"),
            ("000078", "QSound Labs"),
            ("000079", "Westrex"),
            ("00007A", "Nvidia"),
            ("00007B", "ESS Technology"),
            ("00007C", "Media Trix Peripherals"),
            ("00007D", "Brooktree Corp"),
            ("00007E", "Otari Corp"),
            ("00007F", "Key Electronics, Inc."),
            ("000100", "Shure Incorporated"),
            ("000101", "AuraSound"),
            ("000102", "Crystal Semiconductor"),
            ("000103", "Conexant (Rockwell)"),
            ("000104", "Silicon Graphics"),
            ("000105", "M-Audio (Midiman)"),
            ("000106", "PreSonus"),
            // 000107?
            ("000108", "Topaz Enterprises"),
            ("000109", "Cast Lighting"),
            ("00010A", "Microsoft Consumer Division"),
            ("00010B", "Sonic Foundry"),
            ("00010C", "Line 6 (Fast Forward) (Yamaha)"),
            ("00010D", "Beatnik Inc"),
            ("00010E", "Van Koevering Company"),
            ("00010F", "Altech Systems"),
            ("000110", "S & S Research"),
            ("000111", "VLSI Technology"),
            ("000112", "Chromatic Research"),
            ("000113", "Sapphire"),
            ("000114", "IDRC"),
            ("000115", "Justonic Tuning"),
            ("000116", "TorComp Research Inc."),
            ("000117", "Newtek Inc."),
            ("000118", "Sound Sculpture"),
            ("000119", "Walker Technical"),
            ("00011A", "Digital Harmony (PAVO)"),
            ("00011B", "InVision Interactive"),
            ("00011C", "T-Square Design"),
            ("00011D", "Nemesys Music Technology"),
            ("00011E", "DBX Professional (Harman Intl)"),
            ("00011F", "Syndyne Corporation"),
            ("000120", "Bitheadz"),
            ("000121", "BandLab Technologies"),
            ("000122", "Analog Devices"),
            ("000123", "National Semiconductor"),
            ("000124", "Boom Theory / Adinolfi Alternative Percussion"),
            ("000125", "Virtual DSP Corporation"),
            ("000126", "Antares Systems"),
            ("000127", "Angel Software"),
            ("000128", "St Louis Music"),
            ("000129", "Passport Music Software LLC (Gvox)"),
            ("00012A", "Ashley Audio Inc."),
            ("00012B", "Vari-Lite Inc."),
            ("00012C", "Summit Audio Inc."),
            ("00012D", "Aureal Semiconductor Inc."),
            ("00012E", "SeaSound LLC"),
            ("00012F", "U.S. Robotics"),
            ("000130", "Aurisis Research"),
            ("000131", "Nearfield Research"),
            ("000132", "FM7 Inc"),
            ("000133", "Swivel Systems"),
            ("000134", "Hyperactive Audio Systems"),
            ("000135", "MidiLite (Castle Studios Productions)"),
            ("000136", "Radikal Technologies"),
            ("000137", "Roger Linn Design"),
            ("000138", "TC-Helicon Vocal Technologies"),
            ("000139", "Event Electronics"),
            ("00013A", "Sonic Network Inc"),
            ("00013B", "Realtime Music Solutions"),
            ("00013C", "Apogee Digital"),
            ("00013D", "Classical Organs, Inc."),
            ("00013E", "Microtools Inc."),
            ("00013F", "Numark Industries"),
            ("000140", "Frontier Design Group, LLC"),
            ("000141", "Recordare LLC"),
            ("000142", "Starr Labs"),
            ("000143", "Voyager Sound Inc."),
            ("000144", "Manifold Labs"),
            ("000145", "Aviom Inc."),
            ("000146", "Mixmeister Technology"),
            ("000147", "Notation Software"),
            ("000148", "Mercurial Communications"),
            ("000149", "Wave Arts"),
            ("00014A", "Logic Sequencing Devices"),
            ("00014B", "Axess Electronics"),
            ("00014C", "Muse Research"),
            ("00014D", "Open Labs"),
            ("00014E", "Guillemot Corp"),
            ("00014F", "Samson Technologies"),
            ("000150", "Electronic Theatre Controls"),
            ("000151", "Blackberry (RIM)"),
            ("000152", "Mobileer"),
            ("000153", "Synthogy"),
            ("000154", "Lynx Studio Technology Inc."),
            ("000155", "Damage Control Engineering LLC"),
            ("000156", "Yost Engineering, Inc."),
            ("000157", "Brooks & Forsman Designs LLC / DrumLite"),
            ("000158", "Infinite Response"),
            ("000159", "Garritan Corp"),
            ("00015A", "Plogue Art et Technologie, Inc"),
            ("00015B", "RJM Music Technology"),
            ("00015C", "Custom Solutions Software"),
            ("00015D", "Sonarcana LLC / Highly Liquid"),
            ("00015E", "Centrance"),
            ("00015F", "Kesumo LLC"),
            ("000160", "Stanton (Gibson Brands)"),
            ("000161", "Livid Instruments"),
            ("000162", "First Act / 745 Media"),
            ("000163", "Pygraphics, Inc."),
            ("000164", "Panadigm Innovations Ltd"),
            ("000165", "Avedis Zildjian Co"),
            ("000166", "Auvital Music Corp"),
            ("000167", "You Rock Guitar (was: Inspired Instruments)"),
            ("000168", "Chris Grigg Designs"),
            ("000169", "Slate Digital LLC"),
            ("00016A", "Mixware"),
            ("00016B", "Social Entropy"),
            ("00016C", "Source Audio LLC"),
            ("00016D", "Ernie Ball / Music Man"),
            ("00016E", "Fishman"),
            ("00016F", "Custom Audio Electronics"),
            ("000170", "American Audio/DJ"),
            ("000171", "Mega Control Systems"),
            ("000172", "Kilpatrick Audio"),
            ("000173", "iConnectivity"),
            ("000174", "Fractal Audio"),
            ("000175", "NetLogic Microsystems"),
            ("000176", "Music Computing"),
            ("000177", "Nektar Technology Inc"),
            ("000178", "Zenph Sound Innovations"),
            ("000179", "DJTechTools.com"),
            ("00017A", "Rezonance Labs"),
            ("00017B", "Decibel Eleven"),
            ("00017C", "CNMAT"),
            ("00017D", "Media Overkill"),
            ("00017E", "Confusion Studios"),
            ("00017F", "moForte Inc"),
            ("000200", "Miselu Inc"),
            ("000201", "Amelia's Compass LLC"),
            ("000202", "Zivix LLC"),
            ("000203", "Artiphon"),
            ("000204", "Synclavier Digital"),
            ("000205", "Light & Sound Control Devices LLC"),
            ("000206", "Retronyms Inc"),
            ("000207", "JS Technologies"),
            ("000208", "Quicco Sound"),
            ("000209", "A-Designs Audio"),
            ("00020A", "McCarthy Music Corp"),
            ("00020B", "Denon DJ"),
            ("00020C", "Keith Robert Murray"),
            ("00020D", "Google"),
            ("00020E", "ISP Technologies"),
            ("00020F", "Abstrakt Instruments LLC"),
            ("000210", "Meris LLC"),
            ("000211", "Sensorpoint LLC"),
            ("000212", "Hi-Z Labs"),
            ("000213", "Imitone"),
            ("000214", "Intellijel Designs Inc."),
            ("000215", "Dasz Instruments Inc."),
            ("000216", "Remidi"),
            ("000217", "Disaster Area Designs LLC"),
            ("000218", "Universal Audio"),
            ("000219", "Carter Duncan Corp"),
            ("00021A", "Essential Technology"),
            ("00021B", "Cantux Research LLC"),
            ("00021C", "Hummel Technologies"),
            ("00021D", "Sensel Inc"),
            ("00021E", "DBML Group"),
            ("00021F", "Madrona Labs"),
            ("000220", "Mesa Boogie"),
            ("000221", "Effigy Labs"),
            ("000222", "Amenote"),
            ("000223", "Red Panda LLC"),
            ("000224", "OnSong LLC"),
            ("000225", "Jamboxx Inc."),
            ("000226", "Electro-Harmonix"),
            ("000227", "RnD64 Inc"),
            ("000228", "Neunaber Technology LLC"),
            ("000229", "Kaom Inc."),
            ("00022A", "Hallowell EMC"),
            ("00022B", "Sound Devices, LLC"),
            ("00022C", "Spectrasonics, Inc"),
            ("00022D", "Second Sound, LLC"),
            ("00022E", "8eo (Horn)"),
            ("00022F", "VIDVOX LLC"),
            ("000230", "Matthews Effects"),
            ("000231", "Bright Blue Beetle"),
            ("000232", "Audio Impressions"),
            ("000233", " Looperlative"),
            ("000234", "Steinway"),
            ("000235", "Ingenious Arts and Technologies LLC"),
            ("000236", "DCA Audio"),
            ("000237", "Buchla USA"),
            ("000238", "Sinicon"),
            ("000239", "Isla Instruments"),
            ("00023A", "Soundiron LLC"),
            ("00023B", "Sonoclast, LLC"),
            ("00023C", "Copper and Cedar"),
            ("00023D", "Whirled Notes"),
            ("00023E", "Cejetvole, LLC"),
            ("00023F", "DAWn Audio LLC"),
            ("000240", "Space Brain Circuits"),
            ("000241", "Caedence"),
            ("000242", "HCN Designs, LLC (The MIDI Maker)"),
            ("000243", "PTZOptics"),
            ("000244", "Noise Engineering"),
            ("000245", "Synthesia LLC"),
            ("000246", "Jeff Whitehead Lutherie LLC"),
            ("000247", "Wampler Pedals Inc."),
            ("000248", "Tapis Magique"),
            ("000249", "Leaf Secrets"),
            ("00024A", "Groove Synthesis"),
            ("00024B", "Audiocipher Technologies LLC"),
            ("00024C", "Mellotron Inc."),
            ("00024D", "Hologram Electronics LLC"),
            ("00024E", "iCON Americas, LLC"),
            ("00024F", "Singular Sound"),
            ("000250", "Genovation Inc"),
            ("000251", "Method Red"),
            ("000252", "Brain Inventions"),
            ("000253", "Synervoz Communications Inc."),
            ("000254", "Hypertriangle Inc"),

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
            ("002014", "Celco/ Electrosonic"),
            ("002015", "ADB"),
            ("002016", "Marshall Products Limited"),
            ("002017", "DDA"),
            ("002018", "BSS Audio Ltd."),
            ("002019", "MA Lighting Technology"),
            ("00201A", "Fatar SRL c/o Music Industries"),
            ("00201B", "QSC Audio Products Inc."),
            ("00201C", "Artisan Clasic Organ Inc."),
            ("00201D", "Orla Spa"),
            ("00201E", "Pinnacle Audio (Klark Teknik PLC)"),
            ("00201F", "TC Electronics"),
            ("002020", "Doepfer Musikelektronik GmbH"),
            ("002021", "Creative ATC / E-mu"),
            ("002022", "Seyddo/Minami"),
            ("002023", "LG Electronics (Goldstar)"),
            ("002024", "Midisoft sas di M.Cima & C"),
            ("002025", "Samick Musical Inst. Co. Ltd."),
            ("002026", "Penny and Giles (Bowthorpe PLC)"),
            ("002027", "Acorn Computer"),
            ("002028", "LSC Electronics Pty. Ltd."),
            ("002029", "Focusrite/Novation"),
            ("00202A", "Samkyung Mechatronics"),
            ("00202B", "Medeli Electronics Co."),
            ("00202C", "Charlie Lab SRL"),
            ("00202D", "Blue Chip Music Technology"),
            ("00202E", "BEE OH Corp"),
            ("00202F", "LG Semicon America"),
            ("002030", "TESI"),
            ("002031", "EMAGIC"),
            ("002032", "Behringer GmbH"),
            ("002033", "Access Music Electronics"),
            ("002034", "Synoptic"),
            ("002035", "Hanmesoft"),
            ("002036", "Terratec Electronic GmbH"),
            ("002037", "Proel SpA"),
            ("002038", "IBK MIDI"),
            ("002039", "IRCAM"),
            ("00203A", "Propellerhead Software"),
            ("00203B", "Red Sound Systems Ltd"),
            ("00203C", "Elektron ESI AB"),
            ("00203D", "Sintefex Audio"),
            ("00203E", "MAM (Music and More)"),
            ("00203F", "Amsaro GmbH"),
            ("002040", "CDS Advanced Technology BV (Lanbox)"),
            ("002041", "Mode Machines (Touched By Sound GmbH)"),
            ("002042", "DSP Arts"),
            ("002043", "Phil Rees Music Tech"),
            ("002044", "Stamer Musikanlagen GmbH"),
            ("002045", "Musical Muntaner S.A. dba Soundart"),
            ("002046", "C-Mexx Software"),

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
}
