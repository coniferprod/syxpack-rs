use std::io::prelude::*;
use std::fs;
use std::env;
use std::path::{Path, PathBuf};

use syxpack::{Message, message_count};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("usage: syxextract infile outfile");
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

    let message = Message::new(buffer);
    match message {
        Message::ManufacturerSpecific(_, payload) => {
            // At this point, the SysEx delimiters and the manufacturer byte(s)
            // have already been stripped off. What's left is the payload.
            // For example, if the original message was "F0 42 30 28 54 02 ... 5C F7",
            // then the payload is "30 28 54 02 ... 5C".
            let output_file = &args[2];
            let mut f = fs::File::create(&output_file).expect("unable to create file");
            f.write_all(&payload).expect("unable to write to file");
        },
        Message::Universal(_, _, _, payload) => {
            let output_file = &args[2];
            let mut f = fs::File::create(&output_file).expect("unable to create file");
            f.write_all(&payload).expect("unable to write to file");
        }
    }
}
