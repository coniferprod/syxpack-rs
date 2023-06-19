// Reads a line of input produced by ReceiveMIDI
// filters out everything else but MIDI System Exclusive messages,
// and interprets the message data.

use std::io::prelude::*;
use std::fs;
use std::env;

use syxpack::{Message, message_count, read_file};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {  // read from standard input
        loop {
            let mut input = String::new();
            match std::io::stdin().read_line(&mut input) {
                Ok(len) => if len == 0 {
                    return;
                }
                else {
                    process_line(&input);
                },
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            }
        }
    }
    else if args.len() == 2 {  // read from input file
        let input_file = &args[1];

    }
    else {
        println!("usage: syxreceive [infile]");
        std::process::exit(1);
    }
}

fn process_line(line: &str) {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts[0] == "system-exclusive" {
        let mut data: Vec<u8> = Vec::new();

        for part in &parts[1..] {
            match u8::from_str_radix(part, 16) {
                Ok(b) => data.push(b),
                Err(e) => {
                    eprintln!("Error in hex string: {}", e);
                    return;
                }
            }
        }

        println!("data = {:?}", data);
    }
}
