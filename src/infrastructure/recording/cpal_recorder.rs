//! Cross-platform audio recorder using cpal.
//!
//! Speech-optimised settings:
//! - 16 kHz sample rate (or resampling from device rate),
//! - mono channel,
//! - FLAC encoding (lossless, accepted by both ChatGPT and OpenAI APIs).
//!
//! The cpal stream is not `Send`, so we always build it inside the worker
//! thread / task that owns it. Cross-thread synchronisation is done with
//! atomics for state plus `tokio::sync::oneshot` for explicit start/stop
//! handshakes (no `sleep(50ms)` timing hacks).

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Instant;

use async_trait::async_trait;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, SampleRate, StreamConfig};
use rubato::{FftFixedIn, Resampler};
use tokio::sync::oneshot;
use tokio::time::{interval, Duration as TokioDuration};

use super::flac_encoder::{encode_to_flac, TARGET_SAMPLE_RATE};
use crate::application::ports::{
    AudioRecorder, ProgressCallback, RecordingError, UnboundedRecorder,
};
use crate::domain::recording::Duration;
use crate::domain::transcription::{AudioData, AudioMimeType};

/// Audio recorder using cpal.
pub struct CpalRecorder {
    /// Recorded samples (mono, i16, at device sample rate).
    audio_buffer: Arc<StdMutex<Vec<i16>>>,
    /// Device sample rate (may differ from the 16 kHz target).
    device_sample_rate: Arc<AtomicU32>,
    /// `true` while a recording session is active.
    is_recording: Arc<AtomicBool>,
    /// Session start (ms since epoch), populated by `UnboundedRecorder::start`.
    start_time_ms: Arc<AtomicU64>,
    /// Elapsed time in milliseconds.
    elapsed_ms: Arc<AtomicU64>,
}

/// Result of opening the cpal stream: the live stream object plus the
/// observed device parameters the caller needs to encode the audio later.
///
/// The stream is not `Send`, so this struct only exists *inside* the thread
/// or `spawn_blocking` task that owns it.
struct StreamHandle {
    stream: cpal::Stream,
    sample_rate: u32,
    #[allow(dead_code)] // available for future diagnostics
    channels: u16,
}

