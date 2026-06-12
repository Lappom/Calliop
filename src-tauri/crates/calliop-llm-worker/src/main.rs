//! Sidecar process for local LLM cleanup. Keeps llama.cpp out of the main binary
//! to avoid ggml symbol conflicts with whisper-rs.

// Never show a console when spawned from the GUI app (release builds).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

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
use llama_cpp_2::openai::OpenAIChatTemplateParams;
use llama_cpp_2::sampling::LlamaSampler;
use llama_cpp_2::token::LlamaToken;
use serde::{Deserialize, Serialize};

const CLEANUP_CONTEXT_TOKENS: u32 = 2048;
const CLEANUP_MAX_OUTPUT_TOKENS: i32 = 256;

fn resolve_chat_template(
    model: &LlamaModel,
    model_path: &Path,
) -> Result<LlamaChatTemplate, String> {
    let force_qwen3_fallback = model_path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.contains("qwen3.5"));

    if force_qwen3_fallback {
        return LlamaChatTemplate::new(QWEN3_CHAT_TEMPLATE).map_err(|e| e.to_string());
    }

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

#[derive(Default)]
struct ToneKvCache {
    tone: Option<ToneProfile>,
    system_token_len: usize,
}

struct InferenceEngine {
    backend: LlamaBackend,
    model: LlamaModel,
    n_threads: i32,
    chat_template: LlamaChatTemplate,
    system_prompts: [String; 4],
}

impl InferenceEngine {
    fn load(model_path: &Path, n_gpu_layers: u32) -> Result<Self, String> {
        let backend = LlamaBackend::init().map_err(|err| err.to_string())?;
        let n_threads = std::thread::available_parallelism()
            .map(|n| n.get() as i32)
            .unwrap_or(4)
            .min(8);
        let model_params = LlamaModelParams::default().with_n_gpu_layers(n_gpu_layers);
        let model = LlamaModel::load_from_file(&backend, model_path, &model_params)
            .map_err(|err| err.to_string())?;
        let chat_template = resolve_chat_template(&model, model_path)?;
        let system_prompts = [
            build_system_prompt(ToneProfile::Default),
            build_system_prompt(ToneProfile::Casual),
            build_system_prompt(ToneProfile::Formal),
            build_system_prompt(ToneProfile::Technical),
        ];
        Ok(Self {
            backend,
            model,
            n_threads,
            chat_template,
            system_prompts,
        })
    }

    fn system_prompt(&self, tone: ToneProfile) -> &str {
        match tone {
            ToneProfile::Default => &self.system_prompts[0],
            ToneProfile::Casual => &self.system_prompts[1],
            ToneProfile::Formal => &self.system_prompts[2],
            ToneProfile::Technical => &self.system_prompts[3],
        }
    }

    fn new_session_context(&self) -> Result<(LlamaContext<'_>, LlamaBatch<'_>), String> {
        let ctx_size = NonZeroU32::new(CLEANUP_CONTEXT_TOKENS).expect("non-zero context");
        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(Some(ctx_size))
            .with_n_threads(self.n_threads)
            .with_n_threads_batch(self.n_threads);
        let ctx = self
            .model
            .new_context(&self.backend, ctx_params)
            .map_err(|err| err.to_string())?;
        let batch = LlamaBatch::new(CLEANUP_CONTEXT_TOKENS as usize, 1);
        Ok((ctx, batch))
    }

