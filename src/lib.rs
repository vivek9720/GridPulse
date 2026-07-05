//! GridPulse decodes low-bandwidth smart-grid mesh telemetry.
//!
//! The crate keeps dependencies at zero so the fuzzing bundle can build in a
//! network-disabled ClusterFuzzLite image. The public entry points are small,
//! but the decoder behind them is staged: frames, fragments, sessions,
//! dictionaries, measurements, routes, scripts, and replay analysis.

pub mod analytics;
pub mod catalog;
pub mod crc;
pub mod cursor;
pub mod error;
pub mod frame;
pub mod fragment;
pub mod measurements;
pub mod model;
pub mod replay;
pub mod route;
pub mod scripts;
pub mod session;
pub mod template;
pub mod tlv;

pub use crate::error::{GridError, Result};
pub use crate::model::{DecodedBatch, ReplayReport, ScriptProgram};
pub use crate::session::Decoder;

/// Parse a stream of GridPulse mesh frames using a fresh decoder.
pub fn parse(data: &[u8]) -> Result<DecodedBatch> {
    let mut decoder = Decoder::new();
    decoder.ingest_stream(data)
}

/// Parse a stream with replay-oriented staging across segments and sessions.
pub fn replay(data: &[u8]) -> Result<ReplayReport> {
    let mut runner = replay::ReplayRunner::new();
    runner.run(data)
}

/// Parse only the embedded field-crew script language.
pub fn parse_script(data: &[u8]) -> Result<ScriptProgram> {
    scripts::parse_script(data)
}

/// Parse one or more frames, dropping any semantic processing.
pub fn parse_frames(data: &[u8]) -> Result<Vec<frame::Frame>> {
    frame::parse_frames(data)
}
