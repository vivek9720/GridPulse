#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    if let Ok(program) = gridpulse::parse_script(data) {
        let mut weight = 0u32;
        for instruction in &program.instructions {
            weight = weight.wrapping_add(gridpulse::scripts::instruction_weight(instruction));
        }
        std::hint::black_box(weight);
    }
});