    fn render_chat_prompt(
        &self,
        messages: &[(&str, &str)],
        add_generation_prompt: bool,
    ) -> Result<String, String> {
        let payload: Vec<serde_json::Value> = messages
            .iter()
            .map(|(role, content)| {
                serde_json::json!({
                    "role": role,
                    "content": content,
                })
            })
            .collect();
        let messages_json = serde_json::to_string(&payload).map_err(|err| err.to_string())?;
        let params = OpenAIChatTemplateParams {
            messages_json: &messages_json,
            tools_json: None,
            tool_choice: None,
            json_schema: None,
            grammar: None,
            reasoning_format: None,
            chat_template_kwargs: Some(r#"{"enable_thinking":false}"#),
            add_generation_prompt,
            use_jinja: true,
            parallel_tool_calls: false,
            enable_thinking: false,
            add_bos: false,
            add_eos: false,
            parse_tool_calls: false,
        };

        if let Ok(result) = self
            .model
            .apply_chat_template_oaicompat(&self.chat_template, &params)
        {
            return Ok(result.prompt);
        }

        let chat_messages: Vec<LlamaChatMessage> = messages
            .iter()
            .map(|(role, content)| {
                LlamaChatMessage::new((*role).into(), (*content).into())
                    .map_err(|err| err.to_string())
            })
            .collect::<Result<_, _>>()?;

        self.model
            .apply_chat_template(&self.chat_template, &chat_messages, add_generation_prompt)
            .map_err(|err| err.to_string())
    }

    fn tokenize_prompt(&self, messages: &[(&str, &str)]) -> Result<Vec<LlamaToken>, String> {
        let prompt = self.render_chat_prompt(messages, true)?;
        self.model
            .str_to_token(&prompt, AddBos::Never)
            .map_err(|err| err.to_string())
    }

    fn tokenize_system_prefix(&self, tone: ToneProfile) -> Result<Vec<LlamaToken>, String> {
        let system_prompt = self.system_prompt(tone);
        let messages = [("system", system_prompt)];
        let prompt = self.render_chat_prompt(&messages, false)?;
        self.model
            .str_to_token(&prompt, AddBos::Never)
            .map_err(|err| err.to_string())
    }

    fn decode_prompt_tokens(
        &self,
        ctx: &mut LlamaContext<'_>,
        batch: &mut LlamaBatch,
        tokens: &[LlamaToken],
        start_pos: i32,
    ) -> Result<(), String> {
        if tokens.is_empty() {
            return Ok(());
        }
        batch.clear();
        let last_index = tokens.len().saturating_sub(1);
        for (offset, token) in tokens.iter().enumerate() {
            let pos = start_pos + offset as i32;
            batch
                .add(*token, pos, &[0], offset == last_index)
                .map_err(|err| err.to_string())?;
        }
        ctx.decode(batch).map_err(|err| err.to_string())
    }

    fn cleanup_with_context(
        &self,
        ctx: &mut LlamaContext<'_>,
        batch: &mut LlamaBatch,
        tone_cache: &mut ToneKvCache,
        raw: &str,
        tone: ToneProfile,
    ) -> Result<String, String> {
        let raw = raw.trim();
        if raw.is_empty() {
            return Err("empty input text".into());
        }

        let user_message = build_cleanup_user_message(raw).map_err(|err| err.to_string())?;
        let system_prompt = self.system_prompt(tone);
        let messages = [("system", system_prompt), ("user", user_message.as_str())];

        let full_tokens = self.tokenize_prompt(&messages)?;
        let system_tokens = self.tokenize_system_prefix(tone)?;
        let system_prefix_matches = full_tokens.len() >= system_tokens.len()
            && full_tokens[..system_tokens.len()] == system_tokens[..];

        let reuse_system_kv = tone_cache.tone == Some(tone)
            && tone_cache.system_token_len > 0
            && tone_cache.system_token_len == system_tokens.len()
            && system_prefix_matches;

        if reuse_system_kv {
            ctx.clear_kv_cache_seq(Some(0), Some(tone_cache.system_token_len as u32), None)
                .map_err(|err| err.to_string())?;
            let suffix = &full_tokens[tone_cache.system_token_len..];
            self.decode_prompt_tokens(ctx, batch, suffix, tone_cache.system_token_len as i32)?;
        } else {
            ctx.clear_kv_cache();
            self.decode_prompt_tokens(ctx, batch, &full_tokens, 0)?;
            tone_cache.tone = Some(tone);
            tone_cache.system_token_len = if system_prefix_matches {
                system_tokens.len()
            } else {
                0
            };
        }

        let max_tokens = (full_tokens.len() as i32 + CLEANUP_MAX_OUTPUT_TOKENS)
            .min(CLEANUP_CONTEXT_TOKENS as i32);
        if full_tokens.len() >= max_tokens as usize {
            return Err("prompt too long".into());
        }

        let mut decoder = encoding_rs::UTF_8.new_decoder();
        let mut output = String::new();
        let mut n_cur = full_tokens.len() as i32;
        let mut sampler = LlamaSampler::greedy();

        let mut generated = 0_i32;
        while n_cur < max_tokens && generated < CLEANUP_MAX_OUTPUT_TOKENS {
            let token = sampler.sample(ctx, batch.n_tokens() - 1);
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

fn parse_n_gpu_layers(args: &[String]) -> u32 {
    for (index, arg) in args.iter().enumerate() {
        if arg == "--ngl" {
            if let Some(value) = args.get(index + 1) {
                return value.parse().unwrap_or(0);
            }
        }
    }
    if cfg!(feature = "gpu") {
        99
    } else {
        0
    }
}

fn parse_oneshot_text(args: &[String]) -> Result<String, String> {
    let mut text_parts = Vec::new();
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--oneshot" | "--serve" => index += 1,
            "--model-path" | "--ngl" => {
                if index + 1 >= args.len() {
                    return Err("missing value for flag".into());
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

fn serve(model_path: PathBuf, n_gpu_layers: u32) -> Result<(), String> {
    let engine = match InferenceEngine::load(&model_path, n_gpu_layers) {
        Ok(engine) => engine,
        Err(err) => {
            write_response(&WorkerResponse {
                text: None,
                error: Some(err.clone()),
            });
            return Err(err);
        }
    };
    let (mut ctx, mut batch) = match engine.new_session_context() {
        Ok(parts) => parts,
        Err(err) => {
            write_response(&WorkerResponse {
                text: None,
                error: Some(err.clone()),
            });
            return Err(err);
        }
    };
    let mut tone_cache = ToneKvCache::default();
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
        match engine.cleanup_with_context(&mut ctx, &mut batch, &mut tone_cache, &text, tone) {
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

fn oneshot(model_path: PathBuf, n_gpu_layers: u32, text: String) -> Result<(), String> {
    let engine = InferenceEngine::load(&model_path, n_gpu_layers)?;
    let (mut ctx, mut batch) = engine.new_session_context()?;
    let mut tone_cache = ToneKvCache::default();
    let cleaned = engine.cleanup_with_context(
        &mut ctx,
        &mut batch,
        &mut tone_cache,
        &text,
        ToneProfile::Default,
    )?;
    println!("{cleaned}");
    Ok(())
}

fn usage() -> ! {
    eprintln!("Usage:");
    eprintln!("  calliop-llm-worker --serve --model-path <path> [--ngl <layers>]");
    eprintln!(
        "  calliop-llm-worker --oneshot --model-path <path> [--ngl <layers>] \"text to clean\""
    );
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

    let n_gpu_layers = parse_n_gpu_layers(&args);

    let result = if args.iter().any(|arg| arg == "--serve") {
        serve(model_path, n_gpu_layers)
    } else if args.iter().any(|arg| arg == "--oneshot") {
        match parse_oneshot_text(&args) {
            Ok(text) => oneshot(model_path, n_gpu_layers, text),
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
