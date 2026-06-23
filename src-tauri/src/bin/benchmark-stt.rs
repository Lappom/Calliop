use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::Instant;

use calliop_lib::audio::TARGET_SAMPLE_RATE;
use calliop_lib::store::AutoEditMode;
use calliop_lib::stt::{ensure_model_blocking, word_error_rate, WhisperEngine, WhisperModel};
use calliop_prompt::{
    join_transcript_segments, join_transcript_segments_for_llm_thresholds,
    join_transcript_segments_with_pauses_thresholds, post_process_transcript, PausePreset,
};
use hound::WavReader;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct CorpusManifest {
    version: String,
    language: String,
    model: String,
    samples: Vec<CorpusSample>,
    #[serde(default)]
    pipeline_cases: Vec<PipelineCase>,
}

#[derive(Debug, Deserialize)]
struct CorpusSample {
    id: String,
    reference: String,
    wav: String,
}

#[derive(Debug, Deserialize)]
struct PipelineCase {
    id: String,
    mode: String,
    preset: String,
    segments: Vec<(String, u32)>,
    reference: String,
}

#[derive(Debug, Serialize)]
struct SampleResult {
    id: String,
    reference: String,
    hypothesis: String,
    wer: f64,
    latency_ms: u64,
}

#[derive(Debug, Serialize)]
struct PipelineResult {
    id: String,
    reference: String,
    hypothesis: String,
    match_score: f64,
}

#[derive(Debug, Serialize)]
struct BenchmarkReport {
    version: String,
    corpus_version: String,
    language: String,
    model: String,
    platform: String,
    cpu_only: bool,
    samples: Vec<SampleResult>,
    mean_wer: f64,
    mean_latency_ms: u64,
    generated_at: String,
}

#[derive(Debug, Serialize)]
struct PipelineBenchmarkReport {
    version: String,
    corpus_version: String,
    language: String,
    platform: String,
    cases: Vec<PipelineResult>,
    mean_match_score: f64,
    generated_at: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let manifest_path = args
        .get(1)
        .filter(|arg| !arg.starts_with("--"))
        .cloned()
        .unwrap_or_else(|| "benchmarks/corpus/fr.json".into());
    let pipeline_only = args.iter().any(|arg| arg == "--pipeline");
    let cpu_only = args.iter().any(|arg| arg == "--cpu");

    let manifest = load_manifest(&manifest_path)?;

    if pipeline_only {
        return run_pipeline_benchmark(&manifest, &manifest_path);
    }

    let corpus_dir = Path::new(&manifest_path)
        .parent()
        .ok_or("manifest path has no parent")?;

    println!(
        "Benchmark STT — corpus {} ({}) model {}",
        manifest.version, manifest.language, manifest.model
    );

    let whisper_model = WhisperModel::parse(&manifest.model).unwrap_or(WhisperModel::DistilFrV02);
    println!("Loading Whisper model (download on first run)...");
    let model_path = ensure_model_blocking(None, whisper_model)?;

    let mut engine = WhisperEngine::new_with_gpu(&model_path, !cpu_only)?;
    if cpu_only {
        println!("Using CPU-only inference (--cpu).");
    }

    let mut results = Vec::new();
    for sample in &manifest.samples {
        let wav_path = corpus_dir.join(&sample.wav);
        let samples = read_wav(&wav_path)?;
        let started = Instant::now();
        let transcript = engine.transcribe(&samples, None)?;
        let latency_ms = started.elapsed().as_millis() as u64;
        let wer = word_error_rate(&sample.reference, &transcript.text);

        println!(
            "[{}] WER {:.1}% — {} ms — {}",
            sample.id,
            wer * 100.0,
            latency_ms,
            transcript.text
        );

        results.push(SampleResult {
            id: sample.id.clone(),
            reference: sample.reference.clone(),
            hypothesis: transcript.text,
            wer,
            latency_ms,
        });
    }

    let mean_wer = if results.is_empty() {
        0.0
    } else {
        results.iter().map(|r| r.wer).sum::<f64>() / results.len() as f64
    };
    let mean_latency_ms = if results.is_empty() {
        0
    } else {
        results.iter().map(|r| r.latency_ms).sum::<u64>() / results.len() as u64
    };

