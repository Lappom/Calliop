use std::path::Path;

use calliop_prompt::ToneProfile;

use super::client::WorkerClient;
use super::model::{ensure_llm_model_blocking, LlmModel};
use super::LlmError;

pub struct LlamaEngine {
    client: WorkerClient,
}

impl LlamaEngine {
    pub fn start() -> Result<Self, LlmError> {
        let model = LlmModel::Qwen3_5_2B;
        let path = model.path();
        Self::start_with_config(
            &path,
            crate::inference::gpu_layers(crate::store::InferenceBackend::Auto),
        )
    }

    pub fn start_with_config(model_path: &Path, n_gpu_layers: u32) -> Result<Self, LlmError> {
        Ok(Self {
            client: WorkerClient::spawn(model_path, n_gpu_layers)?,
        })
    }

    pub fn cleanup(&mut self, raw: &str, tone: ToneProfile) -> Result<String, LlmError> {
        self.client.cleanup(raw, tone)
    }

    pub fn pid(&self) -> u32 {
        self.client.pid()
    }
}

impl Drop for LlamaEngine {
    fn drop(&mut self) {
        // WorkerClient drop sends shutdown to the sidecar process.
    }
}

pub fn ensure_engine_ready(model: LlmModel, n_gpu_layers: u32) -> Result<LlamaEngine, LlmError> {
    let path =
        ensure_llm_model_blocking(None, model).map_err(|err| LlmError::Worker(err.to_string()))?;
    LlamaEngine::start_with_config(&path, n_gpu_layers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_requires_downloaded_model_or_worker() {
        let path = super::super::model::LlmModel::Qwen3_5_2B.path();
        if !path.exists() {
            return;
        }
        let worker_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("debug")
            .join(format!(
                "calliop-llm-worker{}",
                std::env::consts::EXE_SUFFIX
            ));
        if !worker_path.exists() {
            return;
        }
        assert!(LlamaEngine::start_with_config(
            &path,
            crate::inference::gpu_layers(crate::store::InferenceBackend::Auto),
        )
        .is_ok());
    }
}
