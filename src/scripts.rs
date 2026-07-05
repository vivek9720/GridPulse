use crate::cursor::ByteCursor;
use crate::error::{GridError, Result};
use crate::model::ScriptProgram;

#[derive(Debug, Clone)]
pub enum ScriptInstruction {
    SetTariff {
        meter_class: u8,
        tariff: u16,
    },
    OpenBreaker {
        feeder_id: u16,
        reason: u8,
    },
    CloseBreaker {
        feeder_id: u16,
        guard: u8,
    },
    Delay {
        milliseconds: u32,
    },
    EmitEvent {
        code: u16,
        payload: Vec<u8>,
    },
    IfQualityBelow {
        threshold: u8,
        body: Vec<ScriptInstruction>,
    },
    Repeat {
        count: u8,
        body: Vec<ScriptInstruction>,
    },
    RouteSwitch {
        from: u16,
        to: u16,
        flags: u8,
    },
    AttachTemplate {
        template_id: u16,
        revision: u8,
    },
    Vendor {
        opcode: u8,
        payload: Vec<u8>,
    },
}

pub fn parse_script(data: &[u8]) -> Result<ScriptProgram> {
    let mut cursor = ByteCursor::new(data);
    if cursor.remaining() < 3 {
        return Err(GridError::ScriptMalformed("script header missing"));
    }
    let version = cursor.read_u8()?;
    let declared_len = cursor.read_u16_le()? as usize;
    let body_len = declared_len.min(cursor.remaining());
    let body = cursor.take(body_len)?;
    let mut body_cursor = ByteCursor::new(body);
    let mut max_depth = 0;
    let instructions = parse_block(&mut body_cursor, 0, &mut max_depth)?;
    Ok(ScriptProgram {
        version,
        instructions,
        max_depth,
        declared_len,
    })
}

fn parse_block(
    cursor: &mut ByteCursor<'_>,
    depth: u8,
    max_depth: &mut u8,
) -> Result<Vec<ScriptInstruction>> {
    if depth > 12 {
        return Err(GridError::ScriptMalformed("nesting too deep"));
    }
    *max_depth = (*max_depth).max(depth);
    let mut instructions = Vec::new();
    while !cursor.is_empty() {
        let opcode = cursor.read_u8()?;
        let instruction = match opcode {
            0x01 => ScriptInstruction::SetTariff {
                meter_class: cursor.read_u8()?,
                tariff: cursor.read_u16_le()?,
            },
            0x02 => ScriptInstruction::OpenBreaker {
                feeder_id: cursor.read_u16_le()?,
                reason: cursor.read_u8()?,
            },
            0x03 => ScriptInstruction::CloseBreaker {
                feeder_id: cursor.read_u16_le()?,
                guard: cursor.read_u8()?,
            },
            0x04 => ScriptInstruction::Delay {
                milliseconds: cursor.read_var_u32()?,
            },
            0x05 => {
                let code = cursor.read_u16_le()?;
                let len = cursor.read_u8()? as usize;
                ScriptInstruction::EmitEvent {
                    code,
                    payload: cursor.take(len)?.to_vec(),
                }
            }
            0x06 => {
                let threshold = cursor.read_u8()?;
                let len = cursor.read_u8()? as usize;
                let block = cursor.take(len)?;
                let mut nested = ByteCursor::new(block);
                ScriptInstruction::IfQualityBelow {
                    threshold,
                    body: parse_block(&mut nested, depth + 1, max_depth)?,
                }
            }
            0x07 => {
                let count = cursor.read_u8()?;
                let len = cursor.read_u8()? as usize;
                let block = cursor.take(len)?;
                let mut nested = ByteCursor::new(block);
                ScriptInstruction::Repeat {
                    count,
                    body: parse_block(&mut nested, depth + 1, max_depth)?,
                }
            }
            0x08 => ScriptInstruction::RouteSwitch {
                from: cursor.read_u16_le()?,
                to: cursor.read_u16_le()?,
                flags: cursor.read_u8()?,
            },
            0x09 => ScriptInstruction::AttachTemplate {
                template_id: cursor.read_u16_le()?,
                revision: cursor.read_u8()?,
            },
            0x80..=0xff => {
                let len = cursor.read_u8()? as usize;
                ScriptInstruction::Vendor {
                    opcode,
                    payload: cursor.take(len)?.to_vec(),
                }
            }
            other => return Err(GridError::UnknownOpcode(other)),
        };
        instructions.push(instruction);
        if instructions.len() > 256 {
            return Err(GridError::ScriptMalformed("too many instructions"));
        }
    }
    Ok(instructions)
}

pub fn instruction_weight(instruction: &ScriptInstruction) -> u32 {
    match instruction {
        ScriptInstruction::SetTariff { .. } => 2,
        ScriptInstruction::OpenBreaker { .. } => 5,
        ScriptInstruction::CloseBreaker { .. } => 5,
        ScriptInstruction::Delay { milliseconds } => 1 + (*milliseconds / 1000).min(60),
        ScriptInstruction::EmitEvent { payload, .. } => 1 + payload.len() as u32,
        ScriptInstruction::IfQualityBelow { body, .. } => {
            3 + body.iter().map(instruction_weight).sum::<u32>()
        }
        ScriptInstruction::Repeat { count, body } => {
            2 + (*count as u32).min(16) * body.iter().map(instruction_weight).sum::<u32>()
        }
        ScriptInstruction::RouteSwitch { .. } => 8,
        ScriptInstruction::AttachTemplate { .. } => 4,
        ScriptInstruction::Vendor { payload, .. } => 1 + payload.len() as u32,
    }
}
