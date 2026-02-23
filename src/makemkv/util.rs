pub fn u32s_to_u64(high: u32, low: u32) -> u64 {
    ((high as u64) << 32) + (low as u64)
}
