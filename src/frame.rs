use crate::crc::crc16_gridpulse;
use crate::cursor::ByteCursor;
use crate::error::{GridError, Result};

pub const MAGIC: [u8; 2] = [b'G', b'P'];
pub const VERSION: u8 = 1;
pub const FLAG_CRC_PRESENT: u8 = 0x01;
pub const FLAG_FRAGMENT: u8 = 0x02;
pub const FLAG_CONTROL: u8 = 0x04;
pub const FLAG_PRIORITY: u8 = 0x08;
pub const MAX_PAYLOAD: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameKind {
    Hello,
    Template,
    Reading,
    Route,
    Script,
    Ack,
    Clock,
    Unknown(u8),
}

impl FrameKind {
    pub fn from_byte(byte: u8) -> Self {
        match byte {
            0x01 => Self::Hello,
            0x02 => Self::Template,
            0x03 => Self::Reading,
            0x04 => Self::Route,
            0x05 => Self::Script,
            0x06 => Self::Ack,
            0x07 => Self::Clock,
            other => Self::Unknown(other),
        }
    }

    pub fn as_byte(self) -> u8 {
        match self {
            Self::Hello => 0x01,
            Self::Template => 0x02,
            Self::Reading => 0x03,
            Self::Route => 0x04,
            Self::Script => 0x05,
            Self::Ack => 0x06,
            Self::Clock => 0x07,
            Self::Unknown(other) => other,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Frame {
    pub flags: u8,
    pub session_id: u16,
    pub sequence: u16,
    pub fragment_index: u8,
    pub fragment_count: u8,
    pub kind: FrameKind,
    pub payload: Vec<u8>,
    pub header_crc: Option<u16>,
    pub offset: usize,
}

impl Frame {
    pub fn is_fragmented(&self) -> bool {
        self.flags & FLAG_FRAGMENT != 0 || self.fragment_count > 1
    }

    pub fn with_payload(mut self, payload: Vec<u8>) -> Self {
        self.payload = payload;
        self.flags &= !FLAG_FRAGMENT;
        self.fragment_index = 0;
        self.fragment_count = 1;
        self
    }

    pub fn summary_payload_len(&self) -> usize {
        self.payload.len()
    }
}

pub fn parse_frames(data: &[u8]) -> Result<Vec<Frame>> {
    let mut cursor = ByteCursor::new(data);
    let mut frames = Vec::new();
    while cursor.remaining() >= 4 {
        match parse_one_frame(&mut cursor) {
            Ok(frame) => frames.push(frame),
            Err(GridError::InvalidMagic { .. }) => {
                cursor.seek(cursor.position().saturating_add(1))?;
            }
            Err(GridError::Truncated { .. }) => break,
            Err(err) => return Err(err),
        }
        if frames.len() > 1024 {
            break;
        }
    }
    Ok(frames)
}

pub fn parse_one_frame(cursor: &mut ByteCursor<'_>) -> Result<Frame> {
    let offset = cursor.position();
    if cursor.read_u8()? != MAGIC[0] || cursor.read_u8()? != MAGIC[1] {
        return Err(GridError::InvalidMagic { offset });
    }

    let version = cursor.read_u8()?;
    if version != VERSION {
        return Err(GridError::InvalidVersion(version));
    }

    let flags = cursor.read_u8()?;
    let session_id = cursor.read_u16_le()?;
    let sequence = cursor.read_u16_le()?;
    let fragment_index = cursor.read_u8()?;
    let fragment_count = cursor.read_u8()?;
    let kind = FrameKind::from_byte(cursor.read_u8()?);
    let declared_len = cursor.read_u16_le()? as usize;
    if declared_len > MAX_PAYLOAD {
        return Err(GridError::InvalidLength {
            declared: declared_len,
            available: MAX_PAYLOAD,
            context: "frame payload",
        });
    }
    let header_crc = if flags & FLAG_CRC_PRESENT != 0 {
        Some(cursor.read_u16_le()?)
    } else {
        None
    };
    let payload = cursor.take(declared_len)?.to_vec();

    if let Some(expected) = header_crc {
        let actual = crc16_gridpulse(&payload);
        if actual != expected {
            return Err(GridError::CrcMismatch { expected, actual });
        }
    }

    if fragment_count == 0 || fragment_index >= fragment_count {
        return Err(GridError::InvalidLength {
            declared: fragment_index as usize,
            available: fragment_count as usize,
            context: "fragment header",
        });
    }

    Ok(Frame {
        flags,
        session_id,
        sequence,
        fragment_index,
        fragment_count,
        kind,
        payload,
        header_crc,
        offset,
    })
}

pub fn encode_seed_frame(
    kind: FrameKind,
    session_id: u16,
    sequence: u16,
    flags: u8,
    payload: &[u8],
) -> Vec<u8> {
    let mut out = Vec::with_capacity(payload.len() + 16);
    out.extend_from_slice(&MAGIC);
    out.push(VERSION);
    out.push(flags);
    out.extend_from_slice(&session_id.to_le_bytes());
    out.extend_from_slice(&sequence.to_le_bytes());
    out.push(0);
    out.push(1);
    out.push(kind.as_byte());
    out.extend_from_slice(&(payload.len() as u16).to_le_bytes());
    if flags & FLAG_CRC_PRESENT != 0 {
        out.extend_from_slice(&crc16_gridpulse(payload).to_le_bytes());
    }
    out.extend_from_slice(payload);
    out
}
