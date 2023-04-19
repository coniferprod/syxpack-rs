use std::env;
use syxpack::{Message, UniversalKind, message_count, split_messages, read_file};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("usage: syxident file");
        std::process::exit(1);
    }

    let input_file = &args[1];
    if let Some(buffer) = read_file(input_file) {
        let mut all_messages: Vec<Message> = Vec::new();
        let count = message_count(&buffer);
        if count >= 1 {
            if count == 1 {
                all_messages.push(Message::new(&buffer).ok().unwrap());
            }
            else {
                let messages = split_messages(buffer.to_vec());
                for message in messages {
                    all_messages.push(Message::new(&message).ok().unwrap());
                }
            }
        };

        let mut number = 1;
        for message in all_messages {
            println!("Message {} of {}", number, count);
            identify(&message);
            println!("MD5 digest: {:x}", message.digest());
            println!();
            number += 1;
        }
    }
}

fn identify(message: &Message) {
    match message {
        Message::ManufacturerSpecific { manufacturer, payload } => {
            println!("Manufacturer: {}, payload = {} bytes", manufacturer, payload.len());
        },
        Message::Universal { kind, sub_id1, sub_id2, payload } => {
            println!("Universal, kind: {:?}, {:X} {:X}, payload = {} bytes",
                match kind {
                    UniversalKind::NonRealTime => "Non-Real-time",
                    UniversalKind::RealTime => "Real-time",
                },
                sub_id1, sub_id2, payload.len());
        },
    }
}
