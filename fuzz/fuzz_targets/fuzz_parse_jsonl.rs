#![no_main]

use glhf::ingest::parse_jsonl_file;
use libfuzzer_sys::fuzz_target;
use std::io::Write;

fuzz_target!(|data: &[u8]| {
    // Write fuzzed bytes to a temp file, then parse
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fuzz.jsonl");
    {
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(data).unwrap();
    }

    // Should never panic — always Ok with 0+ docs
    let _ = parse_jsonl_file(&path);
});
