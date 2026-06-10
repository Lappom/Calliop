//! Audio capture and voice activity detection (Phase 1+).

mod capture;

pub use capture::{resample_to_16k_mono, AudioCapture, AudioError, TARGET_SAMPLE_RATE};

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
