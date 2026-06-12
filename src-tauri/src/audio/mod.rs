//! Audio capture and voice activity detection (Phase 1+).

mod capture;
mod devices;
mod vad;

pub use capture::{
    resample_to_16k_mono, AudioCapture, AudioChunkSender, AudioError, AudioLevelSample,
    AudioLevelSender, AUDIO_BAND_COUNT, TARGET_SAMPLE_RATE,
};
pub use devices::{list_input_devices, InputDeviceInfo, DEFAULT_INPUT_DEVICE_ID};
pub use vad::{SpeechSegment, VadError, VadSegmenter, VAD_CHUNK_SIZE};

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
