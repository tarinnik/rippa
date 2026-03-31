pub fn u32s_to_u64(high: u32, low: u32) -> u64 {
    ((high as u64) << 32) + (low as u64)
}

pub fn u64_to_le_u32(value: u64) -> [u32; 2] {
    [(value & 0xFFFFFF) as u32, ((value >> 32) & 0xFFFFFF) as u32]
}
