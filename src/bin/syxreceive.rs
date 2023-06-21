// Reads a line of input produced by ReceiveMIDI
// Filters out everything else but MIDI System Exclusive messages,
// and interprets the message data.

use std::io::Write;
use std::fs::File;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    loop {
        let mut input = String::new();
        match std::io::stdin().read_line(&mut input) {
            Ok(len) => if len == 0 {
                return;
            }
            else {
                let parts: Vec<&str> = input.split_whitespace().collect();

                // We want at least "system-exclusive", "hex" or "dec", and one byte
                if parts.len() < 3 {
                    continue;
                }

                // Only deal with SysEx:
                if parts[0] == "system-exclusive" {
                    // Get the base of the byte strings.
                    let base = if parts[1] == "hex" { 16 } else { 10 };

                    let mut data: Vec<u8> = Vec::new();

                    for part in &parts[2..] {
                        match u8::from_str_radix(part, base) {
                            Ok(b) => data.push(b),
                            Err(_) => {
                                //eprintln!("Error in byte string '{}': {}", part, e);
                                continue;
                            }
                        }
                    }

                    // Add the MIDI System Exclusive delimiters:
                    data.insert(0, 0xf0);
                    data.push(0xf7);

                    println!("Received {} bytes of System Exclusive data", data.len());

                    // Write the data into a file named by the current timestamp.
                    let now = SystemTime::now();
                    let epoch_now = now
                        .duration_since(UNIX_EPOCH)
                        .expect("System time should be after Unix epoch");
                    let filename = format!("{:?}.syx", epoch_now.as_secs());
                    let path = Path::new(&filename);
                    let display = path.display();
                    let mut file = match File::create(&path) {
                        Err(why) => panic!("couldn't create {}: {}", display, why),
                        Ok(file) => file,
                    };

                    match file.write_all(&data) {
                        Err(why) => panic!("couldn't write to {}: {}", display, why),
                        Ok(_) => { },
                    }
                }
            },
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
    }
}
