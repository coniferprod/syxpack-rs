# syxpack

Helpers for processing MIDI System Exclusive messages.

## Notes about parsing MIDI System Exclusive messages

MIDI System Exclusive messages always start with the status byte 0xF0.
Usually they are terminated with the status byte 0xF7, but according to
the MIDI specification it can be replaced with any other status byte
except a Real-Time message. However, this seems to be quite rare. Still,
technically the block between 0xF0 and 0xF7 could contain Timing Clock
(0xF8) or Active Sensing (0xFE) messages.

For simplicity, and because these other messages practically never occur
in System Exclusive dumps from synthesizers, the SyxPack library only
deals with System Exclusive messages as being strictly delimited by 0xF0 and 0xF7.

Some synthesizers pack multiple System Exclusive messages into the same
dump. SyxPack assumes that the `Vec<u8>` passed to `Message::new` contains
only one System Exclusive message delimited by 0xF0 and 0xF7. If there are
multiple messages, the payload will end up containing the actual message data
and all the other data up to the final terminating 0xF7.

The `message_count` function returns the number of messages in the byte vector,
based on the number of 0xF7 terminator bytes found. If this function returns
a value greater than one, you should first split the vector into multiple vectors
by the 0xF7 terminator, using the `split_messages` function:

    use syxpack::Message;

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
    println!("Messages found: {}", count);

