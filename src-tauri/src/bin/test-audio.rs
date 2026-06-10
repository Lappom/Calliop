use std::env;
use std::fs::File;
use std::thread;
use std::time::Duration;

use calliop_lib::audio::{AudioCapture, TARGET_SAMPLE_RATE};
use hound::{WavSpec, WavWriter};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 || args[1] != "record" {
        eprintln!("Usage: test-audio record <duration> <output.wav>");
        eprintln!("Example: cargo run --bin test-audio -- record 3s output.wav");
        std::process::exit(1);
    }

    let duration = parse_duration(&args[2])?;
    let output_path = &args[3];

    println!("Recording for {duration:?}...");
    let mut capture = AudioCapture::new()?;
    capture.start()?;
    thread::sleep(duration);
    let samples = capture.stop()?;

    write_wav(output_path, &samples)?;
    println!(
        "Saved {} samples ({:.2}s) to {output_path}",
        samples.len(),
        samples.len() as f32 / TARGET_SAMPLE_RATE as f32
    );
    Ok(())
}

fn parse_duration(raw: &str) -> Result<Duration, String> {
    let trimmed = raw.trim_end_matches('s');
    let seconds: u64 = trimmed
        .parse()
        .map_err(|_| format!("invalid duration: {raw}"))?;
    Ok(Duration::from_secs(seconds))
}

fn write_wav(path: &str, samples: &[f32]) -> Result<(), Box<dyn std::error::Error>> {
    let spec = WavSpec {
        channels: 1,
        sample_rate: TARGET_SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let file = File::create(path)?;
    let mut writer = WavWriter::new(file, spec)?;
    for sample in samples {
        let clamped = sample.clamp(-1.0, 1.0);
        let pcm = (clamped * i16::MAX as f32) as i16;
        writer.write_sample(pcm)?;
    }
    writer.finalize()?;
    Ok(())
}
