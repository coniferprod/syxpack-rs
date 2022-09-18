use std::path::{Path, PathBuf};
use std::io::prelude::*;
use std::fs;
use std::env;
use syxpack::{Message, UniversalKind, message_count, split_messages};

fn main() {
    let args: Vec<String> = env::args().collect();
    let input_file = &args[1];

    let path = Path::new(input_file);
    let display = path.display();

    let mut f = fs::File::open(&input_file).expect("no file found");

    /*
    let metadata = fs::metadata(&input_file).expect("unable to read file metadata");
    let mut buffer = vec![0; metadata.len() as usize];
    f.read(&mut buffer).expect("buffer overflow");
    */

    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer).expect("unable to read file");

    let mut all_messages: Vec<Message> = Vec::new();
    let count = message_count(buffer.to_vec());
    if count >= 1 {
        if count == 1 {
            all_messages.push(Message::new(buffer.to_vec()));
        }
        else {
            let messages = split_messages(buffer.to_vec());
            for message in messages {
                all_messages.push(Message::new(message));
            }
        }
    };
    //println!("Messages found: {}", count);

    let mut number = 1;
    for message in all_messages {
        println!("Message {} of {}", number, count);
        identify(&message);
        println!();
        number += 1;
    }
}

fn identify(message: &Message) {
    //let message = syxpack::Message::new(data.to_vec());
    match message {
        Message::ManufacturerSpecific(manufacturer, payload) => {
            println!("Manufacturer: {}, payload = {} bytes", manufacturer, payload.len());
        },
        Message::Universal(kind, sub_id1, sub_id2, payload) => {
            println!("Universal, kind: {:?}, {:X} {:X}, payload = {} bytes",
                match kind {
                    UniversalKind::NonRealTime => "Non-Real-time",
                    UniversalKind::RealTime => "Real-time",
                },
                sub_id1, sub_id2, payload.len());
        },
    }
}
