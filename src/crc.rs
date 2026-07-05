pub fn crc16_gridpulse(data: &[u8]) -> u16 {
    let mut crc = 0x6d0fu16;
    for &byte in data {
        crc ^= byte as u16;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xa001;
            } else {
                crc >>= 1;
            }
        }
    }
    crc
}

pub fn folded_checksum(data: &[u8]) -> u32 {
    let mut acc = 0x9e37_79b9u32;
    for chunk in data.chunks(4) {
        let mut lane = [0u8; 4];
        lane[..chunk.len()].copy_from_slice(chunk);
        acc = acc.rotate_left(5) ^ u32::from_le_bytes(lane);
        acc = acc.wrapping_mul(0x85eb_ca6b);
    }
    acc
}
