use std::env;

use calliop_lib::llm::{ensure_llm_model_blocking, LlamaEngine};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: test-llm \"text to clean\"");
        eprintln!("Example: cargo run --bin test-llm -- \"euh bonjour donc voilà\"");
        std::process::exit(1);
    }

    let raw = args[1..].join(" ");
    println!("Ensuring LLM model (download on first run)...");
    ensure_llm_model_blocking(None)?;

    println!("Starting LLM worker...");
    let mut engine = LlamaEngine::start()?;
    println!("Cleaning: {raw}");
    let cleaned = engine.cleanup(&raw)?;
    println!("Cleaned: {cleaned}");
    Ok(())
}
