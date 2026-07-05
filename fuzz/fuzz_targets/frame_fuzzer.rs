#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    if let Ok(batch) = gridpulse::parse(data) {
        let _ = gridpulse::analytics::score_batch(&batch);
    }
});
