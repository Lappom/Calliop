//! Inference backend selection (CPU default, optional Vulkan GPU).

use serde::Serialize;

use crate::store::{AppSettings, InferenceBackend};
use crate::system::{resolve_llm_model, resolve_whisper_model, SystemCapabilities};

#[derive(Debug, Clone, Serialize)]
pub struct InferenceInfo {
    pub compiled_backend: String,
    pub gpu_available: bool,
    pub active_backend: String,
    pub inference_backend_setting: String,
    pub total_ram_gb: f64,
    pub avail_ram_gb: f64,
    pub perf_tier: String,
    pub effective_whisper: String,
    pub effective_llm: String,
    pub vad_chunk_size: usize,
}

pub fn compiled_backend_name() -> &'static str {
    if cfg!(feature = "gpu") {
        "vulkan"
    } else {
        "cpu"
    }
}

pub fn should_use_gpu(setting: InferenceBackend) -> bool {
    match setting {
        InferenceBackend::Cpu => false,
        InferenceBackend::Auto => cfg!(feature = "gpu"),
    }
}

pub fn gpu_layers(setting: InferenceBackend) -> u32 {
    if should_use_gpu(setting) {
        99
    } else {
        0
    }
}

pub fn get_inference_info(settings: &AppSettings, caps: &SystemCapabilities) -> InferenceInfo {
    let compiled = compiled_backend_name().to_string();
    let gpu_available = cfg!(feature = "gpu");
    let use_gpu = should_use_gpu(settings.inference_backend());
    let active_backend = if use_gpu && gpu_available {
        "vulkan".into()
    } else {
        "cpu".into()
    };

    let perf = crate::system::resolve_perf_config(settings, caps, false);
    let effective_whisper = resolve_whisper_model(settings.whisper_model(), caps);
    let effective_llm = resolve_llm_model(settings.llm_model(), caps);

    InferenceInfo {
        compiled_backend: compiled,
        gpu_available,
        active_backend,
        inference_backend_setting: settings.inference_backend().as_setting_value().into(),
        total_ram_gb: caps.total_ram_gb(),
        avail_ram_gb: caps.avail_ram_gb(),
        perf_tier: perf.tier.label().into(),
        effective_whisper: effective_whisper.as_setting_value().into(),
        effective_llm: effective_llm.as_setting_value().into(),
        vad_chunk_size: perf.vad_chunk_size,
    }
}

pub fn module_name() -> &'static str {
    "inference"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_setting_disables_gpu() {
        assert!(!should_use_gpu(InferenceBackend::Cpu));
        assert_eq!(gpu_layers(InferenceBackend::Cpu), 0);
    }

    #[test]
    fn auto_respects_compile_feature() {
        let use_gpu = should_use_gpu(InferenceBackend::Auto);
        assert_eq!(use_gpu, cfg!(feature = "gpu"));
    }
}
