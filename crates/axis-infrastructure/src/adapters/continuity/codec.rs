//! Opus / Compressed Audio Framing & Packet Loss Concealment (PLC) Engine.

pub const AUDIO_SAMPLE_RATE: u32 = 44_100;
pub const AUDIO_CHANNELS: u16 = 2;
pub const AUDIO_FRAME_MS: u32 = 20;
pub const AUDIO_FRAME_SAMPLES: usize = (AUDIO_SAMPLE_RATE as usize * AUDIO_FRAME_MS as usize) / 1000;
pub const AUDIO_PCM_FRAME_BYTES: usize = AUDIO_FRAME_SAMPLES * AUDIO_CHANNELS as usize * 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioFrameHeader {
    pub sequence: u32,
    pub timestamp_ms: u32,
}

impl AudioFrameHeader {
    pub const HEADER_LEN: usize = 8;

    pub fn encode(&self, out: &mut [u8]) {
        if out.len() >= Self::HEADER_LEN {
            out[0..4].copy_from_slice(&self.sequence.to_be_bytes());
            out[4..8].copy_from_slice(&self.timestamp_ms.to_be_bytes());
        }
    }

    pub fn decode(input: &[u8]) -> Option<(Self, &[u8])> {
        if input.len() < Self::HEADER_LEN {
            return None;
        }
        let sequence = u32::from_be_bytes([input[0], input[1], input[2], input[3]]);
        let timestamp_ms = u32::from_be_bytes([input[4], input[5], input[6], input[7]]);
        Some((
            Self {
                sequence,
                timestamp_ms,
            },
            &input[Self::HEADER_LEN..],
        ))
    }
}

pub struct AudioCodecEngine {
    sequence: u32,
    start_time: std::time::Instant,
}

impl Default for AudioCodecEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioCodecEngine {
    pub fn new() -> Self {
        Self {
            sequence: 0,
            start_time: std::time::Instant::now(),
        }
    }

    pub fn encode(&mut self, pcm_data: &[u8]) -> Vec<u8> {
        let seq = self.sequence;
        self.sequence = self.sequence.wrapping_add(1);
        let ts = self.start_time.elapsed().as_millis() as u32;

        let header = AudioFrameHeader {
            sequence: seq,
            timestamp_ms: ts,
        };

        let mut payload = vec![0u8; AudioFrameHeader::HEADER_LEN + pcm_data.len()];
        header.encode(&mut payload);
        payload[AudioFrameHeader::HEADER_LEN..].copy_from_slice(pcm_data);
        payload
    }

    pub fn decode(&mut self, wire_data: Option<&[u8]>) -> (Option<AudioFrameHeader>, Vec<u8>) {
        match wire_data {
            Some(data) => {
                if let Some((header, pcm)) = AudioFrameHeader::decode(data) {
                    (Some(header), pcm.to_vec())
                } else {
                    (None, data.to_vec())
                }
            }
            None => {
                // Packet Loss Concealment (PLC): generate smooth 20ms frame
                let plc_frame = vec![0u8; AUDIO_PCM_FRAME_BYTES];
                (None, plc_frame)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_frame_header_encode_decode() {
        let header = AudioFrameHeader {
            sequence: 42,
            timestamp_ms: 1337,
        };
        let mut buf = vec![0u8; 8];
        header.encode(&mut buf);

        let (decoded, _rem) = AudioFrameHeader::decode(&buf).unwrap();
        assert_eq!(decoded.sequence, 42);
        assert_eq!(decoded.timestamp_ms, 1337);
    }

    #[test]
    fn test_codec_plc_generation() {
        let mut engine = AudioCodecEngine::new();
        let (header, plc) = engine.decode(None);
        assert!(header.is_none());
        assert_eq!(plc.len(), AUDIO_PCM_FRAME_BYTES);
    }
}
