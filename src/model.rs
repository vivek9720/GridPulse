use crate::frame::FrameKind;
use crate::measurements::Readout;
use crate::route::RouteEvent;
use crate::scripts::ScriptInstruction;

#[derive(Debug, Clone, Default)]
pub struct Diagnostics {
    pub frames_seen: usize,
    pub fragments_completed: usize,
    pub templates_installed: usize,
    pub templates_compacted: usize,
    pub readings_decoded: usize,
    pub route_events: usize,
    pub scripts_decoded: usize,
    pub warnings: Vec<String>,
}

impl Diagnostics {
    pub fn warn(&mut self, message: impl Into<String>) {
        if self.warnings.len() < 64 {
            self.warnings.push(message.into());
        }
    }
}

#[derive(Debug, Clone)]
pub struct DecodedBatch {
    pub session_id: Option<u16>,
    pub frames: Vec<FrameSummary>,
    pub readings: Vec<Readout>,
    pub route_events: Vec<RouteEvent>,
    pub script_programs: Vec<ScriptProgram>,
    pub diagnostics: Diagnostics,
}

impl DecodedBatch {
    pub fn new() -> Self {
        Self {
            session_id: None,
            frames: Vec::new(),
            readings: Vec::new(),
            route_events: Vec::new(),
            script_programs: Vec::new(),
            diagnostics: Diagnostics::default(),
        }
    }
}

impl Default for DecodedBatch {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct FrameSummary {
    pub kind: FrameKind,
    pub session_id: u16,
    pub sequence: u16,
    pub payload_len: usize,
    pub was_fragment: bool,
}

#[derive(Debug, Clone)]
pub struct ScriptProgram {
    pub version: u8,
    pub instructions: Vec<ScriptInstruction>,
    pub max_depth: u8,
    pub declared_len: usize,
}

#[derive(Debug, Clone)]
pub struct ReplayReport {
    pub batches: Vec<DecodedBatch>,
    pub fingerprint: u32,
    pub accepted_segments: usize,
    pub rejected_segments: usize,
}

impl ReplayReport {
    pub fn new() -> Self {
        Self {
            batches: Vec::new(),
            fingerprint: 0,
            accepted_segments: 0,
            rejected_segments: 0,
        }
    }
}

impl Default for ReplayReport {
    fn default() -> Self {
        Self::new()
    }
}
