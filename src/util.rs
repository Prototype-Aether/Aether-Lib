// compile the packet structure into raw bytes
pub fn compile_u32(nu32: u32) -> Vec<u8> {
    let mut u32_vec = Vec::<u8>::new();
    u32_vec.push((nu32 >> 24) as u8);
    u32_vec.push((nu32 >> 16) as u8);
    u32_vec.push((nu32 >> 8) as u8);
    u32_vec.push(nu32 as u8);
    u32_vec
}

pub fn compile_u16(nu16: u16) -> Vec<u8> {
    let mut u16_vec = Vec::<u8>::new();
    u16_vec.push((nu16 >> 8) as u8);
    u16_vec.push(nu16 as u8);
    u16_vec
}
