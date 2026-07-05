use crate::crc::folded_checksum;
use crate::cursor::ByteCursor;
use crate::error::Result;
use crate::model::ReplayReport;
use crate::session::Decoder;

pub struct ReplayRunner {
    decoder: Decoder,
}

impl ReplayRunner {
    pub fn new() -> Self {
        Self {
            decoder: Decoder::new(),
        }
    }

    pub fn run(&mut self, data: &[u8]) -> Result<ReplayReport> {
        let mut cursor = ByteCursor::new(data);
        let mut report = ReplayReport::new();
        report.fingerprint = folded_checksum(data);

        if data.starts_with(b"GPLR") {
            cursor.seek(4)?;
            while cursor.remaining() >= 2 {
                let declared = cursor.read_u16_le()? as usize;
                let len = declared.min(cursor.remaining());
                let segment = cursor.take(len)?;
                match self.decoder.ingest_stream(segment) {
                    Ok(batch) => {
                        report.accepted_segments += 1;
                        report.batches.push(batch);
                    }
                    Err(_) => {
                        report.rejected_segments += 1;
                    }
                }
                if report.accepted_segments + report.rejected_segments > 512 {
                    break;
                }
            }
        } else {
            match self.decoder.ingest_stream(data) {
                Ok(batch) => {
                    report.accepted_segments = 1;
                    report.batches.push(batch);
                }
                Err(_) => {
                    report.rejected_segments = 1;
                }
            }
        }

        Ok(report)
    }
}

impl Default for ReplayRunner {
    fn default() -> Self {
        Self::new()
    }
}
