#![no_main]

use glhf::config::decode_project_path;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &str| {
    // Should never panic on any string input
    let _ = decode_project_path(data);
});
