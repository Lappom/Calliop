use std::env;

use calliop_prompt::ToneProfile;

use calliop_lib::inference;
use calliop_lib::llm::{ensure_llm_model_blocking, LlamaEngine, LlmModel};
use calliop_lib::store::InferenceBackend;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: test-llm \"text to clean\"");
        eprintln!("Example: cargo run --bin test-llm -- \"euh bonjour donc voilà\"");
        std::process::exit(1);
    }

    let raw = args[1..].join(" ");
    println!("Ensuring LLM model (download on first run)...");
    let model = LlmModel::default();
    let model_path = ensure_llm_model_blocking(None, model)?;

    println!("Starting LLM worker...");
    let mut engine =
        LlamaEngine::start_with_config(&model_path, inference::gpu_layers(InferenceBackend::Auto))?;
    println!("Cleaning: {raw}");
    let cleaned = engine.cleanup(&raw, ToneProfile::Default)?;
    println!("Cleaned: {cleaned}");
    Ok(())
}
