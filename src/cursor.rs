use crate::error::{GridError, Result};

#[derive(Clone, Copy)]
pub struct ByteCursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> ByteCursor<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    pub fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    pub fn seek(&mut self, pos: usize) -> Result<()> {
        if pos > self.data.len() {
            return Err(GridError::InvalidLength {
                declared: pos,
                available: self.data.len(),
                context: "cursor seek",
            });
        }
        self.pos = pos;
        Ok(())
    }

    pub fn peek_u8(&self) -> Result<u8> {
        self.data
            .get(self.pos)
            .copied()
            .ok_or(GridError::Truncated {
                needed: 1,
                remaining: self.remaining(),
                context: "peek_u8",
            })
    }

    pub fn read_u8(&mut self) -> Result<u8> {
        let value = self.peek_u8()?;
        self.pos += 1;
        Ok(value)
    }

    pub fn read_i8(&mut self) -> Result<i8> {
        Ok(self.read_u8()? as i8)
    }

    pub fn read_u16_le(&mut self) -> Result<u16> {
        let bytes = self.take(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    pub fn read_i16_le(&mut self) -> Result<i16> {
        Ok(self.read_u16_le()? as i16)
    }

    pub fn read_u32_le(&mut self) -> Result<u32> {
        let bytes = self.take(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn read_i32_le(&mut self) -> Result<i32> {
        Ok(self.read_u32_le()? as i32)
    }

    pub fn read_u64_le(&mut self) -> Result<u64> {
        let bytes = self.take(8)?;
        Ok(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    pub fn read_var_u32(&mut self) -> Result<u32> {
        let mut shift = 0;
        let mut value = 0u32;
        for _ in 0..5 {
            let byte = self.read_u8()?;
            value |= ((byte & 0x7f) as u32) << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
            shift += 7;
        }
        Err(GridError::InvalidLength {
            declared: 5,
            available: self.remaining(),
            context: "var_u32",
        })
    }

    pub fn read_svar_i32(&mut self) -> Result<i32> {
        let raw = self.read_var_u32()?;
        Ok(((raw >> 1) as i32) ^ (-((raw & 1) as i32)))
    }

    pub fn take(&mut self, len: usize) -> Result<&'a [u8]> {
        if len > self.remaining() {
            return Err(GridError::Truncated {
                needed: len,
                remaining: self.remaining(),
                context: "cursor take",
            });
        }
        let start = self.pos;
        self.pos += len;
        Ok(&self.data[start..start + len])
    }

    pub fn take_until(&mut self, marker: u8, limit: usize) -> Result<&'a [u8]> {
        let start = self.pos;
        let max = self.remaining().min(limit);
        for offset in 0..max {
            if self.data[start + offset] == marker {
                self.pos = start + offset + 1;
                return Ok(&self.data[start..start + offset]);
            }
        }
        Err(GridError::InvalidLength {
            declared: limit,
            available: self.remaining(),
            context: "cursor take_until",
        })
    }
}
