use std::collections::VecDeque;

use thiserror::Error;
use voice_activity_detector::VoiceActivityDetector;

use super::TARGET_SAMPLE_RATE;

pub const VAD_CHUNK_SIZE: usize = 512;
const SPEECH_THRESHOLD: f32 = 0.5;
/// ~320 ms of silence at 512-sample frames (32 ms each).
const SILENCE_HANGOVER_FRAMES: u32 = 10;
/// Pre-speech padding (~160 ms).
const PRE_SPEECH_CHUNKS: usize = 5;

#[derive(Debug, Error)]
pub enum VadError {
    #[error("failed to initialize Silero VAD: {0}")]
    Init(String),
}

/// Segments speech from a PCM stream using Silero VAD.
pub struct VadSegmenter {
    vad: VoiceActivityDetector,
    chunk_size: usize,
    pending: Vec<f32>,
    speech: Vec<f32>,
    pre_speech: VecDeque<Vec<f32>>,
    in_speech: bool,
    silence_frames: u32,
}

impl VadSegmenter {
    pub fn new() -> Result<Self, VadError> {
        Self::with_chunk_size(VAD_CHUNK_SIZE)
    }

    pub fn with_chunk_size(chunk_size: usize) -> Result<Self, VadError> {
        let vad = VoiceActivityDetector::builder()
            .sample_rate(TARGET_SAMPLE_RATE)
            .chunk_size(chunk_size)
            .build()
            .map_err(|e| VadError::Init(e.to_string()))?;

        Ok(Self {
            vad,
            chunk_size,
            pending: Vec::new(),
            speech: Vec::new(),
            pre_speech: VecDeque::with_capacity(PRE_SPEECH_CHUNKS + 1),
            in_speech: false,
            silence_frames: 0,
        })
    }

    /// Push resampled 16 kHz mono samples; returns completed speech segments.
    pub fn push(&mut self, samples: &[f32]) -> Result<Vec<Vec<f32>>, VadError> {
        self.pending.extend_from_slice(samples);
        let mut completed = Vec::new();
        let chunk_size = self.chunk_size;

        while self.pending.len() >= chunk_size {
            let chunk: Vec<f32> = self.pending.drain(..chunk_size).collect();
            let probability = self.vad.predict(chunk.iter().copied());
            self.process_chunk(chunk, probability, &mut completed)?;
        }

        Ok(completed)
    }

    /// Flush trailing audio at end of recording.
    pub fn flush(&mut self) -> Result<Vec<Vec<f32>>, VadError> {
        let mut completed = Vec::new();
        let chunk_size = self.chunk_size;

        if !self.pending.is_empty() {
            let mut chunk = self.pending.drain(..).collect::<Vec<_>>();
            chunk.resize(chunk_size, 0.0);
            let probability = self.vad.predict(chunk.iter().copied());
            self.process_chunk(chunk, probability, &mut completed)?;
        }

        if self.in_speech && !self.speech.is_empty() {
            completed.push(std::mem::take(&mut self.speech));
            self.in_speech = false;
            self.silence_frames = 0;
        }

        self.pre_speech.clear();
        Ok(completed)
    }

    pub fn reset(&mut self) {
        self.vad.reset();
        self.pending.clear();
        self.speech.clear();
        self.pre_speech.clear();
        self.in_speech = false;
        self.silence_frames = 0;
    }

    fn process_chunk(
        &mut self,
        chunk: Vec<f32>,
        probability: f32,
        completed: &mut Vec<Vec<f32>>,
    ) -> Result<(), VadError> {
        let is_speech = probability >= SPEECH_THRESHOLD;

        if is_speech {
            if !self.in_speech {
                for buffered in self.pre_speech.drain(..) {
                    self.speech.extend(buffered);
                }
                self.in_speech = true;
            }
            self.speech.extend_from_slice(&chunk);
            self.silence_frames = 0;
        } else if self.in_speech {
            self.speech.extend_from_slice(&chunk);
            self.silence_frames += 1;
            if self.silence_frames >= SILENCE_HANGOVER_FRAMES {
                completed.push(std::mem::take(&mut self.speech));
                self.in_speech = false;
                self.silence_frames = 0;
            }
        } else {
            self.pre_speech.push_back(chunk);
            if self.pre_speech.len() > PRE_SPEECH_CHUNKS {
                self.pre_speech.pop_front();
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_produces_no_segments() {
        let mut vad = VadSegmenter::new().expect("vad");
        let silence = vec![0.0_f32; VAD_CHUNK_SIZE * 4];
        let segments = vad.push(&silence).expect("push");
        assert!(segments.is_empty());
        let flushed = vad.flush().expect("flush");
        assert!(flushed.is_empty());
    }

    #[test]
    fn with_chunk_size_256_initializes() {
        let vad = VadSegmenter::with_chunk_size(256);
        assert!(vad.is_ok());
    }

    #[test]
    fn processes_non_silent_audio_without_error() {
        let mut vad = VadSegmenter::new().expect("vad");
        let mut samples = Vec::new();
        for i in 0..VAD_CHUNK_SIZE * 20 {
            let t = i as f32 / TARGET_SAMPLE_RATE as f32;
            samples.push((t * 440.0 * 2.0 * std::f32::consts::PI).sin() * 0.8);
        }
        for chunk in samples.chunks(VAD_CHUNK_SIZE) {
            vad.push(chunk).expect("push");
        }
        let _segments = vad.flush().expect("flush");
    }
}
