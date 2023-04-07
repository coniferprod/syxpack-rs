use std::io::prelude::*;
use std::fs;
use std::env;
use syxpack::{Message, UniversalKind, message_count, split_messages};
use std::path::{Path, PathBuf};

enum SectionKind {
    Initiator,
    Manufacturer,
    Universal,
    Payload,
    Terminator,
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
        std::process::exit(1);
    }

    let input_file = &args[1];

    let path = Path::new(input_file);
    let display = path.display();

    let mut f = fs::File::open(&input_file).expect("no file found");
    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer).expect("unable to read file");

    let count = message_count(buffer.to_vec());
    if count > 1 {
        println!("More than one System Exclusive message found. Please use syxsplit to separate them.");
        std::process::exit(1);
    }

    let message = Message::new(buffer.clone());
    let mut offset = 0;

    let mut sections: Vec<MessageSection> = Vec::new();
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

    for section in sections {
        println!("{}: {} ({})", section.offset, section.name, section.length);
    }
}
