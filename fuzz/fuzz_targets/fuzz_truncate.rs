#![no_main]

use glhf::utils::truncate_text;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() < 2 {
        return;
    }

    // First 2 bytes as max_len, rest as content
    let max_len = u16::from_le_bytes([data[0], data[1]]) as usize;
    let content = String::from_utf8_lossy(&data[2..]);

    // Should never panic on any input
    let _ = truncate_text(&content, max_len);
});
