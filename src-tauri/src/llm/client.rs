use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};

use serde::{Deserialize, Serialize};

use super::model::model_path;
use super::LlmError;

#[derive(Debug, Serialize)]
struct WorkerRequest<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    shutdown: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
struct WorkerResponse {
    text: Option<String>,
    error: Option<String>,
}

pub struct WorkerClient {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<std::process::ChildStdout>,
}

impl WorkerClient {
    pub fn spawn() -> Result<Self, LlmError> {
        let model_path = model_path();
        if !model_path.exists() {
            return Err(LlmError::Worker(format!(
                "model not found at {}",
                model_path.display()
            )));
        }

        let worker_exe = resolve_worker_exe()?;
        let mut child = Command::new(&worker_exe)
            .arg("--serve")
            .arg("--model-path")
            .arg(&model_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|err| {
                LlmError::Worker(format!("failed to spawn {}: {err}", worker_exe.display()))
            })?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| LlmError::Worker("worker stdin unavailable".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| LlmError::Worker("worker stdout unavailable".into()))?;
        let mut stdout = BufReader::new(stdout);

        let mut ready_line = String::new();
        stdout
            .read_line(&mut ready_line)
            .map_err(|err| LlmError::Worker(format!("worker ready handshake failed: {err}")))?;

        let ready: WorkerResponse = serde_json::from_str(ready_line.trim())
            .map_err(|err| LlmError::Worker(format!("invalid worker ready payload: {err}")))?;

        if ready.error.is_some() {
            return Err(LlmError::Worker(
                ready
                    .error
                    .unwrap_or_else(|| "worker failed to start".into()),
            ));
        }

        Ok(Self {
            child,
            stdin,
            stdout,
        })
    }

    pub fn cleanup(&mut self, raw: &str) -> Result<String, LlmError> {
        let payload = serde_json::to_string(&WorkerRequest {
            shutdown: None,
            text: Some(raw),
        })
        .map_err(|err| LlmError::Worker(err.to_string()))?;
        writeln!(self.stdin, "{payload}")
            .map_err(|err| LlmError::Worker(format!("worker write failed: {err}")))?;
        self.stdin
            .flush()
            .map_err(|err| LlmError::Worker(format!("worker flush failed: {err}")))?;

        let mut line = String::new();
        self.stdout
            .read_line(&mut line)
            .map_err(|err| LlmError::Worker(format!("worker read failed: {err}")))?;

        let response: WorkerResponse = serde_json::from_str(line.trim())
            .map_err(|err| LlmError::Worker(format!("invalid worker response: {err}")))?;

        if let Some(error) = response.error {
            return Err(LlmError::Worker(error));
        }

        response
            .text
            .ok_or_else(|| LlmError::Worker("worker returned empty text".into()))
    }
}

impl Drop for WorkerClient {
    fn drop(&mut self) {
        let payload = serde_json::to_string(&WorkerRequest {
            shutdown: Some(true),
            text: None,
        })
        .unwrap_or_else(|_| r#"{"shutdown":true}"#.to_string());
        let _ = writeln!(self.stdin, "{payload}");
        let _ = self.stdin.flush();
        let _ = self.child.wait();
    }
}

fn resolve_worker_exe() -> Result<PathBuf, LlmError> {
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_calliop_llm_worker") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Ok(path);
        }
    }

    if let Ok(current) = std::env::current_exe() {
        if let Some(dir) = current.parent() {
            let sidecar = dir.join(format!(
                "calliop-llm-worker{}",
                std::env::consts::EXE_SUFFIX
            ));
            if sidecar.exists() {
                return Ok(sidecar);
            }
        }
    }

    let dev_candidate = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("debug")
        .join(format!(
            "calliop-llm-worker{}",
            std::env::consts::EXE_SUFFIX
        ));
    if dev_candidate.exists() {
        return Ok(dev_candidate);
    }

    Err(LlmError::Worker(
        "calliop-llm-worker executable not found; build with \
         `cargo build --features llm-worker --bin calliop-llm-worker`"
            .into(),
    ))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    #[test]
    fn dev_worker_path_points_to_target_debug() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("debug")
            .join(format!(
                "calliop-llm-worker{}",
                std::env::consts::EXE_SUFFIX
            ));
        assert!(path.to_string_lossy().contains("calliop-llm-worker"));
    }
}
