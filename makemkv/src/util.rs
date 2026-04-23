pub fn u32s_to_u64(high: u32, low: u32) -> u64 {
    ((high as u64) << 32) + (low as u64)
}

pub fn u64_to_le_u32(value: u64) -> [u32; 2] {
    let bytes = value.to_le_bytes();
    [
        u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
    ]
}

/// Converts a slice into const sized array
///
/// This WILL panic if the slice doesn't contain at least 4 values
pub fn u32_const_slice(slice: &[u8]) -> [u8; 4] {
    [slice[0], slice[1], slice[2], slice[3]]
}
