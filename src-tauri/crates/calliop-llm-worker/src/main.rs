//! Sidecar process for local LLM cleanup. Keeps llama.cpp out of the main binary
//! to avoid ggml symbol conflicts with whisper-rs.

use std::env;
use std::io::{self, BufRead, Write};
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};

use calliop_prompt::{
    build_cleanup_user_message, build_system_prompt, validate_cleanup_output, ToneProfile,
    QWEN3_CHAT_TEMPLATE,
};
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::context::LlamaContext;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaChatMessage, LlamaChatTemplate, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;
use serde::{Deserialize, Serialize};

const CLEANUP_CONTEXT_TOKENS: u32 = 2048;
const CLEANUP_MAX_OUTPUT_TOKENS: i32 = 128;
const DEFAULT_SEED: u32 = 42;

fn resolve_chat_template(model: &LlamaModel) -> Result<LlamaChatTemplate, String> {
    match model.chat_template(None) {
        Ok(template) => Ok(template),
        Err(err) => {
            eprintln!("model chat template missing ({err}), using Qwen3 fallback");
            LlamaChatTemplate::new(QWEN3_CHAT_TEMPLATE).map_err(|e| e.to_string())
        }
    }
}

#[derive(Debug, Deserialize)]
struct WorkerRequest {
    shutdown: Option<bool>,
    text: Option<String>,
    tone: Option<ToneProfile>,
}

#[derive(Debug, Serialize)]
struct WorkerResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

struct InferenceEngine {
    backend: LlamaBackend,
    model: LlamaModel,
    n_threads: i32,
}

impl InferenceEngine {
    fn load(model_path: &Path) -> Result<Self, String> {
        let backend = LlamaBackend::init().map_err(|err| err.to_string())?;
        let n_threads = std::thread::available_parallelism()
            .map(|n| n.get() as i32)
            .unwrap_or(4)
            .min(8);
        let model = LlamaModel::load_from_file(&backend, model_path, &LlamaModelParams::default())
            .map_err(|err| err.to_string())?;
        Ok(Self {
            backend,
            model,
            n_threads,
        })
    }

    fn new_session_context(&self) -> Result<(LlamaContext<'_>, LlamaBatch), String> {
        let ctx_size = NonZeroU32::new(CLEANUP_CONTEXT_TOKENS).expect("non-zero context");
        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(Some(ctx_size))
            .with_n_threads(self.n_threads);
        let ctx = self
            .model
            .new_context(&self.backend, ctx_params)
            .map_err(|err| err.to_string())?;
        let batch = LlamaBatch::new(CLEANUP_CONTEXT_TOKENS as usize, 1);
        Ok((ctx, batch))
    }

    fn cleanup_with_context(
        &self,
        ctx: &mut LlamaContext<'_>,
        batch: &mut LlamaBatch,
        raw: &str,
        tone: ToneProfile,
    ) -> Result<String, String> {
        let raw = raw.trim();
        if raw.is_empty() {
            return Err("empty input text".into());
        }

        ctx.clear_kv_cache();

        let system_prompt = build_system_prompt(tone);
        let user_message = build_cleanup_user_message(raw).map_err(|err| err.to_string())?;
        let messages = vec![
            LlamaChatMessage::new("system".into(), system_prompt).map_err(|err| err.to_string())?,
            LlamaChatMessage::new("user".into(), user_message).map_err(|err| err.to_string())?,
        ];

        let template = resolve_chat_template(&self.model)?;
        let prompt = self
            .model
            .apply_chat_template(&template, &messages, true)
            .map_err(|err| err.to_string())?;
        let tokens = self
            .model
            .str_to_token(&prompt, AddBos::Never)
            .map_err(|err| err.to_string())?;

        let max_tokens =
            (tokens.len() as i32 + CLEANUP_MAX_OUTPUT_TOKENS).min(CLEANUP_CONTEXT_TOKENS as i32);
        if tokens.len() >= max_tokens as usize {
            return Err("prompt too long".into());
        }

        batch.clear();
        let last_index = (tokens.len().saturating_sub(1)) as i32;
        for (i, token) in (0_i32..).zip(tokens) {
            batch
                .add(token, i, &[0], i == last_index)
                .map_err(|err| err.to_string())?;
        }
        ctx.decode(batch).map_err(|err| err.to_string())?;

        let mut decoder = encoding_rs::UTF_8.new_decoder();
        let mut output = String::new();
        let mut n_cur = batch.n_tokens();
        let mut sampler =
            LlamaSampler::chain_simple([LlamaSampler::dist(DEFAULT_SEED), LlamaSampler::greedy()]);

        let mut generated = 0_i32;
        while n_cur < max_tokens && generated < CLEANUP_MAX_OUTPUT_TOKENS {
            let token = sampler.sample(&ctx, batch.n_tokens() - 1);
            sampler.accept(token);
            if self.model.is_eog_token(token) {
                break;
            }

            let piece = self
                .model
                .token_to_piece(token, &mut decoder, false, None)
                .map_err(|err| err.to_string())?;
            output.push_str(&piece);

            batch.clear();
            batch
                .add(token, n_cur, &[0], true)
                .map_err(|err| err.to_string())?;
            n_cur += 1;
            ctx.decode(batch).map_err(|err| err.to_string())?;
            generated += 1;
        }

        validate_cleanup_output(raw, &output).map_err(|err| {
            if output.trim().is_empty() {
                eprintln!("llm cleanup produced no tokens before validation");
            } else {
                eprintln!("llm cleanup raw output before validation: {output:?}");
            }
            err.to_string()
        })
    }
}

