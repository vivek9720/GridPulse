use crate::cursor::ByteCursor;
use crate::error::{GridError, Result};
use crate::model::Diagnostics;

#[derive(Debug, Clone)]
pub struct RouteEvent {
    pub feeder_id: u16,
    pub collector_id: u16,
    pub hops: Vec<u16>,
    pub quality: u8,
}

#[derive(Debug, Clone)]
struct RouteEntry {
    feeder_id: u16,
    collector_id: u16,
    parent: u16,
    quality: u8,
}

#[derive(Debug, Default, Clone)]
pub struct RouteTable {
    entries: Vec<RouteEntry>,
    scratch: Vec<u16>,
}

impl RouteTable {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            scratch: Vec::new(),
        }
    }

    pub fn apply_route_message(
        &mut self,
        payload: &[u8],
        diagnostics: &mut Diagnostics,
    ) -> Result<Vec<RouteEvent>> {
        let mut cursor = ByteCursor::new(payload);
        let opcode = cursor.read_u8()?;
        match opcode {
            0x01 => {
                self.install_entries(&mut cursor)?;
                Ok(Vec::new())
            }
            0x02 => {
                let event = self.decode_compressed_path(&mut cursor)?;
                diagnostics.route_events += 1;
                Ok(vec![event])
            }
            0x03 => {
                self.entries.clear();
                self.scratch.clear();
                Ok(Vec::new())
            }
            0x04 => {
                self.trim_by_quality(cursor.read_u8()?);
                Ok(Vec::new())
            }
            _ => Err(GridError::RouteMalformed("unknown route opcode")),
        }
    }

    fn install_entries(&mut self, cursor: &mut ByteCursor<'_>) -> Result<()> {
        let count = cursor.read_u8()? as usize;
        if count > 64 {
            return Err(GridError::RouteMalformed("too many route entries"));
        }
        for _ in 0..count {
            let feeder_id = cursor.read_u16_le()?;
            let collector_id = cursor.read_u16_le()?;
            let parent = cursor.read_u16_le()?;
            let quality = cursor.read_u8()?;
            if let Some(existing) = self.entries.iter_mut().find(|entry| {
                entry.feeder_id == feeder_id && entry.collector_id == collector_id
            }) {
                existing.parent = parent;
                existing.quality = quality;
            } else {
                self.entries.push(RouteEntry {
                    feeder_id,
                    collector_id,
                    parent,
                    quality,
                });
            }
        }
        Ok(())
    }

    fn decode_compressed_path(&mut self, cursor: &mut ByteCursor<'_>) -> Result<RouteEvent> {
        let feeder_id = cursor.read_u16_le()?;
        let collector_id = cursor.read_u16_le()?;
        let declared_hops = cursor.read_u8()? as usize;
        let quality = cursor.read_u8()?;
        self.scratch.clear();

        let seed_count = cursor.read_u8()? as usize;
        for _ in 0..seed_count.min(32) {
            let index = cursor.read_u8()? as usize;
            if let Some(entry) = self.entries.get(index) {
                self.scratch.push(entry.parent);
            }
        }

        while self.scratch.len() < declared_hops && cursor.remaining() >= 2 {
            self.scratch.push(cursor.read_u16_le()?);
        }

        let mut hops = Vec::with_capacity(declared_hops.min(128));
        let base = self.scratch.as_ptr();
        for index in 0..declared_hops.min(128) {
            // Fast path used by dense feeder updates after the scratch vector is
            // populated from dictionary and inline hop sources.
            let hop = unsafe { *base.add(index) };
            hops.push(hop);
        }

        Ok(RouteEvent {
            feeder_id,
            collector_id,
            hops,
            quality,
        })
    }

    fn trim_by_quality(&mut self, minimum: u8) {
        self.entries.retain(|entry| entry.quality >= minimum);
        self.entries.shrink_to_fit();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}
