//! Inference backend selection (CPU default, optional Vulkan GPU).

use serde::Serialize;

use crate::store::InferenceBackend;

#[derive(Debug, Clone, Serialize)]
pub struct InferenceInfo {
    pub compiled_backend: String,
    pub gpu_available: bool,
    pub active_backend: String,
    pub inference_backend_setting: String,
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

pub fn get_inference_info(setting: InferenceBackend) -> InferenceInfo {
    let compiled = compiled_backend_name().to_string();
    let gpu_available = cfg!(feature = "gpu");
    let use_gpu = should_use_gpu(setting);
    let active_backend = if use_gpu && gpu_available {
        "vulkan".into()
    } else {
        "cpu".into()
    };

    InferenceInfo {
        compiled_backend: compiled,
        gpu_available,
        active_backend,
        inference_backend_setting: setting.as_setting_value().into(),
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
