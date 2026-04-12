use std::collections::HashMap;

use crate::util::u32_const_slice;

pub struct LanguageData {
    table: HashMap<u32, String>,
}

impl LanguageData {
    pub fn new(data: &[u8]) -> Self {
        let mut table = HashMap::new();

        let count_bytes = [data[0], data[1], data[2], data[3]];
        let count = u32::from_le_bytes(count_bytes) as usize;

        for i in (4..(4 * count + 1)).step_by(4) {
            let id = u32::from_le_bytes(u32_const_slice(&data[i..i + 4]));
            let offset =
                u32::from_le_bytes(u32_const_slice(&data[i + (count * 4)..i + (count * 4) + 4]));

            let start = (offset * 4) as usize;
            let end = start + find_utf16_terminator(&data[start..]).unwrap_or_default();
            let value = String::from_utf16_lossy(&u8_to_u16(&data[start..end])[..]);
            table.insert(id, value);
        }

        Self { table }
    }

    pub fn get(&self, id: u32) -> Option<String> {
        self.table.get(&id).cloned()
    }
}

fn find_utf16_terminator(data: &[u8]) -> Option<usize> {
    for i in (0..data.len()).step_by(2) {
        if data[i..i + 1] == [0, 0] {
            return Some(i);
        }
    }

    None
}

fn u8_to_u16(data: &[u8]) -> Vec<u16> {
    let mut u16s = Vec::new();
    for i in (0..data.len()).step_by(2) {
        u16s.push(u16::from_le_bytes([data[i], data[i + 1]]));
    }
    u16s
}
