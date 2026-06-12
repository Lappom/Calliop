//! Runtime performance profile resolution from host capabilities and settings.

use std::time::Duration;

use crate::llm::LlmModel;
use crate::store::AppSettings;
use crate::stt::WhisperModel;

use super::capabilities::{
    SystemCapabilities, MINIMIZED_PRELOAD_MIN_AVAIL_RAM_BYTES, PRELOAD_MIN_AVAIL_RAM_BYTES,
};

const GB: u64 = 1024 * 1024 * 1024;
const WHISPER_UNLOAD_IDLE: Duration = Duration::from_secs(600);

pub const DEFAULT_VAD_CHUNK_SIZE: usize = 512;
pub const MIN_VAD_CHUNK_SIZE: usize = 256;
pub const MAX_VAD_CHUNK_SIZE: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerfTier {
    Eco,
    Balanced,
    HighBalanced,
    Performance,
}

impl PerfTier {
    pub fn label(self) -> &'static str {
        match self {
            Self::Eco => "eco",
            Self::Balanced => "balanced",
            Self::HighBalanced => "balanced",
            Self::Performance => "performance",
        }
    }
}

struct TierDefaults {
    whisper: WhisperModel,
    llm: LlmModel,
    vad_chunk_size: usize,
    stt_threads: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimePerfConfig {
    pub whisper: WhisperModel,
    pub llm: LlmModel,
    pub vad_chunk_size: usize,
    pub stt_threads: i32,
    pub preload_whisper: bool,
    pub preload_llm: bool,
    pub llm_lazy_load: bool,
    pub whisper_unload_idle: Option<Duration>,
    pub tier: PerfTier,
}

pub fn resolve_whisper_model(setting: WhisperModel, caps: &SystemCapabilities) -> WhisperModel {
    if setting != WhisperModel::Auto {
        return setting;
    }
    tier_defaults(tier_from_caps(caps), caps.cpu_logical_cores).whisper
}

pub fn resolve_llm_model(setting: LlmModel, caps: &SystemCapabilities) -> LlmModel {
    if setting != LlmModel::Auto {
        return setting;
    }
    tier_defaults(tier_from_caps(caps), caps.cpu_logical_cores).llm
}

pub fn resolve_perf_config(
    settings: &AppSettings,
    caps: &SystemCapabilities,
    start_minimized: bool,
) -> RuntimePerfConfig {
    let tier = tier_from_caps(caps);
    let defaults = tier_defaults(tier, caps.cpu_logical_cores);

    let whisper = resolve_whisper_model(settings.whisper_model(), caps);
    let llm = resolve_llm_model(settings.llm_model(), caps);

    let (vad_chunk_size, stt_threads) = if settings.adaptive_perf() {
        (defaults.vad_chunk_size, defaults.stt_threads)
    } else {
        (
            DEFAULT_VAD_CHUNK_SIZE,
            stt_threads_for_cores(caps.cpu_logical_cores, 8),
        )
    };

    let low_power = settings.low_power_mode();
    let llm_lazy_load = low_power;

    let whisper_unload_idle = if low_power {
        Some(WHISPER_UNLOAD_IDLE)
    } else {
        None
    };

    let mut preload_whisper = !low_power;
    let mut preload_llm = !low_power && settings.auto_edit && !llm_lazy_load;

    if start_minimized {
        preload_whisper =
            !low_power && caps.avail_ram_bytes >= MINIMIZED_PRELOAD_MIN_AVAIL_RAM_BYTES;
        preload_llm = !low_power
            && settings.auto_edit
            && caps.avail_ram_bytes >= MINIMIZED_PRELOAD_MIN_AVAIL_RAM_BYTES;
    } else if preload_whisper {
        preload_whisper = caps.avail_ram_bytes >= PRELOAD_MIN_AVAIL_RAM_BYTES;
    }

    RuntimePerfConfig {
        whisper,
        llm,
        vad_chunk_size,
        stt_threads,
        preload_whisper,
        preload_llm,
        llm_lazy_load,
        whisper_unload_idle,
        tier,
    }
}

fn tier_from_caps(caps: &SystemCapabilities) -> PerfTier {
    if caps.total_ram_bytes >= 32 * GB && caps.gpu_compiled {
        PerfTier::Performance
    } else if caps.total_ram_bytes >= 16 * GB && caps.gpu_compiled {
        PerfTier::HighBalanced
    } else if caps.total_ram_bytes >= 12 * GB {
        PerfTier::Balanced
    } else {
        PerfTier::Eco
    }
}

fn tier_defaults(tier: PerfTier, cpu_cores: u32) -> TierDefaults {
    match tier {
        PerfTier::Eco => TierDefaults {
            whisper: WhisperModel::Small,
            llm: LlmModel::Qwen3_0_6B,
            vad_chunk_size: 256,
            stt_threads: stt_threads_for_cores(cpu_cores, 4),
        },
        PerfTier::Balanced => TierDefaults {
            whisper: WhisperModel::Small,
            llm: LlmModel::Qwen3_1_7B,
            vad_chunk_size: 512,
            stt_threads: stt_threads_for_cores(cpu_cores, 6),
        },
        PerfTier::HighBalanced => TierDefaults {
            whisper: WhisperModel::DistilFrDec16,
            llm: LlmModel::Qwen3_1_7B,
            vad_chunk_size: 512,
            stt_threads: stt_threads_for_cores(cpu_cores, 8),
        },
        PerfTier::Performance => TierDefaults {
            whisper: WhisperModel::DistilFrDec16,
            llm: LlmModel::Qwen3_4B,
            vad_chunk_size: 1024,
            stt_threads: stt_threads_for_cores(cpu_cores, 8),
        },
    }
}

fn stt_threads_for_cores(cpu_cores: u32, cap: i32) -> i32 {
    (cpu_cores as i32).min(cap).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::InferenceBackend;

    fn caps(total_gb: u64, avail_gb: u64, gpu: bool, cores: u32) -> SystemCapabilities {
        SystemCapabilities {
            total_ram_bytes: total_gb * GB,
            avail_ram_bytes: avail_gb * GB,
            cpu_logical_cores: cores,
            gpu_compiled: gpu,
        }
    }

    fn settings(whisper: &str, llm: &str, low_power: bool, adaptive: bool) -> AppSettings {
        AppSettings {
            auto_edit: true,
            auto_learn: true,
            auto_update: false,
            stt_language: "fr".into(),
            whisper_model: whisper.into(),
            llm_model: llm.into(),
            hotkey: "Alt+Space".into(),
            inference_backend: InferenceBackend::Auto.as_setting_value().into(),
            low_power_mode: low_power,
            adaptive_perf: adaptive,
            ui_language: "fr".into(),
            input_device: crate::audio::DEFAULT_INPUT_DEVICE_ID.into(),
        }
    }

    #[test]
    fn eco_tier_resolves_auto_models() {
        let c = caps(8, 4, false, 4);
        assert_eq!(
            resolve_whisper_model(WhisperModel::Auto, &c),
            WhisperModel::Small
        );
        assert_eq!(resolve_llm_model(LlmModel::Auto, &c), LlmModel::Qwen3_0_6B);
    }

    #[test]
    fn performance_tier_with_gpu() {
        let c = caps(32, 16, true, 8);
        let cfg = resolve_perf_config(&settings("auto", "auto", false, true), &c, false);
        assert_eq!(cfg.whisper, WhisperModel::DistilFrDec16);
        assert_eq!(cfg.llm, LlmModel::Qwen3_4B);
        assert_eq!(cfg.vad_chunk_size, 1024);
        assert_eq!(cfg.tier, PerfTier::Performance);
    }

    #[test]
    fn manual_models_are_not_overridden() {
        let c = caps(32, 16, true, 8);
        let cfg = resolve_perf_config(&settings("small", "qwen3-0.6b", false, true), &c, false);
        assert_eq!(cfg.whisper, WhisperModel::Small);
        assert_eq!(cfg.llm, LlmModel::Qwen3_0_6B);
    }

    #[test]
    fn low_power_defers_preload_and_enables_unload() {
        let c = caps(16, 8, false, 8);
        let cfg = resolve_perf_config(&settings("auto", "auto", true, true), &c, false);
        assert!(!cfg.preload_whisper);
        assert!(!cfg.preload_llm);
        assert!(cfg.llm_lazy_load);
        assert_eq!(cfg.whisper_unload_idle, Some(WHISPER_UNLOAD_IDLE));
    }

    #[test]
    fn minimized_preloads_when_enough_ram() {
        let c = caps(16, 8, false, 8);
        let mut s = settings("auto", "auto", false, true);
        s.auto_edit = true;
        let cfg = resolve_perf_config(&s, &c, true);
        assert!(cfg.preload_whisper);
        assert!(cfg.preload_llm);
    }

    #[test]
    fn minimized_skips_preload_on_low_ram() {
        let c = caps(16, 2, false, 8);
        let cfg = resolve_perf_config(&settings("auto", "auto", false, true), &c, true);
        assert!(!cfg.preload_whisper);
    }
}
