use std::env;
use std::fs::File;

use calliop_lib::audio::TARGET_SAMPLE_RATE;
use calliop_lib::stt::{ensure_model_blocking, WhisperEngine};
use hound::WavReader;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: test-stt <input.wav>");
        eprintln!("Example: cargo run --bin test-stt -- output.wav");
        std::process::exit(1);
    }

    let input_path = &args[1];
    println!("Loading Whisper model (download on first run)...");
    let model_path = ensure_model_blocking(None)?;

    let samples = read_wav(input_path)?;
    println!(
        "Transcribing {} samples from {input_path}...",
        samples.len()
    );

    let engine = WhisperEngine::new(&model_path)?;
    let text = engine.transcribe(&samples, None)?;
    println!("Transcription: {text}");
    Ok(())
}

fn read_wav(path: &str) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let mut reader = WavReader::new(file)?;
    let spec = reader.spec();

    if spec.sample_rate != TARGET_SAMPLE_RATE {
        eprintln!(
            "Warning: expected {} Hz, got {} Hz — results may be poor",
            TARGET_SAMPLE_RATE, spec.sample_rate
        );
    }

    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().collect::<Result<Vec<_>, _>>()?,
        hound::SampleFormat::Int => reader
            .samples::<i32>()
            .map(|s| s.map(|v| v as f32 / i16::MAX as f32))
            .collect::<Result<Vec<_>, _>>()?,
    };

    Ok(samples)
}
