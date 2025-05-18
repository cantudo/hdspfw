use crate::{packet::{self, Command, Packet}, utils::udiv_ceil};

/// Number of bytes in a screen buffer
pub const SCREEN_BUFFER_NBYTES: usize = 240 * 50;
/// Maximum payload size needed to send a screen buffer
pub const SCREEN_BUFFER_PACKETS_MAX_PAYLOAD: usize = 1 + 240 + SCREEN_BUFFER_NBYTES;
/// Maximum number of packets needed to send a screen buffer
pub const SCREEN_BUFFER_MAX_PACKETS: usize = (SCREEN_BUFFER_PACKETS_MAX_PAYLOAD + packet::MAX_PACKET_PAYLOAD_SIZE - 1) / packet::MAX_PACKET_PAYLOAD_SIZE;

pub fn screen_buffer_to_packets(screen_buffer: [[u8; 50]; 240], selected_lines: &[u8]) -> ([Packet; SCREEN_BUFFER_MAX_PACKETS], usize) {
    let n_lines_changed = selected_lines.len();
    
    // let total_payload_size = 1 + n_lines_changed + n_lines_changed * 50;
    
    let mut total_payload = [0x00; SCREEN_BUFFER_PACKETS_MAX_PAYLOAD];
    let mut payload_ptr = 0;

    total_payload[payload_ptr] = n_lines_changed as u8;
    payload_ptr += 1; 

    for i in 0..n_lines_changed {
        total_payload[payload_ptr] = selected_lines[i];
        payload_ptr += 1;
    }

    for i in 0..n_lines_changed {
        let line = selected_lines[i];
        let line_data = screen_buffer[line as usize];
        
        for j in 0..50 {
            total_payload[payload_ptr] = line_data[j];
            payload_ptr += 1;
        }
    }

    let payload_size = payload_ptr;

    let mut packets = [Packet::new(); SCREEN_BUFFER_MAX_PACKETS];
    

    let n_packets = udiv_ceil(payload_size, packet::MAX_PACKET_PAYLOAD_SIZE);
    let mut payload_ptr = 0;
    
    while payload_ptr < payload_size {
        let packets_ptr = payload_ptr / packet::MAX_PACKET_PAYLOAD_SIZE;
        
        let mut packet = &mut packets[packets_ptr];
        packet.set_command(Command::CMD_SCREEN_BUFFER.into());
        
        let next_payload_ptr = (payload_ptr + packet::MAX_PACKET_PAYLOAD_SIZE).min(payload_size);
        packet.set_payload(total_payload[payload_ptr..next_payload_ptr].as_ref());
        
        payload_ptr = next_payload_ptr;
    }
    
    (packets, n_packets)
}

// #[test]
// fn test_screen_buffer_to_packets() {
//     let screen_buffer = [[0; 50]; 240];
//     let mut selected_lines = [0x00; 240];
//     for i in 0..240 {
//         selected_lines[i] = i as u8;
//     }
    
//     let (packets, n_packets) = screen_buffer_to_packets(screen_buffer, &selected_lines);
    
//     assert_eq!(n_packets, 3);
    
//     for i in 0..n_packets {
//         assert_eq!(packets[i].command(), Command::CMD_SCREEN_BUFFER.into());
//         assert_eq!(packets[i].size, SCREEN_BUFFER_PACKETS_MAX_PAYLOAD);
//     }
// }
