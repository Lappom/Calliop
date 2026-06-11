//! Background observer that detects user corrections in the target app field.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use tauri::AppHandle;

use super::read_focused_text;
use super::region::{build_anchors, extract_region, is_stabilized, MAX_REGION_FACTOR};

const BASELINE_DELAY_MS: u64 = 300;
const POLL_INTERVAL_MS: u64 = 2000;
const MAX_OBSERVE_MS: u64 = 60_000;

pub type CorrectionHandler = Arc<dyn Fn(&AppHandle, &str, &str) + Send + Sync>;

pub fn spawn_correction_watcher(
    app: AppHandle,
    injected_text: String,
    generation: Arc<AtomicU64>,
    watch_generation: u64,
    handler: CorrectionHandler,
) {
    thread::spawn(move || {
        if let Err(err) = run_correction_watcher(
            &app,
            &injected_text,
            &generation,
            watch_generation,
            &handler,
        ) {
            eprintln!("correction watcher: {err}");
        }
    });
}

fn run_correction_watcher(
    app: &AppHandle,
    injected_text: &str,
    generation: &Arc<AtomicU64>,
    watch_generation: u64,
    handler: &CorrectionHandler,
) -> Result<(), String> {
    thread::sleep(Duration::from_millis(BASELINE_DELAY_MS));

    if generation.load(Ordering::SeqCst) != watch_generation {
        return Ok(());
    }

    let Some(baseline) = read_focused_text() else {
        return Ok(());
    };

    if !baseline.contains(injected_text.trim()) {
        let normalized_baseline = baseline.split_whitespace().collect::<Vec<_>>().join(" ");
        let normalized_injected = injected_text
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if !normalized_baseline.contains(&normalized_injected) {
            return Ok(());
        }
    }

    let anchors = build_anchors(&baseline, injected_text)
        .ok_or_else(|| "could not locate injected text in baseline".to_string())?;

    let initial_region =
        extract_region(&baseline, &anchors, injected_text.len() * MAX_REGION_FACTOR)
            .unwrap_or_else(|| injected_text.to_string());

    let mut stable_reads = 0u32;
    let mut last_region = initial_region;
    let deadline = std::time::Instant::now() + Duration::from_millis(MAX_OBSERVE_MS);

    while std::time::Instant::now() < deadline {
        if generation.load(Ordering::SeqCst) != watch_generation {
            return Ok(());
        }

        thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));

        if generation.load(Ordering::SeqCst) != watch_generation {
            return Ok(());
        }

        let Some(current_doc) = read_focused_text() else {
            stable_reads = 0;
            continue;
        };

        let Some(current_region) = extract_region(
            &current_doc,
            &anchors,
            injected_text.len() * MAX_REGION_FACTOR,
        ) else {
            stable_reads = 0;
            continue;
        };

        if current_region == injected_text.trim() {
            stable_reads = 0;
            last_region = current_region;
            continue;
        }

        if current_region == last_region {
            stable_reads += 1;
        } else {
            stable_reads = 1;
            last_region = current_region.clone();
        }

        if is_stabilized(&last_region, &current_region, stable_reads) {
            handler(app, injected_text, &current_region);
            return Ok(());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::region::{build_anchors, extract_region};

    #[test]
    fn watcher_region_flow_detects_word_fix() {
        let injected = "Calliop";
        let baseline = "Hello Calliop world";
        let corrected_doc = "Hello Calliope world";

        let anchors = build_anchors(baseline, injected).expect("anchors");
        let max_len = injected.len() * super::MAX_REGION_FACTOR;
        let region = extract_region(corrected_doc, &anchors, max_len).expect("region");
        assert_eq!(region, "Calliope");
    }
}
