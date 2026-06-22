use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, StreamTrait};
use thiserror::Error;

pub const TARGET_SAMPLE_RATE: u32 = 16_000;
pub const AUDIO_BAND_COUNT: usize = 14;
/// Max spooled 16 kHz samples while the chunk channel is backlogged (~10 s).
const CHUNK_SPOOL_MAX_SAMPLES: usize = TARGET_SAMPLE_RATE as usize * 10;

pub type AudioChunkSender = std::sync::mpsc::SyncSender<Vec<f32>>;
pub type AudioLevelSender = std::sync::mpsc::SyncSender<AudioLevelSample>;

#[derive(Debug, Clone, Copy)]
pub struct AudioLevelSample {
    pub level: f32,
    pub bands: [f32; AUDIO_BAND_COUNT],
}

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("no input device available")]
    NoInputDevice,
    #[error("failed to get default input config: {0}")]
    DefaultConfig(String),
    #[error("failed to build input stream: {0}")]
    StreamBuild(String),
    #[error("failed to play input stream: {0}")]
    StreamPlay(String),
    #[error("capture is already running")]
    AlreadyRecording,
    #[error("capture is not running")]
    NotRecording,
    #[error("audio thread unavailable")]
    ThreadUnavailable,
    #[error("audio thread failed: {0}")]
    ThreadFailed(String),
    #[error("failed to enumerate audio devices: {0}")]
    DeviceEnumerate(String),
}

enum AudioCommand {
    Start {
        reply: std::sync::mpsc::Sender<Result<(), AudioError>>,
        chunk_tx: Option<AudioChunkSender>,
        level_tx: Option<AudioLevelSender>,
        device_id: String,
    },
    Stop(std::sync::mpsc::Sender<Result<Vec<f32>, AudioError>>),
    Shutdown,
}

struct ResampleStreamState {
    mono: Vec<f32>,
    pending_frame: Vec<f32>,
    emitted_output: usize,
    source_rate: u32,
    channels: u16,
}

impl ResampleStreamState {
    fn new(source_rate: u32, channels: u16) -> Self {
        Self {
            mono: Vec::new(),
            pending_frame: Vec::new(),
            emitted_output: 0,
            source_rate,
            channels,
        }
    }

    fn push_chunk(&mut self, chunk: &[f32]) {
        append_interleaved(
            &mut self.mono,
            &mut self.pending_frame,
            chunk,
            self.channels,
        );
    }

    fn drain_delta(&mut self) -> Vec<f32> {
        let delta = drain_resampled(&self.mono, self.source_rate, self.emitted_output, false);
        self.emitted_output += delta.len();
        delta
    }

    fn drain_remainder(&mut self) -> Vec<f32> {
        let delta = drain_resampled(&self.mono, self.source_rate, self.emitted_output, true);
        self.emitted_output += delta.len();
        delta
    }
}

fn trim_chunk_spool(spool: &mut Vec<f32>) {
    if spool.len() <= CHUNK_SPOOL_MAX_SAMPLES {
        return;
    }
    let excess = spool.len() - CHUNK_SPOOL_MAX_SAMPLES;
    spool.drain(..excess);
}

struct StreamingSidecar {
    resample_state: Mutex<ResampleStreamState>,
    accumulated_16k: Mutex<Vec<f32>>,
    chunk_tx: Option<AudioChunkSender>,
    level_tx: Option<AudioLevelSender>,
    last_level_emit: Mutex<Instant>,
    convert_scratch: Mutex<Vec<f32>>,
    /// Audio deltas that could not be sent because the chunk channel was full.
    chunk_spool: Mutex<Vec<f32>>,
    /// When true, stop returns the accumulated 16 kHz stream instead of resampling raw PCM.
    streaming_capture: bool,
}

impl StreamingSidecar {
    fn push_f32(&self, chunk: &[f32], raw_buffer: Option<&Arc<Mutex<Vec<f32>>>>) {
        if let Some(buffer) = raw_buffer {
            append_samples(buffer, chunk);
        }
        self.process_resampled(chunk);
    }

    fn push_i16(&self, data: &[i16], raw_buffer: Option<&Arc<Mutex<Vec<f32>>>>) {
        let mut scratch = self.convert_scratch.lock().expect("convert scratch lock");
        scratch.clear();
        scratch.reserve(data.len());
        for &sample in data {
            scratch.push(sample as f32 / i16::MAX as f32);
        }
        let converted = scratch.as_slice();
        if let Some(buffer) = raw_buffer {
            append_samples(buffer, converted);
        }
        self.process_resampled(converted);
    }