    let report = BenchmarkReport {
        version: env!("CARGO_PKG_VERSION").into(),
        corpus_version: manifest.version,
        language: manifest.language,
        model: manifest.model,
        platform: std::env::consts::OS.into(),
        cpu_only,
        samples: results,
        mean_wer,
        mean_latency_ms,
        generated_at: chrono_lite_now(),
    };

    println!(
        "\nSummary: mean WER {:.1}% — mean latency {} ms",
        mean_wer * 100.0,
        mean_latency_ms
    );

    let output = resolve_output_path(&manifest_path, "stt");
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(&report)?;
    std::fs::write(&output, &json)?;
    println!("Wrote {}", output.display());

    Ok(())
}

fn run_pipeline_benchmark(
    manifest: &CorpusManifest,
    manifest_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "Benchmark pipeline quality — corpus {} ({})",
        manifest.version, manifest.language
    );

    let mut results = Vec::new();
    for case in &manifest.pipeline_cases {
        let hypothesis = run_pipeline_case(case);
        let match_score = 1.0 - word_error_rate(&case.reference, &hypothesis);
        println!(
            "[{}] match {:.1}% — expected {:?} — got {:?}",
            case.id,
            match_score * 100.0,
            case.reference,
            hypothesis
        );
        results.push(PipelineResult {
            id: case.id.clone(),
            reference: case.reference.clone(),
            hypothesis,
            match_score,
        });
    }

    let mean_match_score = if results.is_empty() {
        0.0
    } else {
        results.iter().map(|r| r.match_score).sum::<f64>() / results.len() as f64
    };

    let report = PipelineBenchmarkReport {
        version: env!("CARGO_PKG_VERSION").into(),
        corpus_version: manifest.version.clone(),
        language: manifest.language.clone(),
        platform: std::env::consts::OS.into(),
        cases: results,
        mean_match_score,
        generated_at: chrono_lite_now(),
    };

    println!(
        "\nSummary: mean match score {:.1}%",
        mean_match_score * 100.0
    );

    let output = resolve_output_path(manifest_path, "pipeline");
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(&report)?;
    std::fs::write(&output, &json)?;
    println!("Wrote {}", output.display());

    Ok(())
}

fn run_pipeline_case(case: &PipelineCase) -> String {
    let mode = AutoEditMode::parse(&case.mode).unwrap_or(AutoEditMode::Light);
    let preset = PausePreset::parse(&case.preset).unwrap_or_default();
    let thresholds = preset.thresholds();

    let joined = if mode.uses_llm() {
        join_transcript_segments_for_llm_thresholds(&case.segments, &thresholds)
    } else if mode.uses_pause_punctuation() {
        join_transcript_segments_with_pauses_thresholds(&case.segments, &thresholds)
    } else {
        let text_only: Vec<&str> = case
            .segments
            .iter()
            .map(|(text, _)| text.as_str())
            .collect();
        join_transcript_segments(&text_only)
    };

    if mode.uses_deterministic_cleanup() {
        post_process_transcript(joined.trim())
    } else {
        joined.trim().to_string()
    }
}

fn load_manifest(path: &str) -> Result<CorpusManifest, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content)?)
}

fn resolve_output_path(manifest_path: &str, kind: &str) -> PathBuf {
    let version = env!("CARGO_PKG_VERSION");
    if let Ok(custom) = std::env::var("BENCHMARK_OUTPUT") {
        return PathBuf::from(custom);
    }
    let repo_root = find_repo_root(manifest_path);
    repo_root
        .join("benchmarks")
        .join("results")
        .join(format!("v{version}-{kind}.json"))
}

fn find_repo_root(manifest_path: &str) -> PathBuf {
    let mut dir = Path::new(manifest_path)
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    for _ in 0..6 {
        if dir.join("src-tauri").is_dir() {
            return dir;
        }
        if !dir.pop() {
            break;
        }
    }
    PathBuf::from(".")
}

fn read_wav(path: &Path) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let mut reader = WavReader::new(file)?;
    let spec = reader.spec();

    if spec.sample_rate != TARGET_SAMPLE_RATE {
        eprintln!(
            "Warning: expected {} Hz, got {} Hz for {}",
            TARGET_SAMPLE_RATE,
            spec.sample_rate,
            path.display()
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

fn chrono_lite_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("unix:{secs}")
}
