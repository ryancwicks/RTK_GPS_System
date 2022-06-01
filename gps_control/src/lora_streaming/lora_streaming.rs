use tokio_util::codec::{Encoder, Decoder};
use bytes::{Buf, BytesMut};
//use miniz_oxide::inflate::decompress_to_vec;
//use miniz_oxide::deflate::compress_to_vec;

const MAX: usize = 1024;

#[derive(PartialEq,Debug)]
pub enum LORAMessage {
    Data(Vec<u8>),
    SignalStrength (f32), 
}


/// This structure handles the serial connection to a LORA transiever.
/// The data is compressed before it is sent over the air. 
/// 
/// The protocol is 2 bytes size (excluding these two bytes) followed by a single byte packet id, followed by the data, LE encoded.
/// 
/// Data - packet type 0
/// SignalStrength - packet type 1
/// 
pub struct LORAStream {

}

impl Encoder<LORAMessage> for LORAStream {
    type Error = std::io::Error;

    fn encode(&mut self, item: LORAMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match item {
            LORAMessage::Data(data) => {

                //let compressed_data = compress_to_vec(&data, 8);

                let size = 3 + data.len(); //size(2) + packet id (1) + data size

                // Don't send a string if it is longer than the other end will
                // accept.
                if size > MAX {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("Frame of length {} is too large.", size)
                    ));
                }

                // Convert the length into a byte array.
                // The cast to u16 cannot overflow due to the length check above.
                // +1 is to include the id tag.
                let len_slice = u16::to_le_bytes((data.len() + 1) as u16);  
                let id = [0];             

                // Reserve space in the buffer.
                dst.reserve(size);

                // Write the length and string to the buffer.
                dst.extend_from_slice(&len_slice);
                dst.extend_from_slice(&id);
                dst.extend_from_slice(&data);
                Ok(())
            },
            LORAMessage::SignalStrength(strength) => {
                let size = 7; //size (2) + packet id (1) + f32 signal strength (4)// Reserve space in the buffer.
                dst.reserve(size);
        
                let len_slice = u16::to_le_bytes(5 as u16);
                let id = [1]; 
                let strength_slice = f32::to_le_bytes(strength as f32);
                // Write the length and string to the buffer.
                dst.extend_from_slice(&len_slice);
                dst.extend_from_slice(&id);
                dst.extend_from_slice(&strength_slice);
                Ok(())

            }
        }
    }
}

impl Decoder for LORAStream {
    type Item = LORAMessage;
    type Error = Box<dyn std::error::Error>;

    fn decode(&mut self, src: &mut BytesMut
    ) -> Result<Option<Self::Item>, Self::Error> {

        if src.len() < 3 {
            // Not enough data to read length marker.
            return Ok(None);
        }

        // Read length marker.
        let mut length_bytes = [0u8; 2];
        length_bytes.copy_from_slice(&src[..2]);
        let length = u16::from_le_bytes(length_bytes) as usize;

        // Check that the length is not too large to avoid memory problems with bad communication.
        if length > MAX {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Frame of length {} is too large.", length)
            )));
        }

        if src.len() < 2 + length {
            // The full message has not yet arrived.
            //
            // We reserve more space in the buffer. This is not strictly
            // necessary, but is a good idea performance-wise.
            src.reserve(2 + length - src.len());

            // We inform the Framed that we need more bytes to form the next
            // frame.
            return Ok(None);
        }

        // Use advance to modify src such that it no longer contains
        // this frame.
        let id = src[2];
        let data = src[3..3 + (length-1)].to_vec();
        src.advance(2 + length);

        if id == 0 {
            //let uncompressed_data = match decompress_to_vec(data.as_slice()) {
            //    Ok(val) => val,
            //    Err(err) => {
            //        return Err(Box::new(std::io::Error::new(
            //            std::io::ErrorKind::InvalidData,
            //            format!("Decompression error: {:?}", err))
            //        ))
            //    }
            //};

            Ok(Some(LORAMessage::Data(data)))
        } else if id == 1 {
            if data.len() != 4 {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Insufficient data for signal strength.",
                )))
            }
            let strength_arr = data.try_into().unwrap(); //I've already checked the data is the correct length.
            let strength = f32::from_le_bytes(strength_arr);
            Ok(Some(LORAMessage::SignalStrength(strength)))
        } else {
            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid messsage type.",
            )))
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_signal_strength () {
        let mut codec = LORAStream{};
        let message = LORAMessage::SignalStrength(16.2);
        let mut binary_data = BytesMut::new();

        assert! (!codec.encode(message, &mut binary_data).is_err());

        let ret =  codec.decode(&mut binary_data).unwrap().unwrap();

        assert_eq! (ret, LORAMessage::SignalStrength(16.2));
    }

    #[test]
    fn test_data () {
        let mut codec = LORAStream{};
        let test_data: Vec<u8> = b"Hello World! \r\n The quick brown fox jupmed over the lazy brown dog.".to_vec();
        let message = LORAMessage::Data (test_data.clone());
        let mut binary_data = BytesMut::new();

        assert! (!codec.encode(message, &mut binary_data).is_err());

        let ret =  codec.decode(&mut binary_data).unwrap().unwrap();

        assert_eq! (ret, LORAMessage::Data(test_data));
    } 
}