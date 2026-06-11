//! Audio capture and voice activity detection (Phase 1+).

mod capture;
mod vad;

pub use capture::{
    resample_to_16k_mono, AudioCapture, AudioChunkSender, AudioError, AudioLevelSender,
    TARGET_SAMPLE_RATE,
};
pub use vad::{VadError, VadSegmenter, VAD_CHUNK_SIZE};

pub fn module_name() -> &'static str {
    "audio"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_module_name() {
        assert_eq!(module_name(), "audio");
    }
}
