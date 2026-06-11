use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let target = std::env::var("TARGET").expect("TARGET must be set by Cargo");
    println!("cargo:rustc-env=CALLIOP_TARGET_TRIPLE={target}");

    println!("cargo:rerun-if-changed=src/bin/llm-worker.rs");
    println!("cargo:rerun-if-changed=src/llm/prompt.rs");
    println!("cargo:rerun-if-changed=src/llm/mod.rs");

    let bin_name = std::env::var("CARGO_BIN_NAME").unwrap_or_default();
    if bin_name != "calliop-llm-worker" && std::env::var("CALLIOP_SKIP_SIDECAR_BUILD").is_err() {
        build_and_stage_llm_sidecar(&target);
    }

    tauri_build::build();
}

fn build_and_stage_llm_sidecar(target: &str) {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".into());
    let built_worker = manifest_dir
        .join("target")
        .join(&profile)
        .join(format!("calliop-llm-worker{}", std::env::consts::EXE_SUFFIX));

    let status = Command::new("cargo")
        .current_dir(&manifest_dir)
        .env("CALLIOP_SKIP_SIDECAR_BUILD", "1")
        .args([
            "build",
            "--features",
            "llm-worker",
            "--bin",
            "calliop-llm-worker",
        ])
        .status();

    match status {
        Ok(s) if s.success() => {}
        Ok(s) => {
            println!(
                "cargo:warning=LLM sidecar build failed (exit {}); auto-edit may use a stale worker",
                s.code().unwrap_or(-1)
            );
            return;
        }
        Err(err) => {
            println!("cargo:warning=LLM sidecar build failed to start: {err}");
            return;
        }
    }

    if !built_worker.exists() {
        println!("cargo:warning=LLM sidecar binary missing at {}", built_worker.display());
        return;
    }

    let bin_dir = manifest_dir.join("bin");
    if let Err(err) = std::fs::create_dir_all(&bin_dir) {
        println!("cargo:warning=failed to create bin dir: {err}");
        return;
    }

    let staged_name = format!("calliop-llm-worker-{target}{}", std::env::consts::EXE_SUFFIX);
    let staged_path = bin_dir.join(&staged_name);
    if let Err(err) = copy_file(&built_worker, &staged_path) {
        println!("cargo:warning=failed to stage LLM sidecar: {err}");
    }
}

fn copy_file(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::copy(from, to)?;
    Ok(())
}
