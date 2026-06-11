//! Host capability detection and adaptive performance profiles.

mod capabilities;
mod profile;

pub use capabilities::{
    SystemCapabilities, MINIMIZED_PRELOAD_MIN_AVAIL_RAM_BYTES, PRELOAD_MIN_AVAIL_RAM_BYTES,
};
pub use profile::{
    resolve_llm_model, resolve_perf_config, resolve_whisper_model, PerfTier, RuntimePerfConfig,
    DEFAULT_VAD_CHUNK_SIZE, MAX_VAD_CHUNK_SIZE, MIN_VAD_CHUNK_SIZE,
};

pub fn module_name() -> &'static str {
    "system"
}
