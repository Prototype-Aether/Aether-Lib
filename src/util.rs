use rand::{rngs::OsRng, RngCore};

/// Compile a 32-bit value into vector of bytes
///
/// # Arguments
///
/// * `nu32`    -   A `u32` integer value
///
/// # Examples
///
/// ```
/// use aether_lib::util::compile_u32;
/// let bytes: Vec<u8> = compile_u32(32);
/// ```
pub fn compile_u32(nu32: u32) -> Vec<u8> {
    vec![
        (nu32 >> 24) as u8,
        (nu32 >> 16) as u8,
        (nu32 >> 8) as u8,
        nu32 as u8,
    ]
}

/// Compile a 16-bit value into vector of bytes
///
/// # Arguments
///
/// * `nu16`    -   A `u16` integer value
///
/// # Examples
///
/// ```
/// use aether_lib::util::compile_u16;
/// let bytes: Vec<u8> = compile_u16(3242);
/// ```
pub fn compile_u16(nu16: u16) -> Vec<u8> {
    vec![(nu16 >> 8) as u8, nu16 as u8]
}

/// Generate a cryptographically secure random nonce of the given size in bytes
///
/// # Arguments
///
/// * `size`    -   Size of the nonce in bytes
///
/// # Examples
///
/// ```
/// use aether_lib::util::gen_nonce;
/// // to generate a 16 bytes nonce
/// let nonce = gen_nonce(16);
/// ```
pub fn gen_nonce(size: usize) -> Vec<u8> {
    let mut buf = vec![0u8; size];
    OsRng.fill_bytes(&mut buf);
    buf
}

pub fn xor(lhs: Vec<u8>, rhs: Vec<u8>) -> Vec<u8> {
    lhs.iter().zip(rhs).map(|(x, y)| x ^ y).collect()
}