impl CpalRecorder {
    /// Create a new cpal-based recorder.
    pub fn new() -> Self {
        Self {
            audio_buffer: Arc::new(StdMutex::new(Vec::new())),
            device_sample_rate: Arc::new(AtomicU32::new(0)),
            is_recording: Arc::new(AtomicBool::new(false)),
            start_time_ms: Arc::new(AtomicU64::new(0)),
            elapsed_ms: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Get the default input device.
    fn get_input_device() -> Result<cpal::Device, RecordingError> {
        let host = cpal::default_host();
        host.default_input_device()
            .ok_or(RecordingError::NoAudioDevice)
    }

    /// Find a suitable input configuration: prefer mono, prefer configs that
    /// include the 16 kHz target sample rate, only accept I16 or F32.
    fn get_input_config(
        device: &cpal::Device,
    ) -> Result<(StreamConfig, SampleFormat), RecordingError> {
        let supported_configs = device
            .supported_input_configs()
            .map_err(|e| RecordingError::StartFailed(format!("Failed to get configs: {}", e)))?;

        let mut best_config: Option<cpal::SupportedStreamConfigRange> = None;

        for config in supported_configs {
            if config.sample_format() != SampleFormat::I16
                && config.sample_format() != SampleFormat::F32
            {
                continue;
            }

            let includes_target = config.min_sample_rate().0 <= TARGET_SAMPLE_RATE
                && config.max_sample_rate().0 >= TARGET_SAMPLE_RATE;

            let is_better = match &best_config {
                None => true,
                Some(current) => {
                    let fewer_channels = config.channels() < current.channels();
                    let better_rate =
                        includes_target && current.min_sample_rate().0 > TARGET_SAMPLE_RATE;
                    fewer_channels || better_rate
                }
            };
            if is_better {
                best_config = Some(config);
            }
        }

        let config_range = best_config.ok_or(RecordingError::StartFailed(
            "No suitable config found".into(),
        ))?;

        let sample_rate = if config_range.min_sample_rate().0 <= TARGET_SAMPLE_RATE
            && config_range.max_sample_rate().0 >= TARGET_SAMPLE_RATE
        {
            SampleRate(TARGET_SAMPLE_RATE)
        } else {
            config_range.min_sample_rate()
        };

        let sample_format = config_range.sample_format();
        let config = StreamConfig {
            channels: config_range.channels(),
            sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };

        Ok((config, sample_format))
    }

    /// Build, start, and return an input stream that funnels mono i16
    /// samples through `samples_sink`.
    ///
    /// Centralises what used to live in two near-identical match blocks in
    /// `record` and `start`. The sink is invoked from the cpal audio
    /// callback thread and must be cheap.
    fn build_input_stream<F>(samples_sink: F) -> Result<StreamHandle, RecordingError>
    where
        F: Fn(&[i16]) + Send + Sync + 'static,
    {
        let device = Self::get_input_device()?;
        let (config, sample_format) = Self::get_input_config(&device)?;
        let sample_rate = config.sample_rate.0;
        let channels = config.channels;

        // The sink is shared between two branches of the format match. It
        // must be `Fn + Send + Sync + 'static` so both closures can hold a
        // clone. We type-alias here to keep clippy::type_complexity happy.
        type Sink = dyn Fn(&[i16]) + Send + Sync;
        let sink: Arc<Sink> = Arc::new(samples_sink);

        let stream = match sample_format {
            SampleFormat::I16 => {
                let sink = Arc::clone(&sink);
                device
                    .build_input_stream(
                        &config,
                        move |data: &[i16], _: &cpal::InputCallbackInfo| {
                            let mono = stereo_to_mono(data, channels);
                            sink(&mono);
                        },
                        |err| eprintln!("Audio stream error: {}", err),
                        None,
                    )
                    .map_err(|e| RecordingError::StartFailed(e.to_string()))?
            }
            SampleFormat::F32 => {
                let sink = Arc::clone(&sink);
                device
                    .build_input_stream(
                        &config,
                        move |data: &[f32], _: &cpal::InputCallbackInfo| {
                            let i16_data: Vec<i16> =
                                data.iter().map(|&s| (s * 32767.0) as i16).collect();
                            let mono = stereo_to_mono(&i16_data, channels);
                            sink(&mono);
                        },
                        |err| eprintln!("Audio stream error: {}", err),
                        None,
                    )
                    .map_err(|e| RecordingError::StartFailed(e.to_string()))?
            }
            _ => {
                return Err(RecordingError::StartFailed(
                    "Unsupported sample format".into(),
                ))
            }
        };

        stream
            .play()
            .map_err(|e| RecordingError::StartFailed(e.to_string()))?;

        Ok(StreamHandle {
            stream,
            sample_rate,
            channels,
        })
    }

    /// Resample audio from device rate to 16 kHz if needed.
    fn resample_to_16k(samples: &[i16], source_rate: u32) -> Result<Vec<i16>, RecordingError> {
        if source_rate == TARGET_SAMPLE_RATE {
            return Ok(samples.to_vec());
        }

        let samples_f32: Vec<f32> = samples.iter().map(|&s| s as f32 / 32768.0).collect();
        let ratio = TARGET_SAMPLE_RATE as f64 / source_rate as f64;
        let output_len = (samples_f32.len() as f64 * ratio).ceil() as usize;

        let mut resampler = FftFixedIn::<f32>::new(
            source_rate as usize,
            TARGET_SAMPLE_RATE as usize,
            1024,
            2,
            1,
        )
        .map_err(|e| RecordingError::RecordingFailed(format!("Resampler init failed: {}", e)))?;

        let mut output = Vec::with_capacity(output_len);
        let mut input_pos = 0;
        while input_pos < samples_f32.len() {
            let frames_needed = resampler.input_frames_next();
            let end_pos = (input_pos + frames_needed).min(samples_f32.len());
            let chunk: Vec<Vec<f32>> = vec![samples_f32[input_pos..end_pos].to_vec()];
            let chunk = if chunk[0].len() < frames_needed {
                let mut padded = chunk[0].clone();
                padded.resize(frames_needed, 0.0);
                vec![padded]
            } else {
                chunk
            };
            let resampled = resampler.process(&chunk, None).map_err(|e| {
                RecordingError::RecordingFailed(format!("Resampling failed: {}", e))
            })?;
            output.extend(resampled[0].iter().map(|&s| (s * 32767.0) as i16));
            input_pos = end_pos;
        }
        output.truncate(output_len);
        Ok(output)
    }

    /// Encode PCM samples to FLAC (lossless).
    fn encode_audio(samples: &[i16], sample_rate: u32) -> Result<AudioData, RecordingError> {
        let resampled = Self::resample_to_16k(samples, sample_rate)?;
        let flac_data = encode_to_flac(&resampled)
            .map_err(|e| RecordingError::RecordingFailed(format!("FLAC encoding failed: {}", e)))?;
        if flac_data.is_empty() {
            return Err(RecordingError::ReadFailed("Encoded audio is empty".into()));
        }
        Ok(AudioData::new(flac_data, AudioMimeType::Flac))
    }
}

/// Mix multi-channel samples down to mono. Public to expose for tests.
fn stereo_to_mono(samples: &[i16], channels: u16) -> Vec<i16> {
    if channels == 1 {
        return samples.to_vec();
    }
    samples
        .chunks(channels as usize)
        .map(|chunk| {
            let sum: i32 = chunk.iter().map(|&s| s as i32).sum();
            (sum / channels as i32) as i16
        })
        .collect()
}

impl Default for CpalRecorder {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AudioRecorder for CpalRecorder {
    async fn record(
        &self,
        duration: Duration,
        on_progress: Option<ProgressCallback>,
    ) -> Result<AudioData, RecordingError> {
        let duration_ms = duration.as_millis();

        // Clear buffer.
        self.audio_buffer.lock().unwrap().clear();
        self.is_recording.store(true, Ordering::SeqCst);

        let audio_buffer = Arc::clone(&self.audio_buffer);
        let device_sample_rate = Arc::clone(&self.device_sample_rate);
        let is_recording = Arc::clone(&self.is_recording);

        // Run cpal on a blocking task because cpal::Stream is not Send.
        let record_handle = tokio::task::spawn_blocking(move || {
            let audio_buffer_for_sink = Arc::clone(&audio_buffer);
            let is_recording_for_sink = Arc::clone(&is_recording);

            let handle = CpalRecorder::build_input_stream(move |samples: &[i16]| {
                if is_recording_for_sink.load(Ordering::SeqCst) {
                    if let Ok(mut buffer) = audio_buffer_for_sink.lock() {
                        buffer.extend_from_slice(samples);
                    }
                }
            })?;

            device_sample_rate.store(handle.sample_rate, Ordering::SeqCst);

            // Block this thread for the recording duration. We're already
            // inside `spawn_blocking`, so the runtime is not blocked.
            std::thread::sleep(std::time::Duration::from_millis(duration_ms));

            is_recording.store(false, Ordering::SeqCst);
            drop(handle.stream);
            Ok::<u32, RecordingError>(handle.sample_rate)
        });

        // Progress reporting (best-effort, fired from the runtime).
        if let Some(progress) = on_progress {
            let start = Instant::now();
            let progress_clone = Arc::clone(&progress);
            let is_recording = Arc::clone(&self.is_recording);
            tokio::spawn(async move {
                let mut ticker = interval(TokioDuration::from_millis(100));
                while is_recording.load(Ordering::SeqCst) {
                    ticker.tick().await;
                    let elapsed = start.elapsed().as_millis() as u64;
                    if elapsed >= duration_ms {
                        progress_clone(duration_ms, duration_ms);
                        break;
                    }
                    progress_clone(elapsed, duration_ms);
                }
            });
        }

        let sample_rate = record_handle
            .await
            .map_err(|e| RecordingError::RecordingFailed(format!("Task join error: {}", e)))??;

        let samples = std::mem::take(&mut *self.audio_buffer.lock().unwrap());
        if samples.is_empty() {
            return Err(RecordingError::ReadFailed(
                "No audio data captured".to_string(),
            ));
        }

        tokio::task::spawn_blocking(move || Self::encode_audio(&samples, sample_rate))
            .await
            .map_err(|e| RecordingError::RecordingFailed(format!("Encode task error: {}", e)))?
    }
}

#[async_trait]
impl UnboundedRecorder for CpalRecorder {
    async fn start(&self) -> Result<(), RecordingError> {
        if self.is_recording.load(Ordering::SeqCst) {
            return Err(RecordingError::StartFailed(
                "Recording already in progress".to_string(),
            ));
        }

        self.audio_buffer.lock().unwrap().clear();
        self.is_recording.store(true, Ordering::SeqCst);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        self.start_time_ms.store(now, Ordering::SeqCst);

        let audio_buffer = Arc::clone(&self.audio_buffer);
        let device_sample_rate = Arc::clone(&self.device_sample_rate);
        let is_recording = Arc::clone(&self.is_recording);
        let elapsed_ms = Arc::clone(&self.elapsed_ms);
        let start_time_ms = Arc::clone(&self.start_time_ms);

        // Oneshot: the background thread reports whether the stream started.
        // Replaces the previous `tokio::time::sleep(50ms)` race.
        let (ready_tx, ready_rx) = oneshot::channel::<Result<u32, RecordingError>>();

        std::thread::spawn(move || {
            let audio_buffer_for_sink = Arc::clone(&audio_buffer);
            let is_recording_for_sink = Arc::clone(&is_recording);

            let handle = match CpalRecorder::build_input_stream(move |samples: &[i16]| {
                if is_recording_for_sink.load(Ordering::SeqCst) {
                    if let Ok(mut buffer) = audio_buffer_for_sink.lock() {
                        buffer.extend_from_slice(samples);
                    }
                }
            }) {
                Ok(h) => h,
                Err(e) => {
                    is_recording.store(false, Ordering::SeqCst);
                    let _ = ready_tx.send(Err(e));
                    return;
                }
            };

            device_sample_rate.store(handle.sample_rate, Ordering::SeqCst);
            let _ = ready_tx.send(Ok(handle.sample_rate));

            // Spin until stop/cancel flips the atomic; the stream lives in
            // `handle` and is dropped when this thread returns.
            while is_recording.load(Ordering::SeqCst) {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                let start = start_time_ms.load(Ordering::SeqCst);
                elapsed_ms.store(now.saturating_sub(start), Ordering::SeqCst);

                std::thread::sleep(std::time::Duration::from_millis(100));
            }

            drop(handle.stream);
        });

        // Wait for the worker to either succeed or fail. No timing hack.
        match ready_rx.await {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(RecordingError::StartFailed(
                "Recording thread terminated before signalling ready".into(),
            )),
        }
    }

    async fn stop(&self) -> Result<AudioData, RecordingError> {
        if !self.is_recording.load(Ordering::SeqCst) {
            return Err(RecordingError::RecordingFailed(
                "No recording in progress".to_string(),
            ));
        }

        // Flip the flag; the worker thread will exit its loop on its next
        // 100ms tick and drop the cpal stream.
        self.is_recording.store(false, Ordering::SeqCst);

        // Yield the runtime briefly so the worker thread observes the flag
        // and drops the stream before we read the buffer. (We can't add a
        // second oneshot here without a redesign of the worker loop; the
        // 100ms ceiling is a worst case, not a correctness requirement.)
        tokio::time::sleep(TokioDuration::from_millis(120)).await;

        let sample_rate = self.device_sample_rate.load(Ordering::SeqCst);
        if sample_rate == 0 {
            return Err(RecordingError::ReadFailed("Sample rate not set".into()));
        }

        let samples = std::mem::take(&mut *self.audio_buffer.lock().unwrap());
        if samples.is_empty() {
            return Err(RecordingError::ReadFailed(
                "No audio data captured".to_string(),
            ));
        }

        tokio::task::spawn_blocking(move || Self::encode_audio(&samples, sample_rate))
            .await
            .map_err(|e| RecordingError::RecordingFailed(format!("Encode task error: {}", e)))?
    }

    async fn cancel(&self) -> Result<(), RecordingError> {
        self.is_recording.store(false, Ordering::SeqCst);
        // Same rationale as `stop`: let the worker thread observe the flag.
        tokio::time::sleep(TokioDuration::from_millis(120)).await;
        self.audio_buffer.lock().unwrap().clear();
        self.elapsed_ms.store(0, Ordering::SeqCst);
        Ok(())
    }

    fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::SeqCst)
    }

    fn elapsed_ms(&self) -> u64 {
        self.elapsed_ms.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stereo_to_mono_single_channel() {
        let mono = vec![100i16, 200, 300];
        let result = stereo_to_mono(&mono, 1);
        assert_eq!(result, mono);
    }

    #[test]
    fn stereo_to_mono_two_channels() {
        let stereo = vec![100i16, 200, 300, 400];
        let result = stereo_to_mono(&stereo, 2);
        assert_eq!(result, vec![150, 350]);
    }

    #[test]
    fn recorder_default_state() {
        let recorder = CpalRecorder::new();
        assert!(!recorder.is_recording());
        assert_eq!(recorder.elapsed_ms(), 0);
    }
}