fn write_response(response: &WorkerResponse) {
    let line = serde_json::to_string(response)
        .unwrap_or_else(|_| r#"{"error":"failed to serialize worker response"}"#.to_string());
    println!("{line}");
    let _ = io::stdout().flush();
}

fn parse_model_path(args: &[String]) -> Result<PathBuf, String> {
    for (index, arg) in args.iter().enumerate() {
        if arg == "--model-path" {
            return args
                .get(index + 1)
                .map(PathBuf::from)
                .ok_or_else(|| "missing value for --model-path".into());
        }
    }
    Err("missing --model-path".into())
}

fn parse_oneshot_text(args: &[String]) -> Result<String, String> {
    let mut text_parts = Vec::new();
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--oneshot" | "--serve" => index += 1,
            "--model-path" => {
                if index + 1 >= args.len() {
                    return Err("missing value for --model-path".into());
                }
                index += 2;
            }
            arg if arg.starts_with('-') => return Err(format!("unknown flag: {arg}")),
            arg => {
                text_parts.push(arg);
                index += 1;
            }
        }
    }
    let text = text_parts.join(" ");
    if text.is_empty() {
        Err("missing text argument".into())
    } else {
        Ok(text)
    }
}

fn serve(model_path: PathBuf) -> Result<(), String> {
    let engine = InferenceEngine::load(&model_path)?;
    let (mut ctx, mut batch) = engine.new_session_context()?;
    write_response(&WorkerResponse {
        text: Some(String::new()),
        error: None,
    });

    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    while let Some(line) = lines.next().transpose().map_err(|err| err.to_string())? {
        let request: WorkerRequest =
            serde_json::from_str(&line).map_err(|err| format!("invalid request: {err}"))?;
        if request.shutdown.unwrap_or(false) {
            break;
        }

        let Some(text) = request.text else {
            write_response(&WorkerResponse {
                text: None,
                error: Some("missing text field".into()),
            });
            continue;
        };

        let tone = request.tone.unwrap_or_default();
        match engine.cleanup_with_context(&mut ctx, &mut batch, &text, tone) {
            Ok(cleaned) => write_response(&WorkerResponse {
                text: Some(cleaned),
                error: None,
            }),
            Err(err) => write_response(&WorkerResponse {
                text: None,
                error: Some(err),
            }),
        }
    }

    Ok(())
}

fn oneshot(model_path: PathBuf, text: String) -> Result<(), String> {
    let engine = InferenceEngine::load(&model_path)?;
    let (mut ctx, mut batch) = engine.new_session_context()?;
    let cleaned = engine.cleanup_with_context(&mut ctx, &mut batch, &text, ToneProfile::Default)?;
    println!("{cleaned}");
    Ok(())
}

fn usage() -> ! {
    eprintln!("Usage:");
    eprintln!("  calliop-llm-worker --serve --model-path <path>");
    eprintln!("  calliop-llm-worker --oneshot --model-path <path> \"text to clean\"");
    std::process::exit(1);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        usage();
    }

    let model_path = match parse_model_path(&args) {
        Ok(path) => path,
        Err(err) => {
            eprintln!("{err}");
            usage();
        }
    };

    let result = if args.iter().any(|arg| arg == "--serve") {
        serve(model_path)
    } else if args.iter().any(|arg| arg == "--oneshot") {
        match parse_oneshot_text(&args) {
            Ok(text) => oneshot(model_path, text),
            Err(err) => Err(err),
        }
    } else {
        Err("expected --serve or --oneshot".into())
    };

    if let Err(err) = result {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_oneshot_text_skips_model_path_value() {
        let args = vec![
            "calliop-llm-worker".into(),
            "--oneshot".into(),
            "--model-path".into(),
            "/tmp/model.gguf".into(),
            "bonjour".into(),
            "monde".into(),
        ];
        assert_eq!(parse_oneshot_text(&args).unwrap(), "bonjour monde");
    }
}
