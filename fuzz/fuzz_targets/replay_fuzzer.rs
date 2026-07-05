#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    if let Ok(report) = gridpulse::replay(data) {
        let mut fingerprint = report.fingerprint;
        for batch in &report.batches {
            let score = gridpulse::analytics::score_batch(batch);
            fingerprint = fingerprint
                .wrapping_add(score.voltage_samples as u32)
                .wrapping_add(score.route_hops as u32)
                .wrapping_add(score.script_weight);
        }
        std::hint::black_box(fingerprint);
    }
});
