use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use calliop_prompt::{post_process_transcript, ToneProfile};

use calliop_lib::inference;
use calliop_lib::llm::{ensure_llm_model_blocking, LlamaEngine, LlmModel};
use calliop_lib::store::InferenceBackend;
use calliop_lib::system::{resolve_llm_model, SystemCapabilities};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!(
            "Usage: test-llm [--raw] [--model qwen3-0.6b|qwen3-1.7b|qwen3.5-4b] \"text to clean\""
        );
        eprintln!("  --raw  skip deterministic post_process (LLM-only, not production path)");
        eprintln!("After calliop-prompt changes, rebuild test-llm (validation runs in-process).");
        eprintln!("Rebuild the sidecar only when worker/LLM prompt logic changes:");
        eprintln!("  ..\\scripts\\build-llm-worker.ps1   (from src-tauri, uses portable CMake)");
        eprintln!("Example: cargo run --bin test-llm -- \"euh bonjour donc voilà\"");
        std::process::exit(1);
    }

    let (model, raw, skip_post_process) = parse_args(&args[1..])?;
    let caps = SystemCapabilities::detect();
    let model = resolve_llm_model(model, &caps);

    warn_if_worker_stale();

    println!(
        "Ensuring LLM model {} (download on first run)...",
        model.as_setting_value()
    );
    let model_path = ensure_llm_model_blocking(None, model)?;

    println!("Starting LLM worker...");
    let mut engine =
        LlamaEngine::start_with_config(&model_path, inference::gpu_layers(InferenceBackend::Auto))?;
    println!("Raw: {raw}");
    let llm_input = if skip_post_process {
        raw.clone()
    } else {
        let processed = post_process_transcript(&raw);
        println!("Post-processed: {processed}");
        processed
    };
    let cleaned = engine.cleanup(&llm_input, ToneProfile::Default)?;
    println!("Cleaned: {cleaned}");
    Ok(())
}

fn parse_args(args: &[String]) -> Result<(LlmModel, String, bool), String> {
    let mut skip_post_process = false;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--raw" => {
                skip_post_process = true;
                index += 1;
            }
            "--model" => {
                let model_id = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value after --model".to_string())?;
                let model = LlmModel::parse(model_id).and_then(|candidate| {
                    if candidate.is_concrete() {
                        Some(candidate)
                    } else {
                        None
                    }
                });
                let model = model.ok_or_else(|| format!("unknown llm model: {model_id}"))?;
                let raw = args[index + 2..].join(" ");
                if raw.trim().is_empty() {
                    return Err("missing text to clean".into());
                }
                return Ok((model, raw, skip_post_process));
            }
            _ => break,
        }
    }

    let raw = args[index..].join(" ");
    if raw.trim().is_empty() {
        return Err("missing text to clean".into());
    }
    Ok((LlmModel::default(), raw, skip_post_process))
}

fn warn_if_worker_stale() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let prompt_lib = manifest_dir.join("crates/calliop-prompt/src/lib.rs");
    let worker = resolve_local_worker_exe(&manifest_dir);

    let Some(worker) = worker else {
        eprintln!(
            "Warning: calliop-llm-worker not found in target/debug. \
             Build it once from src-tauri: cargo build -p calliop-llm-worker"
        );
        return;
    };

    let Ok(prompt_modified) = file_modified_time(&prompt_lib) else {
        return;
    };
    let Ok(worker_modified) = file_modified_time(&worker) else {
        return;
    };

    if worker_modified < prompt_modified {
        eprintln!(
            "Note: {} is older than calliop-prompt (validation still runs in-process). \
             Rebuild only if cleanup prompts/hints changed: cargo build -p calliop-llm-worker",
            worker.display()
        );
    }
}

fn resolve_local_worker_exe(manifest_dir: &Path) -> Option<PathBuf> {
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let target_dir = manifest_dir.join("target").join(profile);
    let triple = env!("CALLIOP_TARGET_TRIPLE");
    let versioned = target_dir.join(format!(
        "calliop-llm-worker-{triple}{}",
        env::consts::EXE_SUFFIX
    ));
    if versioned.exists() {
        return Some(versioned);
    }
    let legacy = target_dir.join(format!("calliop-llm-worker{}", env::consts::EXE_SUFFIX));
    if legacy.exists() {
        return Some(legacy);
    }
    None
}

fn file_modified_time(path: &Path) -> Result<SystemTime, ()> {
    fs::metadata(path)
        .map_err(|_| ())?
        .modified()
        .map_err(|_| ())
}
