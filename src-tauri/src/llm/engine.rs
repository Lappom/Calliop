use super::client::WorkerClient;
use super::model::ensure_llm_model_blocking;
use super::LlmError;

pub struct LlamaEngine {
    client: WorkerClient,
}

impl LlamaEngine {
    pub fn start() -> Result<Self, LlmError> {
        Ok(Self {
            client: WorkerClient::spawn()?,
        })
    }

    pub fn cleanup(&mut self, raw: &str) -> Result<String, LlmError> {
        self.client.cleanup(raw)
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

pub fn ensure_engine_ready() -> Result<LlamaEngine, LlmError> {
    ensure_llm_model_blocking(None).map_err(|err| LlmError::Worker(err.to_string()))?;
    LlamaEngine::start()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_requires_downloaded_model_or_worker() {
        let path = super::super::model::model_path();
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
        assert!(LlamaEngine::start().is_ok());
    }
}
