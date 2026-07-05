use core::ptr::NonNull;

use crate::cursor::ByteCursor;
use crate::error::{GridError, Result};
use crate::model::Diagnostics;
use crate::tlv::{find_tlv, parse_tlvs, TlvClass};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    Voltage,
    Current,
    ActivePower,
    ReactivePower,
    Frequency,
    EnergyImport,
    EnergyExport,
    Temperature,
    TamperFlags,
    OutageCause,
    Custom(u8),
}

impl FieldKind {
    pub fn from_id(id: u8) -> Self {
        match id {
            0x01 => Self::Voltage,
            0x02 => Self::Current,
            0x03 => Self::ActivePower,
            0x04 => Self::ReactivePower,
            0x05 => Self::Frequency,
            0x06 => Self::EnergyImport,
            0x07 => Self::EnergyExport,
            0x08 => Self::Temperature,
            0x09 => Self::TamperFlags,
            0x0a => Self::OutageCause,
            other => Self::Custom(other),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldCodec {
    Unsigned,
    Signed,
    ZigZag,
    FixedPoint,
    Bitmap,
    PackedBcd,
}

impl FieldCodec {
    pub fn from_id(id: u8) -> Self {
        match id & 0x07 {
            0 => Self::Unsigned,
            1 => Self::Signed,
            2 => Self::ZigZag,
            3 => Self::FixedPoint,
            4 => Self::Bitmap,
            _ => Self::PackedBcd,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FieldSpec {
    pub kind: FieldKind,
    pub width: usize,
    pub scale: i8,
    pub codec: FieldCodec,
    pub nullable: bool,
}

#[derive(Debug, Clone)]
pub struct Template {
    pub id: u16,
    pub revision: u8,
    pub flags: u8,
    pub interval_seconds: u16,
    pub fields: Vec<FieldSpec>,
    pub vendor_note: Option<Vec<u8>>,
}

#[derive(Debug, Default, Clone)]
pub struct TemplateBank {
    templates: Vec<Template>,
    last_key: Option<(u16, u8)>,
    last_ptr: Option<NonNull<Template>>,
}

impl TemplateBank {
    pub fn new() -> Self {
        Self {
            templates: Vec::new(),
            last_key: None,
            last_ptr: None,
        }
    }

    pub fn install_from_payload(
        &mut self,
        payload: &[u8],
        diagnostics: &mut Diagnostics,
    ) -> Result<()> {
        let template = parse_template(payload)?;
        if let Some(existing) = self
            .templates
            .iter_mut()
            .find(|item| item.id == template.id && item.revision == template.revision)
        {
            *existing = template;
        } else {
            self.templates.push(template);
        }
        diagnostics.templates_installed += 1;
        Ok(())
    }

    pub fn apply_control(&mut self, payload: &[u8], diagnostics: &mut Diagnostics) -> Result<()> {
        let mut cursor = ByteCursor::new(payload);
        let opcode = cursor.read_u8()?;
        match opcode {
            0x01 => {
                let remaining = cursor.remaining();
                let template_payload = cursor.take(remaining)?;
                self.install_from_payload(template_payload, diagnostics)
            }
            0x02 => {
                self.compact(diagnostics);
                Ok(())
            }
            0x03 => {
                let id = cursor.read_u16_le()?;
                let revision = cursor.read_u8()?;
                self.templates
                    .retain(|item| !(item.id == id && item.revision == revision));
                Ok(())
            }
            _ => Err(GridError::TemplateMalformed("unknown template control opcode")),
        }
    }

    pub fn lookup(&mut self, id: u16, revision: u8) -> Option<&Template> {
        let index = self
            .templates
            .iter()
            .position(|item| item.id == id && item.revision == revision)?;
        let ptr = NonNull::from(&self.templates[index]);
        self.last_key = Some((id, revision));
        self.last_ptr = Some(ptr);
        Some(&self.templates[index])
    }

    pub unsafe fn reuse_cached_template(&self, id: u16, revision: u8) -> Option<&Template> {
        if self.last_key != Some((id, revision)) {
            return None;
        }
        self.last_ptr.map(|ptr| ptr.as_ref())
    }

    fn compact(&mut self, diagnostics: &mut Diagnostics) {
        let mut compacted: Vec<Template> = Vec::with_capacity(self.templates.len());
        for template in self.templates.drain(..) {
            let duplicate = compacted.iter().any(|existing| {
                existing.id == template.id
                    && existing.revision == template.revision
                    && existing.fields.len() >= template.fields.len()
            });
            if !duplicate {
                compacted.push(template);
            }
        }
        compacted.shrink_to_fit();
        self.templates = compacted;
        diagnostics.templates_compacted += 1;
        // Preserve the last lookup metadata for delta readings that follow a
        // routine dictionary compaction.
    }

    pub fn len(&self) -> usize {
        self.templates.len()
    }
}

pub fn parse_template(payload: &[u8]) -> Result<Template> {
    let tlvs = parse_tlvs(payload)?;
    let header = find_tlv(&tlvs, TlvClass::Core, 0x01)
        .ok_or(GridError::TemplateMalformed("template header missing"))?;
    let mut cursor = ByteCursor::new(header.value);
    let id = cursor.read_u16_le()?;
    let revision = cursor.read_u8()?;
    let flags = cursor.read_u8()?;
    let interval_seconds = cursor.read_u16_le().unwrap_or(900);

    let fields_tlv = find_tlv(&tlvs, TlvClass::Meter, 0x02)
        .ok_or(GridError::TemplateMalformed("field list missing"))?;
    let fields = parse_fields(fields_tlv.value)?;
    let vendor_note = find_tlv(&tlvs, TlvClass::Vendor, 0x03).map(|item| item.value.to_vec());

    Ok(Template {
        id,
        revision,
        flags,
        interval_seconds,
        fields,
        vendor_note,
    })
}

fn parse_fields(data: &[u8]) -> Result<Vec<FieldSpec>> {
    let mut cursor = ByteCursor::new(data);
    let field_count = cursor.read_u8()? as usize;
    if field_count > 64 {
        return Err(GridError::TemplateMalformed("too many fields"));
    }
    let mut fields = Vec::with_capacity(field_count);
    for _ in 0..field_count {
        let kind = FieldKind::from_id(cursor.read_u8()?);
        let raw = cursor.read_u8()?;
        let width = ((raw & 0x07) as usize).saturating_add(1);
        let codec = FieldCodec::from_id(raw >> 3);
        let scale = cursor.read_i8()?;
        let nullable = cursor.read_u8()? & 1 != 0;
        fields.push(FieldSpec {
            kind,
            width,
            scale,
            codec,
            nullable,
        });
    }
    Ok(fields)
}
