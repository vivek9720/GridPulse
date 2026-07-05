use crate::measurements::{ReadingValue, Readout};
use crate::model::DecodedBatch;
use crate::scripts::{instruction_weight, ScriptInstruction};
use crate::template::FieldKind;

#[derive(Debug, Clone, Default)]
pub struct BatchScore {
    pub voltage_samples: usize,
    pub outage_indicators: usize,
    pub route_hops: usize,
    pub script_weight: u32,
    pub warning_count: usize,
}

pub fn score_batch(batch: &DecodedBatch) -> BatchScore {
    let mut score = BatchScore::default();
    score.warning_count = batch.diagnostics.warnings.len();
    for reading in &batch.readings {
        score_reading(reading, &mut score);
    }
    for route in &batch.route_events {
        score.route_hops += route.hops.len();
    }
    for script in &batch.script_programs {
        for instruction in &script.instructions {
            score.script_weight += instruction_weight(instruction);
            score_nested(instruction, &mut score);
        }
    }
    score
}

fn score_reading(reading: &Readout, score: &mut BatchScore) {
    for value in &reading.values {
        match value {
            ReadingValue::Integer {
                kind: FieldKind::Voltage,
                ..
            } => score.voltage_samples += 1,
            ReadingValue::Bitmap {
                kind: FieldKind::TamperFlags,
                bits,
            } if bits & 0x03 != 0 => score.outage_indicators += 1,
            ReadingValue::Integer {
                kind: FieldKind::OutageCause,
                value,
                ..
            } if *value != 0 => score.outage_indicators += 1,
            _ => {}
        }
    }
}

fn score_nested(instruction: &ScriptInstruction, score: &mut BatchScore) {
    match instruction {
        ScriptInstruction::IfQualityBelow { body, .. }
        | ScriptInstruction::Repeat { body, .. } => {
            for nested in body {
                score.script_weight += instruction_weight(nested);
                score_nested(nested, score);
            }
        }
        _ => {}
    }
}