    fn push_u16(&self, data: &[u16], raw_buffer: Option<&Arc<Mutex<Vec<f32>>>>) {
        let mut scratch = self.convert_scratch.lock().expect("convert scratch lock");
        scratch.clear();
        scratch.reserve(data.len());
        for &sample in data {
            scratch.push((sample as f32 / u16::MAX as f32) * 2.0 - 1.0);
        }
        let converted = scratch.as_slice();
        if let Some(buffer) = raw_buffer {
            append_samples(buffer, converted);
        }
        self.process_resampled(converted);
    }

    fn process_resampled(&self, chunk: &[f32]) {
        if self.chunk_tx.is_none() && self.level_tx.is_none() {
            return;
        }

        let mut state = self.resample_state.lock().expect("resample lock");
        state.push_chunk(chunk);
        let delta = state.drain_delta();
        drop(state);

        if !delta.is_empty() {
            self.emit_delta(delta);
        }
    }

    fn emit_delta(&self, delta: Vec<f32>) {
        if self.streaming_capture {
            self.accumulated_16k
                .lock()
                .expect("accumulated lock")
                .extend_from_slice(&delta);
        }

        if let Some(level_tx) = &self.level_tx {
            let mut last = self.last_level_emit.lock().expect("level lock");
            if last.elapsed() >= Duration::from_millis(50) {
                if level_tx.try_send(compute_audio_level(&delta)).is_ok() {
                    *last = Instant::now();
                }
            }
        }

        self.enqueue_chunk(delta);
    }

    fn enqueue_chunk(&self, delta: Vec<f32>) {
        if delta.is_empty() {
            return;
        }
        let Some(tx) = &self.chunk_tx else {
            return;
        };

        let mut spool = self.chunk_spool.lock().expect("chunk spool lock");
        spool.extend_from_slice(&delta);
        trim_chunk_spool(&mut spool);
        match tx.try_send(std::mem::take(&mut *spool)) {
            Ok(()) => {}
            Err(std::sync::mpsc::TrySendError::Full(payload)) => *spool = payload,
            Err(std::sync::mpsc::TrySendError::Disconnected(payload)) => *spool = payload,
        }
    }

    fn drain_chunk_spool(&self) {
        let Some(tx) = &self.chunk_tx else {
            return;
        };
        let mut spool = self.chunk_spool.lock().expect("chunk spool lock");
        if spool.is_empty() {
            return;
        }
        let payload = std::mem::take(&mut *spool);
        drop(spool);
        let _ = tx.send(payload);
    }

    fn flush(&self) {
        if self.chunk_tx.is_none() {
            return;
        }

        let mut state = self.resample_state.lock().expect("resample lock");
        let delta = state.drain_remainder();
        drop(state);
        if !delta.is_empty() {
            self.emit_delta(delta);
        }
        self.drain_chunk_spool();
    }

    fn take_accumulated(&self) -> Vec<f32> {
        std::mem::take(&mut *self.accumulated_16k.lock().expect("accumulated lock"))
    }

    fn uses_streaming_accumulator(&self) -> bool {
        self.streaming_capture
    }
}

struct AudioSession {
    _stream: cpal::Stream,
    source_sample_rate: u32,
    source_channels: u16,
    sidecar: Arc<StreamingSidecar>,
}

/// Thread-safe handle to a dedicated audio capture thread (cpal streams are !Send).
pub struct AudioCapture {
    command_tx: std::sync::mpsc::Sender<AudioCommand>,
    recording: AtomicBool,
    thread: Option<JoinHandle<()>>,
}

impl AudioCapture {
    pub fn new() -> Result<Self, AudioError> {
        let (command_tx, command_rx) = std::sync::mpsc::channel();
        let buffer = Arc::new(Mutex::new(Vec::new()));

        let thread_buffer = Arc::clone(&buffer);
        let thread = thread::spawn(move || audio_thread_main(command_rx, thread_buffer));

        Ok(Self {
            command_tx,
            recording: AtomicBool::new(false),
            thread: Some(thread),
        })
    }

    pub fn is_recording(&self) -> bool {
        self.recording.load(Ordering::SeqCst)
    }

    pub fn start(&mut self) -> Result<(), AudioError> {
        self.start_with_streaming(None, None, None)
    }

