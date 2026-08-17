//! Adaptive Receiver Jitter Buffer for Low-Latency Audio Streaming.

use super::codec::{AUDIO_PCM_FRAME_BYTES, AudioCodecEngine};
use log::debug;
use std::collections::BTreeMap;

pub struct AdaptiveJitterBuffer {
    buffer: BTreeMap<u32, Vec<u8>>,
    expected_sequence: u32,
    initialized: bool,
    max_buffer_frames: usize,
    codec: AudioCodecEngine,
}

impl Default for AdaptiveJitterBuffer {
    fn default() -> Self {
        Self::new(5)
    }
}

impl AdaptiveJitterBuffer {
    pub fn new(max_buffer_frames: usize) -> Self {
        Self {
            buffer: BTreeMap::new(),
            expected_sequence: 0,
            initialized: false,
            max_buffer_frames,
            codec: AudioCodecEngine::new(),
        }
    }

    pub fn push(&mut self, data: &[u8]) {
        let (header_opt, payload) = self.codec.decode(Some(data));
        if let Some(header) = header_opt {
            if !self.initialized {
                self.expected_sequence = header.sequence;
                self.initialized = true;
            }
            if header.sequence >= self.expected_sequence {
                self.buffer.insert(header.sequence, payload);
            }
        } else {
            self.buffer.insert(self.expected_sequence, payload);
        }

        while self.buffer.len() > self.max_buffer_frames {
            let oldest = *self.buffer.keys().next().unwrap();
            self.buffer.remove(&oldest);
        }
    }

    pub fn pop_next(&mut self) -> Vec<u8> {
        if !self.initialized {
            let (_, plc) = self.codec.decode(None);
            return plc;
        }

        if let Some(payload) = self.buffer.remove(&self.expected_sequence) {
            self.expected_sequence = self.expected_sequence.wrapping_add(1);
            payload
        } else if let Some(&next_seq) = self.buffer.keys().next() {
            if next_seq > self.expected_sequence
                && next_seq.wrapping_sub(self.expected_sequence) < 10
            {
                debug!(
                    "[continuity-audio] sequence gap ({}), issuing PLC frame",
                    self.expected_sequence
                );
                self.expected_sequence = self.expected_sequence.wrapping_add(1);
                let (_, plc) = self.codec.decode(None);
                plc
            } else {
                self.expected_sequence = next_seq;
                self.buffer
                    .remove(&next_seq)
                    .unwrap_or_else(|| vec![0u8; AUDIO_PCM_FRAME_BYTES])
            }
        } else {
            let (_, plc) = self.codec.decode(None);
            plc
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jitter_buffer_ordering_and_plc() {
        let mut jb = AdaptiveJitterBuffer::new(5);
        let mut engine = AudioCodecEngine::new();

        let frame1 = engine.encode(&vec![1u8; 100]);
        let frame2 = engine.encode(&vec![2u8; 100]);

        jb.push(&frame1);
        jb.push(&frame2);

        let pop1 = jb.pop_next();
        assert_eq!(pop1, vec![1u8; 100]);

        let pop2 = jb.pop_next();
        assert_eq!(pop2, vec![2u8; 100]);
    }
}
