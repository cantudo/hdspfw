#![allow(dead_code)]

/// Maximum size of a packet
pub const MAX_PACKET_SIZE: usize = 256;
/// Maximum size of the payload inside a packet
pub const MAX_PACKET_PAYLOAD_SIZE: usize = MAX_PACKET_SIZE - 1;

pub enum Command {
    CMD_INVALID = 0x00,
    CMD_SCREEN_BUFFER = 0x01,
    CMD_ACK = 0x02,
}

impl From<Command> for u8 {
    fn from(command: Command) -> Self {
        command as u8
    }
}

impl From<u8> for Command {
    fn from(command: u8) -> Self {
        match command {
            0x01 => Command::CMD_SCREEN_BUFFER,
            0x02 => Command::CMD_ACK,
            _ => Command::CMD_INVALID,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Packet {
    pub data: [u8; MAX_PACKET_SIZE],
    pub size: usize,
}

impl Packet  {
    pub fn new() -> Self {
        Self {
            data: [0; MAX_PACKET_SIZE],
            size: 0,
        }
    }

    pub fn command(&self) -> u8 {
        self.data[0]
    }
    
    pub fn set_command(&mut self, command: u8) {
        self.data[0] = command;
    }

    pub fn get_payload(&self) -> &[u8] {
        &self.data[1..self.size]
    }
    
    pub fn set_payload(&mut self, payload: &[u8]) {
        let size = payload.len();
        if size > MAX_PACKET_SIZE - 1 {
            panic!("Payload size exceeds maximum packet size");
        }
        self.size = size + 1;
        self.data[1..self.size].copy_from_slice(payload);
    }

    // Decode COBS (http://www.stuartcheshire.org/papers/cobsforton.pdf)
    pub fn from_cobs(encoded_data: &[u8]) -> Result<Self, ()> {
        // let mut packet = Packet::new(data.len());
        let mut packet = Packet::new();
        
        let decoded_data = packet.data_mut();
        let mut encoded_ptr = 1;
        let mut next_idx = encoded_data[0] as usize;
        let mut block_size = encoded_data[0] as usize - 1;
        let mut decoded_ptr = 0;


        while encoded_ptr < encoded_data.len() - 1 {
            let mut value = encoded_data[encoded_ptr];

            if encoded_ptr == next_idx {
                let offset = value as usize;
                next_idx += offset as usize;
                

                if block_size < 0xFE {
                    value = 0x00;
                    block_size = offset - 1;
                } else { // It is another pointer
                    encoded_ptr += 1;
                    block_size = offset - 1;
                    continue;
                }

                if next_idx >= encoded_data.len() {
                    return Err(());
                }
            }

            if decoded_ptr >= decoded_data.len() {
                return Err(());
            }
            decoded_data[decoded_ptr] = value;
            
            decoded_ptr += 1;
            encoded_ptr += 1;

        }

        packet.size = decoded_ptr;

        Ok(packet)
    }
    
    /*
        Encode COBS (http://www.stuartcheshire.org/papers/cobsforton.pdf)
        Returns a tuple of the encoded data and the size of the encoded data
     */
    pub fn to_cobs_slice(&self) -> ([u8; MAX_PACKET_SIZE+4], usize) {
        let mut encoded_data = [0; MAX_PACKET_SIZE+4];
        let mut encoded_ptr = 1usize;
        
        let mut decoded_ptr = 0usize;
        let decoded_data = self.data();

        let mut last_ptr_idx = 0usize;
        let mut block_size = 1u8;

        while decoded_ptr < self.size {
            let value = decoded_data[decoded_ptr];

            if block_size == 0xFF {
                encoded_data[last_ptr_idx] = block_size;

                last_ptr_idx = encoded_ptr;
                
                encoded_ptr += 1;

                block_size = 1;
                
                continue;
            }

            if value == 0x00 {
                encoded_data[last_ptr_idx] = block_size;

                last_ptr_idx = encoded_ptr;
                
                encoded_ptr += 1;
                decoded_ptr += 1;

                block_size = 1;
                
                continue;
            } else {
                encoded_data[encoded_ptr] = value;
                
                encoded_ptr += 1;
                decoded_ptr += 1;
                block_size += 1;
            }

        }

        if block_size > 0 {
            encoded_data[last_ptr_idx] = block_size;
        }
        encoded_data[encoded_ptr] = 0x00;
        encoded_ptr += 1;

        
        (encoded_data, encoded_ptr)
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }
}

// impl From<&[u8]> for Packet {
//     fn from(data: &[u8]) -> Self {
//         let mut packet = Packet::new(data.len());
//         packet.data.copy_from_slice(data);
//         packet
//     }
// }

#[test]
fn test_packet() {
    let encoded_data_good = [0x02, 0x23, 0x03, 0xD4, 0x81, 0x02, 0xFA, 0x00];
    let packet = Packet::from_cobs(&encoded_data_good).unwrap();
    
    assert_eq!(&packet.data()[..packet.size], &[0x23, 0x00, 0xD4, 0x81, 0x00, 0xFA]);

    let (encoded_data, encoded_size) = packet.to_cobs_slice();
    assert_eq!(encoded_data[0..encoded_size], encoded_data_good);


    let mut encoded_data_good = [0x00; 258];
    for i in 1..255 {
        encoded_data_good[i] = i as u8;
    }
    encoded_data_good[0] = 0xFF;
    encoded_data_good[255] = 0x02;
    encoded_data_good[256] = 0xFF;
    encoded_data_good[257] = 0x00;

    let packet = Packet::from_cobs(&encoded_data_good).unwrap();

    let mut decoded_data = [0x00; 255];
    for i in 0..255 {
        decoded_data[i] = i as u8 + 1;
    }
    assert_eq!(&packet.data()[..packet.size], &decoded_data);
    
    let (encoded_data, encoded_size) = packet.to_cobs_slice();
    assert_eq!(encoded_data[0..encoded_size], encoded_data_good);

    
    let mut encoded_data_good = [0x00; 258];
    for i in 1..255 {
        encoded_data_good[i] = i as u8 + 1;
    }
    encoded_data_good[0] = 0xFF;
    encoded_data_good[255] = 0x01;
    encoded_data_good[256] = 0x01;
    encoded_data_good[257] = 0x00;

    let packet = Packet::from_cobs(&encoded_data_good).unwrap();
    let mut decoded_data = [0x00; 255];
    for i in 0..254 {
        decoded_data[i] = i as u8 + 2;
    }
    assert_eq!(&packet.data()[..packet.size], &decoded_data);

    let (encoded_data, encoded_size) = packet.to_cobs_slice();
    assert_eq!(encoded_data[0..encoded_size], encoded_data_good);

    let mut encoded_data_good = [0x00; 257];
    for i in 1..254 {
        encoded_data_good[i] = i as u8 + 2;
    }
    encoded_data_good[0] = 0xFE;
    encoded_data_good[254] = 0x02;
    encoded_data_good[255] = 0x01;
    encoded_data_good[256] = 0x00;
    let packet = Packet::from_cobs(&encoded_data_good).unwrap();

    let mut decoded_data = [0x00; 255];
    for i in 0..253 {
        decoded_data[i] = i as u8 + 3;
    }
    decoded_data[253] = 0x00;
    decoded_data[254] = 0x01;
    assert_eq!(&packet.data()[..packet.size], &decoded_data);

    let (encoded_data, encoded_size) = packet.to_cobs_slice();
    assert_eq!(encoded_data[0..encoded_size], encoded_data_good);
}