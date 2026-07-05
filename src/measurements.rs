use crate::cursor::ByteCursor;
use crate::error::{GridError, Result};
use crate::model::Diagnostics;
use crate::template::{FieldCodec, FieldKind, FieldSpec, Template, TemplateBank};

#[derive(Debug, Clone)]
pub struct Readout {
    pub meter_id: u32,
    pub template_id: u16,
    pub template_revision: u8,
    pub timestamp_delta: i32,
    pub values: Vec<ReadingValue>,
    pub quality: u8,
}

#[derive(Debug, Clone)]
pub enum ReadingValue {
    Integer {
        kind: FieldKind,
        value: i64,
        scale: i8,
    },
    Bitmap {
        kind: FieldKind,
        bits: u64,
    },
    Null {
        kind: FieldKind,
    },
}

pub fn decode_reading(
    payload: &[u8],
    bank: &mut TemplateBank,
    diagnostics: &mut Diagnostics,
) -> Result<Readout> {
    let mut cursor = ByteCursor::new(payload);
    let meter_id = cursor.read_u32_le()?;
    let template_id = cursor.read_u16_le()?;
    let template_revision = cursor.read_u8()?;
    let flags = cursor.read_u8()?;
    let quality = cursor.read_u8()?;
    let timestamp_delta = cursor.read_svar_i32().unwrap_or(0);

    let template = if flags & 0x01 != 0 {
        unsafe { bank.reuse_cached_template(template_id, template_revision) }
    } else {
        bank.lookup(template_id, template_revision)
    }
    .ok_or(GridError::TemplateMissing {
        id: template_id,
        revision: template_revision,
    })?;

    let values = decode_values(template, &mut cursor)?;
    diagnostics.readings_decoded += 1;
    Ok(Readout {
        meter_id,
        template_id,
        template_revision,
        timestamp_delta,
        values,
        quality,
    })
}

fn decode_values(template: &Template, cursor: &mut ByteCursor<'_>) -> Result<Vec<ReadingValue>> {
    let mut values = Vec::with_capacity(template.fields.len());
    for field in &template.fields {
        if field.nullable && cursor.remaining() > 0 {
            let marker = cursor.peek_u8()?;
            if marker == 0xff {
                cursor.read_u8()?;
                values.push(ReadingValue::Null { kind: field.kind });
                continue;
            }
        }
        values.push(decode_value(field, cursor)?);
    }
    Ok(values)
}

fn decode_value(field: &FieldSpec, cursor: &mut ByteCursor<'_>) -> Result<ReadingValue> {
    match field.codec {
        FieldCodec::Bitmap => {
            let bits = read_unsigned(cursor, field.width)?;
            Ok(ReadingValue::Bitmap {
                kind: field.kind,
                bits,
            })
        }
        FieldCodec::Unsigned | FieldCodec::FixedPoint | FieldCodec::PackedBcd => {
            let value = read_unsigned(cursor, field.width)? as i64;
            Ok(ReadingValue::Integer {
                kind: field.kind,
                value: normalize_packed_bcd(value, field.codec),
                scale: field.scale,
            })
        }
        FieldCodec::Signed => {
            let raw = read_unsigned(cursor, field.width)?;
            let shift = 64usize.saturating_sub(field.width.saturating_mul(8));
            let value = ((raw << shift) as i64) >> shift;
            Ok(ReadingValue::Integer {
                kind: field.kind,
                value,
                scale: field.scale,
            })
        }
        FieldCodec::ZigZag => {
            let raw = read_unsigned(cursor, field.width)?;
            let value = ((raw >> 1) as i64) ^ (-((raw & 1) as i64));
            Ok(ReadingValue::Integer {
                kind: field.kind,
                value,
                scale: field.scale,
            })
        }
    }
}

fn read_unsigned(cursor: &mut ByteCursor<'_>, width: usize) -> Result<u64> {
    if width == 0 || width > 8 {
        return Err(GridError::MeasurementMalformed("unsupported field width"));
    }
    let bytes = cursor.take(width)?;
    let mut value = 0u64;
    for (shift, byte) in bytes.iter().enumerate() {
        value |= (*byte as u64) << (shift * 8);
    }
    Ok(value)
}

fn normalize_packed_bcd(value: i64, codec: FieldCodec) -> i64 {
    if codec != FieldCodec::PackedBcd {
        return value;
    }
    let mut raw = value as u64;
    let mut multiplier = 1i64;
    let mut out = 0i64;
    while raw != 0 {
        let digit = (raw & 0x0f) as i64;
        if digit > 9 {
            return value;
        }
        out += digit * multiplier;
        multiplier *= 10;
        raw >>= 4;
    }
    out
}
