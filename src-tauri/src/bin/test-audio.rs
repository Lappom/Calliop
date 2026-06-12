use std::env;
use std::fs::File;
use std::thread;
use std::time::Duration;

use calliop_lib::audio::{resample_to_16k_mono, AudioCapture, VadSegmenter, TARGET_SAMPLE_RATE};
use hound::{WavReader, WavSpec, WavWriter};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }

    match args[1].as_str() {
        "record" => run_record(&args),
        "vad" => run_vad(&args),
        _ => {
            print_usage();
            std::process::exit(1);
        }
    }
}

fn run_record(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() < 4 {
        print_usage();
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

fn run_vad(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() < 3 {
        print_usage();
        std::process::exit(1);
    }

    let input_path = &args[2];
    let mut reader = WavReader::open(input_path)?;
    let spec = reader.spec();

    let raw: Vec<f32> = reader
        .samples::<i16>()
        .map(|sample| sample.map(|s| s as f32 / i16::MAX as f32))
        .collect::<Result<Vec<_>, _>>()?;

    let samples = resample_to_16k_mono(&raw, spec.sample_rate, spec.channels);
    let mut vad = VadSegmenter::new()?;
    let mut segments = Vec::new();

    for chunk in samples.chunks(512) {
        segments.extend(vad.push(chunk)?);
    }
    segments.extend(vad.flush()?);

    println!(
        "Detected {} speech segment(s) in {input_path}",
        segments.len()
    );
    for (index, segment) in segments.iter().enumerate() {
        println!(
            "  segment {index}: {:.2}s ({} samples, leading silence {} ms)",
            segment.samples.len() as f32 / TARGET_SAMPLE_RATE as f32,
            segment.samples.len(),
            segment.leading_silence_ms
        );
    }

    Ok(())
}

fn print_usage() {
    eprintln!("Usage:");
    eprintln!("  test-audio record <duration> <output.wav>");
    eprintln!("  test-audio vad <input.wav>");
    eprintln!("Example: cargo run --bin test-audio -- record 3s output.wav");
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
