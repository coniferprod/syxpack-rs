use std::path::Path;
use std::io::prelude::*;
use std::fs;
use std::env;

use syxpack::{message_count, split_messages, read_file};

fn main() {
    let args: Vec<String> = env::args().collect();
    let input_file = &args[1];

    let mut verbose = false;
    if args.len() > 2 {
        verbose = &args[2] == "--verbose";
    }

    let path = Path::new(input_file);
    if let Some(buffer) = read_file(&input_file) {
        let count = message_count(&buffer);
        if verbose {
            println!("Found {} messages", count);
        }

        if count > 1 {
            let messages = split_messages(buffer.to_vec());
            for (i, message) in messages.iter().enumerate() {
                let output_filename = format!(
                    "{}-{:0>3}.{}",
                    path.file_stem().unwrap().to_str().unwrap(),
                    i + 1,
                    path.extension().unwrap().to_str().unwrap());
                if verbose {
                    println!("Writing {}", output_filename);
                }
                let mut file = fs::File::create(output_filename)
                    .expect("unable to create file");
                file.write_all(message).expect("unable to write file");
            }
        }
        else {
            if verbose {
                println!("No messages found");
            }
        }
    }
}
