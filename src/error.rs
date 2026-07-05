use core::fmt;

pub type Result<T> = core::result::Result<T, GridError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GridError {
    Truncated {
        needed: usize,
        remaining: usize,
        context: &'static str,
    },
    InvalidMagic {
        offset: usize,
    },
    InvalidVersion(u8),
    InvalidLength {
        declared: usize,
        available: usize,
        context: &'static str,
    },
    CrcMismatch {
        expected: u16,
        actual: u16,
    },
    UnsupportedFrameKind(u8),
    MissingSession,
    SessionMismatch {
        expected: u16,
        actual: u16,
    },
    FragmentConflict,
    FragmentTooLarge,
    TemplateMissing {
        id: u16,
        revision: u8,
    },
    TemplateMalformed(&'static str),
    MeasurementMalformed(&'static str),
    RouteMalformed(&'static str),
    ScriptMalformed(&'static str),
    UnknownOpcode(u8),
    TlvMalformed(&'static str),
}

impl fmt::Display for GridError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GridError::Truncated {
                needed,
                remaining,
                context,
            } => write!(
                f,
                "truncated input in {context}: needed {needed}, remaining {remaining}"
            ),
            GridError::InvalidMagic { offset } => write!(f, "invalid frame magic at offset {offset}"),
            GridError::InvalidVersion(version) => write!(f, "unsupported gridpulse version {version}"),
            GridError::InvalidLength {
                declared,
                available,
                context,
            } => write!(
                f,
                "invalid length in {context}: declared {declared}, available {available}"
            ),
            GridError::CrcMismatch { expected, actual } => {
                write!(f, "crc mismatch: expected {expected:#06x}, actual {actual:#06x}")
            }
            GridError::UnsupportedFrameKind(kind) => write!(f, "unsupported frame kind {kind:#04x}"),
            GridError::MissingSession => write!(f, "frame requires an open session"),
            GridError::SessionMismatch { expected, actual } => {
                write!(f, "session mismatch: expected {expected:#06x}, actual {actual:#06x}")
            }
            GridError::FragmentConflict => write!(f, "conflicting fragment for sequence"),
            GridError::FragmentTooLarge => write!(f, "fragment reassembly limit exceeded"),
            GridError::TemplateMissing { id, revision } => {
                write!(f, "template {id:#06x}/{revision} is missing")
            }
            GridError::TemplateMalformed(message) => write!(f, "malformed template: {message}"),
            GridError::MeasurementMalformed(message) => write!(f, "malformed measurement: {message}"),
            GridError::RouteMalformed(message) => write!(f, "malformed route message: {message}"),
            GridError::ScriptMalformed(message) => write!(f, "malformed script: {message}"),
            GridError::UnknownOpcode(opcode) => write!(f, "unknown script opcode {opcode:#04x}"),
            GridError::TlvMalformed(message) => write!(f, "malformed tlv: {message}"),
        }
    }
}

impl std::error::Error for GridError {}
