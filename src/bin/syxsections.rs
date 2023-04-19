use std::io::prelude::*;
use std::env;
use std::path::Path;
use std::fmt;
use syxpack::{Message, UniversalKind, message_count, read_file};

enum SectionKind {
    Initiator,
    Manufacturer,
    Universal,
    Payload,
    Terminator,
}

impl fmt::Display for SectionKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match self {
            SectionKind::Initiator => "Message initiator",
            SectionKind::Manufacturer => "Manufacturer identifier",
            SectionKind::Universal => "Universal message identifier",
            SectionKind::Payload => "Message payload",
            SectionKind::Terminator => "Message terminator"
        })
    }
}

struct MessageSection {
    kind: SectionKind,
    name: String,
    offset: usize,  // offset from message start
    length: usize,  // length of section in bytes
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("usage: syxsections infile");
    }

    let mut sections: Vec<MessageSection> = Vec::new();

    let input_file = &args[1];
    if let Some(buffer) = read_file(&input_file) {
        if message_count(&buffer) > 1 {
            println!("More than one System Exclusive message found. Please use syxsplit to separate them.");
            std::process::exit(1);
        }
        let message = Message::new(&buffer);
        let mut offset = 0;

        sections.push(
            MessageSection {
                kind: SectionKind::Initiator,
                name: "System Exclusive Initiator".to_string(),
                offset: offset,
                length: 1,
            }
        );

        offset += 1;

        match message {
            Ok(Message::ManufacturerSpecific { manufacturer, payload }) => {
                sections.push(
                    MessageSection {
                        kind: SectionKind::Manufacturer,
                        name: "Manufacturer".to_string(),
                        offset: offset,
                        length: manufacturer.to_bytes().len(),
                    }
                );
                offset += manufacturer.to_bytes().len();
                sections.push(
                    MessageSection {
                        kind: SectionKind::Payload,
                        name: "Message Payload".to_string(),
                        offset: offset,
                        length: payload.len(),
                    }
                )
            },
            Ok(Message::Universal { kind, sub_id1, sub_id2, payload }) => {
                sections.push(
                    MessageSection {
                        kind: SectionKind::Universal,
                        name: "Universal".to_string(),
                        offset: offset,
                        length: 3,
                    }
                );

                println!("Universal, kind: {:?}, {:X} {:X}, payload = {} bytes",
                    match kind {
                        UniversalKind::NonRealTime => "Non-Real-time",
                        UniversalKind::RealTime => "Real-time",
                    },
                    sub_id1, sub_id2, payload.len());
            },
            Err(e) => {
                println!("Error in message: {:?}", e);
            }
        }

        sections.push(
            MessageSection {
                kind: SectionKind::Terminator,
                name: "System Exclusive Terminator".to_string(),
                offset: buffer.len() - 1,
                length: 1,
            }
        );
    }

    for section in sections {
        println!("{:06X}: {} ({}, {} bytes)", section.offset, section.name, section.kind, section.length);
    }
}