    pub fn start_with_streaming(
        &mut self,
        chunk_tx: Option<AudioChunkSender>,
        level_tx: Option<AudioLevelSender>,
        device_id: Option<&str>,
    ) -> Result<(), AudioError> {
        if self.is_recording() {
            return Err(AudioError::AlreadyRecording);
        }

        let device_id = device_id
            .filter(|id| !id.is_empty())
            .unwrap_or(crate::audio::devices::DEFAULT_INPUT_DEVICE_ID)
            .to_string();

        let (reply_tx, reply_rx) = std::sync::mpsc::channel();
        self.command_tx
            .send(AudioCommand::Start {
                reply: reply_tx,
                chunk_tx,
                level_tx,
                device_id,
            })
            .map_err(|_| AudioError::ThreadUnavailable)?;
        reply_rx
            .recv()
            .map_err(|_| AudioError::ThreadUnavailable)??;
        self.recording.store(true, Ordering::SeqCst);
        Ok(())
    }

    pub fn stop(&mut self) -> Result<Vec<f32>, AudioError> {
        if !self.is_recording() {
            return Err(AudioError::NotRecording);
        }

        let (reply_tx, reply_rx) = std::sync::mpsc::channel();
        self.command_tx
            .send(AudioCommand::Stop(reply_tx))
            .map_err(|_| AudioError::ThreadUnavailable)?;
        let samples = reply_rx
            .recv()
            .map_err(|_| AudioError::ThreadUnavailable)??;
        self.recording.store(false, Ordering::SeqCst);
        Ok(samples)
    }
}

