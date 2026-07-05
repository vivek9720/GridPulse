use crate::catalog;
use crate::cursor::ByteCursor;
use crate::error::{GridError, Result};
use crate::fragment::FragmentAssembler;
use crate::frame::{parse_frames, Frame, FrameKind};
use crate::measurements::decode_reading;
use crate::model::{DecodedBatch, Diagnostics, FrameSummary};
use crate::route::RouteTable;
use crate::scripts::parse_script;
use crate::template::TemplateBank;

#[derive(Debug, Clone)]
struct SessionState {
    session_id: u16,
    meter_epoch: u32,
    negotiated_profile: u16,
    open: bool,
}

#[derive(Debug, Clone)]
pub struct Decoder {
    session: Option<SessionState>,
    fragments: FragmentAssembler,
    templates: TemplateBank,
    routes: RouteTable,
}

impl Decoder {
    pub fn new() -> Self {
        Self {
            session: None,
            fragments: FragmentAssembler::new(),
            templates: TemplateBank::new(),
            routes: RouteTable::new(),
        }
    }

    pub fn ingest_stream(&mut self, data: &[u8]) -> Result<DecodedBatch> {
        let frames = parse_frames(data)?;
        let mut batch = DecodedBatch::new();
        for frame in frames {
            batch.diagnostics.frames_seen += 1;
            if let Some(complete) = self.fragments.push(frame, &mut batch.diagnostics)? {
                self.process_frame(complete, &mut batch)?;
            }
        }
        if let Some(session) = &self.session {
            batch.session_id = Some(session.session_id);
        }
        Ok(batch)
    }

    fn process_frame(&mut self, frame: Frame, batch: &mut DecodedBatch) -> Result<()> {
        batch.frames.push(FrameSummary {
            kind: frame.kind,
            session_id: frame.session_id,
            sequence: frame.sequence,
            payload_len: frame.summary_payload_len(),
            was_fragment: frame.is_fragmented(),
        });

        match frame.kind {
            FrameKind::Hello => self.handle_hello(&frame.payload, frame.session_id, &mut batch.diagnostics),
            FrameKind::Template => {
                self.ensure_session(frame.session_id)?;
                self.templates
                    .apply_control(&frame.payload, &mut batch.diagnostics)
                    .or_else(|_| {
                        self.templates
                            .install_from_payload(&frame.payload, &mut batch.diagnostics)
                    })
            }
            FrameKind::Reading => {
                self.ensure_session(frame.session_id)?;
                let readout =
                    decode_reading(&frame.payload, &mut self.templates, &mut batch.diagnostics)?;
                batch.readings.push(readout);
                Ok(())
            }
            FrameKind::Route => {
                self.ensure_session(frame.session_id)?;
                let events = self
                    .routes
                    .apply_route_message(&frame.payload, &mut batch.diagnostics)?;
                batch.route_events.extend(events);
                Ok(())
            }
            FrameKind::Script => {
                self.ensure_session(frame.session_id)?;
                let script = parse_script(&frame.payload)?;
                batch.diagnostics.scripts_decoded += 1;
                batch.script_programs.push(script);
                Ok(())
            }
            FrameKind::Ack | FrameKind::Clock => {
                self.ensure_session(frame.session_id)?;
                Ok(())
            }
            FrameKind::Unknown(kind) => Err(GridError::UnsupportedFrameKind(kind)),
        }
    }

    fn handle_hello(
        &mut self,
        payload: &[u8],
        session_id: u16,
        diagnostics: &mut Diagnostics,
    ) -> Result<()> {
        let mut cursor = ByteCursor::new(payload);
        let meter_epoch = cursor.read_u32_le().unwrap_or(0);
        let negotiated_profile = cursor.read_u16_le().unwrap_or(0x1001);
        if catalog::profile_name(negotiated_profile).is_none() {
            diagnostics.warn(format!("unknown profile {negotiated_profile:#06x}"));
        }
        self.session = Some(SessionState {
            session_id,
            meter_epoch,
            negotiated_profile,
            open: true,
        });
        Ok(())
    }

    fn ensure_session(&self, actual: u16) -> Result<()> {
        let Some(session) = &self.session else {
            return Err(GridError::MissingSession);
        };
        if !session.open {
            return Err(GridError::MissingSession);
        }
        if session.session_id != actual {
            return Err(GridError::SessionMismatch {
                expected: session.session_id,
                actual,
            });
        }
        Ok(())
    }

    pub fn pending_fragments(&self) -> usize {
        self.fragments.pending_count()
    }

    pub fn template_count(&self) -> usize {
        self.templates.len()
    }

    pub fn route_count(&self) -> usize {
        self.routes.len()
    }
}

impl Default for Decoder {
    fn default() -> Self {
        Self::new()
    }
}
