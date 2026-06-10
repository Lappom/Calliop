use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use thiserror::Error;

pub const TARGET_SAMPLE_RATE: u32 = 16_000;

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
}

enum AudioCommand {
    Start(std::sync::mpsc::Sender<Result<(), AudioError>>),
    Stop(std::sync::mpsc::Sender<Result<Vec<f32>, AudioError>>),
    Shutdown,
}

struct AudioSession {
    _stream: cpal::Stream,
    source_sample_rate: u32,
    source_channels: u16,
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
        if self.is_recording() {
            return Err(AudioError::AlreadyRecording);
        }

        let (reply_tx, reply_rx) = std::sync::mpsc::channel();
        self.command_tx
            .send(AudioCommand::Start(reply_tx))
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
            AudioCommand::Start(reply_tx) => {
                let result = start_session(&mut session, &buffer);
                let _ = reply_tx.send(result);
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
) -> Result<(), AudioError> {
    if session.is_some() {
        return Err(AudioError::AlreadyRecording);
    }

    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or(AudioError::NoInputDevice)?;

    let config = device
        .default_input_config()
        .map_err(|e| AudioError::DefaultConfig(e.to_string()))?;

    let source_sample_rate = config.sample_rate().0;
    let source_channels = config.channels();

    {
        let mut guard = buffer.lock().expect("audio buffer lock poisoned");
        guard.clear();
    }

    let stream_buffer = Arc::clone(buffer);
    let err_fn = |err| eprintln!("audio stream error: {err}");

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => {
            let stream_config: cpal::StreamConfig = config.clone().into();
            device
                .build_input_stream(
                    &stream_config,
                    move |data: &[f32], _| append_samples(&stream_buffer, data),
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
                        let samples: Vec<f32> =
                            data.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
                        append_samples(&stream_buffer, &samples);
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
                        let samples: Vec<f32> = data
                            .iter()
                            .map(|&s| (s as f32 / u16::MAX as f32) * 2.0 - 1.0)
                            .collect();
                        append_samples(&stream_buffer, &samples);
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
    });
    Ok(())
}

fn stop_session(
    session: &mut Option<AudioSession>,
    buffer: &Arc<Mutex<Vec<f32>>>,
) -> Result<Vec<f32>, AudioError> {
    let active = session.take().ok_or(AudioError::NotRecording)?;
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

/// Downmixes to mono and resamples to 16 kHz using linear interpolation.
pub fn resample_to_16k_mono(input: &[f32], source_rate: u32, channels: u16) -> Vec<f32> {
    if input.is_empty() {
        return Vec::new();
    }

    let mono: Vec<f32> = if channels <= 1 {
        input.to_vec()
    } else {
        let ch = channels as usize;
        input
            .chunks(ch)
            .map(|frame| frame.iter().sum::<f32>() / ch as f32)
            .collect()
    };

    if source_rate == TARGET_SAMPLE_RATE {
        return mono;
    }

    let ratio = source_rate as f64 / TARGET_SAMPLE_RATE as f64;
    let output_len = ((mono.len() as f64) / ratio).ceil() as usize;
    let mut output = Vec::with_capacity(output_len);

    for i in 0..output_len {
        let src_pos = i as f64 * ratio;
        let idx = src_pos.floor() as usize;
        let frac = (src_pos - idx as f64) as f32;

        let sample = if idx + 1 < mono.len() {
            mono[idx] * (1.0 - frac) + mono[idx + 1] * frac
        } else {
            mono[idx.min(mono.len() - 1)]
        };
        output.push(sample);
    }

    output
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
}