impl Drop for AudioCapture {
    fn drop(&mut self) {
        let _ = self.command_tx.send(AudioCommand::Shutdown);
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}

fn audio_thread_main(
    command_rx: std::sync::mpsc::Receiver<AudioCommand>,
    buffer: Arc<Mutex<Vec<f32>>>,
) {
    let mut session: Option<AudioSession> = None;

    for command in command_rx {
        match command {
            AudioCommand::Start {
                reply,
                chunk_tx,
                level_tx,
                device_id,
            } => {
                let result = start_session(&mut session, &buffer, chunk_tx, level_tx, &device_id);
                let _ = reply.send(result);
            }
            AudioCommand::Stop(reply_tx) => {
                let result = stop_session(&mut session, &buffer);
                let _ = reply_tx.send(result);
            }
            AudioCommand::Shutdown => break,
        }
    }
}

fn start_session(
    session: &mut Option<AudioSession>,
    buffer: &Arc<Mutex<Vec<f32>>>,
    chunk_tx: Option<AudioChunkSender>,
    level_tx: Option<AudioLevelSender>,
    device_id: &str,
) -> Result<(), AudioError> {
    if session.is_some() {
        return Err(AudioError::AlreadyRecording);
    }

    let host = cpal::default_host();
    let device = crate::audio::devices::resolve_input_device(&host, device_id)?;

    let config = device
        .default_input_config()
        .map_err(|e| AudioError::DefaultConfig(e.to_string()))?;

    let source_sample_rate = config.sample_rate().0;
    let source_channels = config.channels();
    let streaming_capture = chunk_tx.is_some();
    let retain_raw_buffer = !streaming_capture;

    {
        let mut guard = buffer.lock().expect("audio buffer lock poisoned");
        guard.clear();
    }

    let sidecar = Arc::new(StreamingSidecar {
        resample_state: Mutex::new(ResampleStreamState::new(
            source_sample_rate,
            source_channels,
        )),
        accumulated_16k: Mutex::new(Vec::new()),
        chunk_tx,
        level_tx,
        last_level_emit: Mutex::new(Instant::now() - Duration::from_millis(100)),
        convert_scratch: Mutex::new(Vec::new()),
        chunk_spool: Mutex::new(Vec::new()),
        streaming_capture,
    });

    let stream_buffer = if retain_raw_buffer {
        Some(Arc::clone(buffer))
    } else {
        None
    };
    let stream_sidecar = Arc::clone(&sidecar);
    let err_fn = |err| eprintln!("audio stream error: {err}");

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => {
            let stream_config: cpal::StreamConfig = config.clone().into();
            device
                .build_input_stream(
                    &stream_config,
                    move |data: &[f32], _| {
                        stream_sidecar.push_f32(data, stream_buffer.as_ref());
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| AudioError::StreamBuild(e.to_string()))?
        }
        cpal::SampleFormat::I16 => {
            let stream_config: cpal::StreamConfig = config.clone().into();
            device
                .build_input_stream(
                    &stream_config,
                    move |data: &[i16], _| {
                        stream_sidecar.push_i16(data, stream_buffer.as_ref());
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| AudioError::StreamBuild(e.to_string()))?
        }
        cpal::SampleFormat::U16 => {
            let stream_config: cpal::StreamConfig = config.clone().into();
            device
                .build_input_stream(
                    &stream_config,
                    move |data: &[u16], _| {
                        stream_sidecar.push_u16(data, stream_buffer.as_ref());
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| AudioError::StreamBuild(e.to_string()))?
        }
        other => {
            return Err(AudioError::StreamBuild(format!(
                "unsupported sample format: {other:?}"
            )));
        }
    };

    stream
        .play()
        .map_err(|e| AudioError::StreamPlay(e.to_string()))?;

    *session = Some(AudioSession {
        _stream: stream,
        source_sample_rate,
        source_channels,
        sidecar,
    });
    Ok(())
}

fn stop_session(
    session: &mut Option<AudioSession>,
    buffer: &Arc<Mutex<Vec<f32>>>,
) -> Result<Vec<f32>, AudioError> {
    let active = session.take().ok_or(AudioError::NotRecording)?;
    active.sidecar.flush();

    if active.sidecar.uses_streaming_accumulator() {
        return Ok(active.sidecar.take_accumulated());
    }

    let raw = buffer.lock().expect("audio buffer lock poisoned").clone();
    Ok(resample_to_16k_mono(
        &raw,
        active.source_sample_rate,
        active.source_channels,
    ))
}

fn append_samples(buffer: &Arc<Mutex<Vec<f32>>>, data: &[f32]) {
    let mut guard = buffer.lock().expect("audio buffer lock poisoned");
    guard.extend_from_slice(data);
}

fn append_interleaved(
    mono: &mut Vec<f32>,
    pending_frame: &mut Vec<f32>,
    chunk: &[f32],
    channels: u16,
) {
    let ch = channels as usize;
    if ch <= 1 {
        mono.extend_from_slice(chunk);
        return;
    }

    let mut idx = 0;
    if !pending_frame.is_empty() {
        while idx < chunk.len() && pending_frame.len() < ch {
            pending_frame.push(chunk[idx]);
            idx += 1;
        }
        if pending_frame.len() == ch {
            mono.push(pending_frame.iter().sum::<f32>() / ch as f32);
            pending_frame.clear();
        }
    }

    let remaining = &chunk[idx..];
    let full_frames = remaining.len() / ch;
    for frame in remaining[..full_frames * ch].chunks(ch) {
        mono.push(frame.iter().sum::<f32>() / ch as f32);
    }
    pending_frame.extend_from_slice(&remaining[full_frames * ch..]);
}

fn resample_ratio(source_rate: u32) -> f64 {
    source_rate as f64 / TARGET_SAMPLE_RATE as f64
}

fn resampled_output_len(mono_len: usize, source_rate: u32, include_partial: bool) -> usize {
    if mono_len == 0 {
        return 0;
    }
    if source_rate == TARGET_SAMPLE_RATE {
        return mono_len;
    }
    let ratio = resample_ratio(source_rate);
    let len = mono_len as f64 / ratio;
    if include_partial {
        len.ceil() as usize
    } else {
        len.floor() as usize
    }
}

fn resampled_sample_at(output_idx: usize, mono: &[f32], ratio: f64) -> f32 {
    let src_pos = output_idx as f64 * ratio;
    let idx = src_pos.floor() as usize;
    let frac = (src_pos - idx as f64) as f32;

    if idx + 1 < mono.len() {
        mono[idx] * (1.0 - frac) + mono[idx + 1] * frac
    } else {
        mono[idx.min(mono.len() - 1)]
    }
}

fn drain_resampled(
    mono: &[f32],
    source_rate: u32,
    emitted_output: usize,
    include_partial: bool,
) -> Vec<f32> {
    if mono.is_empty() {
        return Vec::new();
    }
    if source_rate == TARGET_SAMPLE_RATE {
        if emitted_output >= mono.len() {
            return Vec::new();
        }
        return mono[emitted_output..].to_vec();
    }

    let ratio = resample_ratio(source_rate);
    let available = resampled_output_len(mono.len(), source_rate, include_partial);
    if emitted_output >= available {
        return Vec::new();
    }

    (emitted_output..available)
        .map(|i| resampled_sample_at(i, mono, ratio))
        .collect()
}

pub fn compute_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f32 = samples.iter().map(|s| s * s).sum();
    (sum / samples.len() as f32).sqrt()
}

pub fn compute_audio_level(samples: &[f32]) -> AudioLevelSample {
    let level = compute_rms(samples);
    let bands = compute_band_levels(samples, AUDIO_BAND_COUNT);
    AudioLevelSample { level, bands }
}

/// RMS per temporal band with triangular overlap — voice-correlated variation without FFT.
pub fn compute_band_levels(samples: &[f32], bands: usize) -> [f32; AUDIO_BAND_COUNT] {
    let mut out = [0.0f32; AUDIO_BAND_COUNT];
    if samples.is_empty() || bands == 0 {
        return out;
    }

    let band_count = bands.min(AUDIO_BAND_COUNT);
    let n = samples.len() as f32;
    let global_rms = compute_rms(samples);
    if global_rms < f32::EPSILON {
        return out;
    }

    let half_width = (n / band_count as f32) * 1.15;

    for (band, slot) in out.iter_mut().take(band_count).enumerate() {
        let center = (band as f32 + 0.5) / band_count as f32 * n;
        let mut weighted_sum_sq = 0.0f32;
        let mut weight_sum = 0.0f32;

        for (i, &sample) in samples.iter().enumerate() {
            let dist = (i as f32 - center).abs();
            let weight = (1.0 - dist / half_width).max(0.0);
            if weight > 0.0 {
                weighted_sum_sq += sample * sample * weight;
                weight_sum += weight;
            }
        }

        let band_rms = if weight_sum > 0.0 {
            (weighted_sum_sq / weight_sum).sqrt()
        } else {
            0.0
        };
        *slot = (band_rms / global_rms).min(1.0);
    }

    out
}

/// Downmixes to mono and resamples to 16 kHz using linear interpolation.
pub fn resample_to_16k_mono(input: &[f32], source_rate: u32, channels: u16) -> Vec<f32> {
    if input.is_empty() {
        return Vec::new();
    }

    let mut mono = Vec::new();
    let mut pending_frame = Vec::new();
    append_interleaved(&mut mono, &mut pending_frame, input, channels);

    if source_rate == TARGET_SAMPLE_RATE {
        return mono;
    }

    let ratio = resample_ratio(source_rate);
    let output_len = resampled_output_len(mono.len(), source_rate, true);
    (0..output_len)
        .map(|i| resampled_sample_at(i, &mono, ratio))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resample_preserves_16k_mono() {
        let input: Vec<f32> = (0..160).map(|i| i as f32 / 160.0).collect();
        let output = resample_to_16k_mono(&input, 16_000, 1);
        assert_eq!(output.len(), input.len());
        assert!((output[10] - input[10]).abs() < f32::EPSILON);
    }

    #[test]
    fn resample_downmixes_stereo() {
        let input = vec![1.0, 0.0, 0.5, 0.5];
        let output = resample_to_16k_mono(&input, 16_000, 2);
        assert_eq!(output, vec![0.5, 0.5]);
    }

    #[test]
    fn resample_halves_sample_rate() {
        let input: Vec<f32> = (0..8).map(|i| i as f32).collect();
        let output = resample_to_16k_mono(&input, 32_000, 1);
        assert_eq!(output.len(), 4);
        assert!((output[0] - 0.0).abs() < f32::EPSILON);
        assert!((output[1] - 2.0).abs() < 0.01);
    }

    #[test]
    fn compute_rms_of_silence_is_zero() {
        assert!(compute_rms(&[0.0, 0.0, 0.0]) < f32::EPSILON);
    }

    #[test]
    fn compute_rms_of_unit_signal() {
        let rms = compute_rms(&[1.0, -1.0, 1.0, -1.0]);
        assert!((rms - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn compute_band_levels_silence_is_zero() {
        let bands = compute_band_levels(&[0.0; 140], AUDIO_BAND_COUNT);
        assert!(bands.iter().all(|b| *b < f32::EPSILON));
    }

    #[test]
    fn compute_band_levels_constant_signal_is_uniform() {
        let samples = vec![0.5f32; 280];
        let bands = compute_band_levels(&samples, AUDIO_BAND_COUNT);
        for band in &bands {
            assert!((*band - 1.0).abs() < 0.05, "band={band}");
        }
    }

    #[test]
    fn compute_band_levels_localized_impulse_concentrates_energy() {
        let mut samples = vec![0.0f32; 280];
        let center = 140usize;
        for i in center.saturating_sub(4)..=(center + 4).min(samples.len() - 1) {
            samples[i] = 1.0;
        }

        let bands = compute_band_levels(&samples, AUDIO_BAND_COUNT);
        let peak_band = bands
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(i, _)| i)
            .unwrap();
        let mid_band = AUDIO_BAND_COUNT / 2;
        assert!(
            (peak_band as i32 - mid_band as i32).abs() <= 3,
            "peak_band={peak_band}, bands={bands:?}"
        );
        assert!(bands[peak_band] > 0.5);
    }

    #[test]
    fn incremental_stream_matches_batch_resample() {
        let input: Vec<f32> = (0..9600)
            .map(|i| ((i as f32) * 0.01).sin())
            .flat_map(|sample| [sample, sample * 0.5])
            .collect();
        let batch = resample_to_16k_mono(&input, 48_000, 2);

        let mut mono = Vec::new();
        let mut pending = Vec::new();
        let mut emitted = 0usize;
        let mut streamed = Vec::new();

        for chunk in input.chunks(512) {
            append_interleaved(&mut mono, &mut pending, chunk, 2);
            let delta = drain_resampled(&mono, 48_000, emitted, false);
            emitted += delta.len();
            streamed.extend(delta);
        }
        streamed.extend(drain_resampled(&mono, 48_000, emitted, true));

        assert_eq!(streamed.len(), batch.len());
        for (streamed_sample, batch_sample) in streamed.iter().zip(batch.iter()) {
            assert!(
                (streamed_sample - batch_sample).abs() < 0.001,
                "streamed={streamed_sample} batch={batch_sample}"
            );
        }
    }

    #[test]
    fn streaming_sidecar_accumulates_16k_deltas() {
        let (chunk_tx, _chunk_rx) = std::sync::mpsc::sync_channel::<Vec<f32>>(64);
        let sidecar = StreamingSidecar {
            resample_state: Mutex::new(ResampleStreamState::new(16_000, 1)),
            accumulated_16k: Mutex::new(Vec::new()),
            chunk_tx: Some(chunk_tx),
            level_tx: None,
            last_level_emit: Mutex::new(Instant::now()),
            convert_scratch: Mutex::new(Vec::new()),
            chunk_spool: Mutex::new(Vec::new()),
            streaming_capture: true,
        };

        let samples: Vec<f32> = (0..320).map(|i| i as f32 / 320.0).collect();
        sidecar.push_f32(&samples, None);
        sidecar.flush();

        let accumulated = sidecar.take_accumulated();
        assert_eq!(accumulated.len(), samples.len());
        assert!((accumulated[10] - samples[10]).abs() < f32::EPSILON);
    }

    #[test]
    fn streaming_sidecar_spools_when_chunk_channel_full() {
        let (chunk_tx, chunk_rx) = std::sync::mpsc::sync_channel::<Vec<f32>>(1);
        chunk_tx.try_send(vec![0.0; 8]).expect("prefill channel");

        let sidecar = StreamingSidecar {
            resample_state: Mutex::new(ResampleStreamState::new(16_000, 1)),
            accumulated_16k: Mutex::new(Vec::new()),
            chunk_tx: Some(chunk_tx),
            level_tx: None,
            last_level_emit: Mutex::new(Instant::now()),
            convert_scratch: Mutex::new(Vec::new()),
            chunk_spool: Mutex::new(Vec::new()),
            streaming_capture: true,
        };

        let samples: Vec<f32> = (0..160).map(|i| i as f32 / 160.0).collect();
        sidecar.push_f32(&samples, None);

        assert!(
            !sidecar.chunk_spool.lock().expect("spool lock").is_empty(),
            "expected spool to retain unsent audio"
        );

        let _ = chunk_rx.recv().expect("prefilled chunk");
        sidecar.drain_chunk_spool();

        let spooled = chunk_rx.recv().expect("spooled chunk");
        assert_eq!(spooled.len(), samples.len());
        assert!((spooled[10] - samples[10]).abs() < f32::EPSILON);
    }

    #[test]
    fn chunk_spool_trim_drops_oldest_samples() {
        let mut spool: Vec<f32> = (0..CHUNK_SPOOL_MAX_SAMPLES + 500)
            .map(|i| i as f32)
            .collect();
        trim_chunk_spool(&mut spool);
        assert_eq!(spool.len(), CHUNK_SPOOL_MAX_SAMPLES);
        assert!((spool[0] - 500.0).abs() < f32::EPSILON);
    }
}
