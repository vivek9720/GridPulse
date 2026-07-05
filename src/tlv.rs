use crate::cursor::ByteCursor;
use crate::error::{GridError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlvClass {
    Core,
    Meter,
    Routing,
    Vendor,
}

#[derive(Debug, Clone)]
pub struct Tlv<'a> {
    pub class: TlvClass,
    pub tag: u8,
    pub value: &'a [u8],
}

impl<'a> Tlv<'a> {
    pub fn tag_id(&self) -> u8 {
        self.tag & 0x3f
    }
}

pub fn parse_tlvs(data: &[u8]) -> Result<Vec<Tlv<'_>>> {
    let mut cursor = ByteCursor::new(data);
    let mut items = Vec::new();
    while !cursor.is_empty() {
        let raw_tag = cursor.read_u8()?;
        let class = match raw_tag >> 6 {
            0 => TlvClass::Core,
            1 => TlvClass::Meter,
            2 => TlvClass::Routing,
            _ => TlvClass::Vendor,
        };
        let len = if raw_tag & 0x20 != 0 {
            cursor.read_var_u32()? as usize
        } else {
            cursor.read_u8()? as usize
        };
        if len > cursor.remaining() {
            return Err(GridError::TlvMalformed("declared length exceeds remaining bytes"));
        }
        let value = cursor.take(len)?;
        items.push(Tlv {
            class,
            tag: raw_tag,
            value,
        });
        if items.len() > 256 {
            return Err(GridError::TlvMalformed("too many tlv records"));
        }
    }
    Ok(items)
}

pub fn find_tlv<'a>(tlvs: &'a [Tlv<'a>], class: TlvClass, tag_id: u8) -> Option<&'a Tlv<'a>> {
    tlvs.iter()
        .find(|item| item.class == class && item.tag_id() == tag_id)
}
