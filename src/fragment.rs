use crate::error::{GridError, Result};
use crate::frame::Frame;
use crate::model::Diagnostics;

const MAX_REASSEMBLED: usize = 8192;

#[derive(Debug, Clone)]
struct FragmentSlot {
    session_id: u16,
    sequence: u16,
    kind_byte: u8,
    flags: u8,
    pieces: Vec<Option<Vec<u8>>>,
    received: usize,
}

#[derive(Debug, Default, Clone)]
pub struct FragmentAssembler {
    slots: Vec<FragmentSlot>,
}

impl FragmentAssembler {
    pub fn new() -> Self {
        Self { slots: Vec::new() }
    }

    pub fn push(&mut self, frame: Frame, diagnostics: &mut Diagnostics) -> Result<Option<Frame>> {
        if !frame.is_fragmented() {
            return Ok(Some(frame));
        }

        if frame.fragment_count as usize > 32 {
            return Err(GridError::FragmentTooLarge);
        }

        let total_len = self
            .slots
            .iter()
            .find(|slot| slot.session_id == frame.session_id && slot.sequence == frame.sequence)
            .and_then(|slot| {
                let known: usize = slot
                    .pieces
                    .iter()
                    .filter_map(|piece| piece.as_ref().map(Vec::len))
                    .sum();
                known.checked_add(frame.payload.len())
            })
            .unwrap_or(frame.payload.len());
        if total_len > MAX_REASSEMBLED {
            return Err(GridError::FragmentTooLarge);
        }

        let index = frame.fragment_index as usize;
        let count = frame.fragment_count as usize;
        let slot_index = self.find_or_create_slot(&frame, count);
        let slot = &mut self.slots[slot_index];
        if index >= slot.pieces.len() {
            return Err(GridError::FragmentConflict);
        }
        if slot.pieces[index].is_some() {
            return Err(GridError::FragmentConflict);
        }
        slot.pieces[index] = Some(frame.payload.clone());
        slot.received += 1;
        if slot.received != slot.pieces.len() {
            return Ok(None);
        }

        let mut payload = Vec::new();
        for piece in &mut slot.pieces {
            if let Some(bytes) = piece.take() {
                payload.extend(bytes);
            }
        }
        diagnostics.fragments_completed += 1;
        let mut complete = frame.with_payload(payload);
        complete.flags = slot.flags;
        complete.kind = crate::frame::FrameKind::from_byte(slot.kind_byte);
        self.slots.swap_remove(slot_index);
        Ok(Some(complete))
    }

    fn find_or_create_slot(&mut self, frame: &Frame, count: usize) -> usize {
        if let Some(index) = self
            .slots
            .iter()
            .position(|slot| slot.session_id == frame.session_id && slot.sequence == frame.sequence)
        {
            return index;
        }
        self.slots.push(FragmentSlot {
            session_id: frame.session_id,
            sequence: frame.sequence,
            kind_byte: frame.kind.as_byte(),
            flags: frame.flags & !crate::frame::FLAG_FRAGMENT,
            pieces: vec![None; count],
            received: 0,
        });
        self.slots.len() - 1
    }

    pub fn pending_count(&self) -> usize {
        self.slots.len()
    }
}
