use std::io::prelude::*;
use std::fs;
use std::env;

use syxpack::{Message, message_count, read_file};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("usage: syxextract infile outfile");
        std::process::exit(1);
    }

    let input_file = &args[1];
    if let Some(buffer) = read_file(&input_file) {
        if message_count(&buffer) > 1 {
            println!("More than one System Exclusive message found. Please use syxsplit to separate them.");
        }
        else {
            match Message::new(&buffer) {
                // At this point, the SysEx delimiters and the manufacturer byte(s)
                // have already been stripped off. What's left is the payload.
                // For example, if the original message was "F0 42 30 28 54 02 ... 5C F7",
                // then the payload is "30 28 54 02 ... 5C".
                Ok(Message::ManufacturerSpecific { payload, .. })
                | Ok(Message::Universal { payload, .. }) => {
                    let output_file = &args[2];
                    let mut f = fs::File::create(&output_file).expect("to create file");
                    f.write_all(&payload).expect("to write to file");
                },
                Err(e) => {
                    eprintln!("Error: {:?}", e);
                }
            };
        }
    }
}
