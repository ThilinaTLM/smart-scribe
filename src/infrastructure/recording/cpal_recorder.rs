//! Cross-platform audio recorder using cpal
//!
//! Matches FFmpeg's speech-optimized settings:
//! - 16kHz sample rate (or resampling from device rate)
//! - Mono channel
//! - Opus codec via separate encoder

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Instant;

use async_trait::async_trait;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, SampleRate, StreamConfig};
use rubato::{FftFixedIn, Resampler};
use tokio::time::{interval, Duration as TokioDuration};

use super::opus_encoder::{OpusEncoder, TARGET_SAMPLE_RATE};
use crate::application::ports::{
    AudioRecorder, ProgressCallback, RecordingError, UnboundedRecorder,
};
use crate::domain::recording::Duration;
use crate::domain::transcription::{AudioData, AudioMimeType};

/// Audio recorder using cpal, matching FFmpeg's speech-optimized settings
///
/// The stream is managed separately from the struct to avoid Send/Sync issues
/// with cpal::Stream which is not thread-safe.
pub struct CpalRecorder {
    /// Recorded audio samples (mono, i16, at device sample rate)
    audio_buffer: Arc<StdMutex<Vec<i16>>>,
    /// Device sample rate (may differ from target 16kHz)
    device_sample_rate: Arc<AtomicU32>,
    /// Recording state
    is_recording: Arc<AtomicBool>,
    /// Recording start time (stored as millis since epoch for atomic access)
    start_time_ms: Arc<AtomicU64>,
    /// Elapsed time in milliseconds
    elapsed_ms: Arc<AtomicU64>,
}

impl CpalRecorder {
    /// Create a new cpal-based recorder
    pub fn new() -> Self {
        Self {
            audio_buffer: Arc::new(StdMutex::new(Vec::new())),
            device_sample_rate: Arc::new(AtomicU32::new(0)),
            is_recording: Arc::new(AtomicBool::new(false)),
            start_time_ms: Arc::new(AtomicU64::new(0)),
            elapsed_ms: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Get the default input device
    fn get_input_device() -> Result<cpal::Device, RecordingError> {
        let host = cpal::default_host();
        host.default_input_device()
            .ok_or(RecordingError::NoAudioDevice)
    }

    /// Get a suitable input configuration
    fn get_input_config(
        device: &cpal::Device,
    ) -> Result<(StreamConfig, SampleFormat), RecordingError> {
        let supported_configs = device
            .supported_input_configs()
            .map_err(|e| RecordingError::StartFailed(format!("Failed to get configs: {}", e)))?;

        // Try to find a config that supports our target sample rate
        // Prefer mono, but accept stereo (we'll mix down)
        let mut best_config: Option<cpal::SupportedStreamConfigRange> = None;

        for config in supported_configs {
            // Only consider i16 or f32 formats
            if config.sample_format() != SampleFormat::I16
                && config.sample_format() != SampleFormat::F32
            {
                continue;
            }

            // Prefer configs that include 16kHz
            let includes_target = config.min_sample_rate().0 <= TARGET_SAMPLE_RATE
                && config.max_sample_rate().0 >= TARGET_SAMPLE_RATE;

            let is_better = match &best_config {
                None => true,
                Some(current) => {
                    // Prefer mono over stereo
                    let fewer_channels = config.channels() < current.channels();
                    // Prefer configs that include our target rate
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

        // Use target sample rate if supported, otherwise use the minimum
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

    /// Resample audio from device rate to 16kHz if needed
    fn resample_to_16k(samples: &[i16], source_rate: u32) -> Result<Vec<i16>, RecordingError> {
        if source_rate == TARGET_SAMPLE_RATE {
            return Ok(samples.to_vec());
        }

        // Convert i16 to f32 for resampling
        let samples_f32: Vec<f32> = samples.iter().map(|&s| s as f32 / 32768.0).collect();

        // Calculate output length
        let ratio = TARGET_SAMPLE_RATE as f64 / source_rate as f64;
        let output_len = (samples_f32.len() as f64 * ratio).ceil() as usize;

        // Use rubato for high-quality resampling
        let mut resampler = FftFixedIn::<f32>::new(
            source_rate as usize,
            TARGET_SAMPLE_RATE as usize,
            1024, // Chunk size
            2,    // Sub-chunks
            1,    // Mono
        )
        .map_err(|e| RecordingError::RecordingFailed(format!("Resampler init failed: {}", e)))?;

        let mut output = Vec::with_capacity(output_len);
        let mut input_pos = 0;

        while input_pos < samples_f32.len() {
            let frames_needed = resampler.input_frames_next();
            let end_pos = (input_pos + frames_needed).min(samples_f32.len());
            let chunk: Vec<Vec<f32>> = vec![samples_f32[input_pos..end_pos].to_vec()];

            // Pad if we don't have enough samples
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

        // Trim to expected output length
        output.truncate(output_len);

        Ok(output)
    }

    /// Mix stereo to mono
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

    /// Encode PCM samples to Opus OGG format
    fn encode_audio(samples: &[i16], sample_rate: u32) -> Result<AudioData, RecordingError> {
        // Resample to 16kHz if needed
        let resampled = Self::resample_to_16k(samples, sample_rate)?;

        // Encode to Opus OGG
        let mut encoder = OpusEncoder::new()
            .map_err(|e| RecordingError::RecordingFailed(format!("Opus init failed: {}", e)))?;

        let ogg_data = encoder
            .encode_to_ogg(&resampled)
            .map_err(|e| RecordingError::RecordingFailed(format!("Encoding failed: {}", e)))?;

        if ogg_data.is_empty() {
            return Err(RecordingError::ReadFailed("Encoded audio is empty".into()));
        }

        Ok(AudioData::new(ogg_data, AudioMimeType::Ogg))
    }
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

        // Clear buffer
        {
            let mut buffer = self.audio_buffer.lock().unwrap();
            buffer.clear();
        }

        // Mark as recording
        self.is_recording.store(true, Ordering::SeqCst);

        // Clone Arcs for the blocking task
        let audio_buffer = Arc::clone(&self.audio_buffer);
        let device_sample_rate = Arc::clone(&self.device_sample_rate);
        let is_recording = Arc::clone(&self.is_recording);

        // Start recording in a blocking task (cpal::Stream is not Send)
        let record_handle = tokio::task::spawn_blocking(move || {
            let device = CpalRecorder::get_input_device()?;
            let (config, sample_format) = CpalRecorder::get_input_config(&device)?;
            let sample_rate = config.sample_rate.0;
            let channels = config.channels;

            device_sample_rate.store(sample_rate, Ordering::SeqCst);

            let audio_buffer_clone = Arc::clone(&audio_buffer);
            let is_recording_clone = Arc::clone(&is_recording);

            let stream = match sample_format {
                SampleFormat::I16 => device
                    .build_input_stream(
                        &config,
                        move |data: &[i16], _: &cpal::InputCallbackInfo| {
                            if is_recording_clone.load(Ordering::SeqCst) {
                                let mono = CpalRecorder::stereo_to_mono(data, channels);
                                if let Ok(mut buffer) = audio_buffer_clone.lock() {
                                    buffer.extend_from_slice(&mono);
                                }
                            }
                        },
                        |err| eprintln!("Audio stream error: {}", err),
                        None,
                    )
                    .map_err(|e| RecordingError::StartFailed(e.to_string()))?,

                SampleFormat::F32 => {
                    let audio_buffer_clone = Arc::clone(&audio_buffer);
                    let is_recording_clone = Arc::clone(&is_recording);

                    device
                        .build_input_stream(
                            &config,
                            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                                if is_recording_clone.load(Ordering::SeqCst) {
                                    let i16_data: Vec<i16> =
                                        data.iter().map(|&s| (s * 32767.0) as i16).collect();
                                    let mono = CpalRecorder::stereo_to_mono(&i16_data, channels);
                                    if let Ok(mut buffer) = audio_buffer_clone.lock() {
                                        buffer.extend_from_slice(&mono);
                                    }
                                }
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

            // Wait for the duration (blocking)
            std::thread::sleep(std::time::Duration::from_millis(duration_ms));

            // Stop recording
            is_recording.store(false, Ordering::SeqCst);
            drop(stream);

            Ok::<u32, RecordingError>(sample_rate)
        });

        // Start progress reporting if callback provided
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

        // Wait for recording to complete
        let sample_rate = record_handle
            .await
            .map_err(|e| RecordingError::RecordingFailed(format!("Task join error: {}", e)))??;

        // Get the recorded samples
        let samples = {
            let buffer = self.audio_buffer.lock().unwrap();
            buffer.clone()
        };

        if samples.is_empty() {
            return Err(RecordingError::ReadFailed(
                "No audio data captured".to_string(),
            ));
        }

        // Encode to Opus OGG (in blocking task for CPU-intensive work)
        let encoded =
            tokio::task::spawn_blocking(move || Self::encode_audio(&samples, sample_rate))
                .await
                .map_err(|e| {
                    RecordingError::RecordingFailed(format!("Encode task error: {}", e))
                })??;

        Ok(encoded)
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

        // Clear buffer
        {
            let mut buffer = self.audio_buffer.lock().unwrap();
            buffer.clear();
        }

        // Mark as recording
        self.is_recording.store(true, Ordering::SeqCst);

        // Store start time
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        self.start_time_ms.store(now, Ordering::SeqCst);

        // Clone Arcs for the background recording thread
        let audio_buffer = Arc::clone(&self.audio_buffer);
        let device_sample_rate = Arc::clone(&self.device_sample_rate);
        let is_recording = Arc::clone(&self.is_recording);
        let elapsed_ms = Arc::clone(&self.elapsed_ms);
        let start_time_ms = Arc::clone(&self.start_time_ms);

        // Start recording in a background thread (not spawn_blocking since we don't await it)
        std::thread::spawn(move || {
            let device = match CpalRecorder::get_input_device() {
                Ok(d) => d,
                Err(_) => {
                    is_recording.store(false, Ordering::SeqCst);
                    return;
                }
            };

            let (config, sample_format) = match CpalRecorder::get_input_config(&device) {
                Ok(c) => c,
                Err(_) => {
                    is_recording.store(false, Ordering::SeqCst);
                    return;
                }
            };

            let sample_rate = config.sample_rate.0;
            let channels = config.channels;
            device_sample_rate.store(sample_rate, Ordering::SeqCst);

            let audio_buffer_clone = Arc::clone(&audio_buffer);
            let is_recording_clone = Arc::clone(&is_recording);

            let stream_result = match sample_format {
                SampleFormat::I16 => device.build_input_stream(
                    &config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        if is_recording_clone.load(Ordering::SeqCst) {
                            let mono = CpalRecorder::stereo_to_mono(data, channels);
                            if let Ok(mut buffer) = audio_buffer_clone.lock() {
                                buffer.extend_from_slice(&mono);
                            }
                        }
                    },
                    |err| eprintln!("Audio stream error: {}", err),
                    None,
                ),

                SampleFormat::F32 => {
                    let audio_buffer_clone = Arc::clone(&audio_buffer);
                    let is_recording_clone = Arc::clone(&is_recording);

                    device.build_input_stream(
                        &config,
                        move |data: &[f32], _: &cpal::InputCallbackInfo| {
                            if is_recording_clone.load(Ordering::SeqCst) {
                                let i16_data: Vec<i16> =
                                    data.iter().map(|&s| (s * 32767.0) as i16).collect();
                                let mono = CpalRecorder::stereo_to_mono(&i16_data, channels);
                                if let Ok(mut buffer) = audio_buffer_clone.lock() {
                                    buffer.extend_from_slice(&mono);
                                }
                            }
                        },
                        |err| eprintln!("Audio stream error: {}", err),
                        None,
                    )
                }

                _ => {
                    is_recording.store(false, Ordering::SeqCst);
                    return;
                }
            };

            let stream = match stream_result {
                Ok(s) => s,
                Err(_) => {
                    is_recording.store(false, Ordering::SeqCst);
                    return;
                }
            };

            if stream.play().is_err() {
                is_recording.store(false, Ordering::SeqCst);
                return;
            }

            // Keep recording until stopped
            while is_recording.load(Ordering::SeqCst) {
                // Update elapsed time
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                let start = start_time_ms.load(Ordering::SeqCst);
                elapsed_ms.store(now.saturating_sub(start), Ordering::SeqCst);

                std::thread::sleep(std::time::Duration::from_millis(100));
            }

            drop(stream);
        });

        // Give the thread a moment to start
        tokio::time::sleep(TokioDuration::from_millis(50)).await;

        // Check if recording actually started
        if !self.is_recording.load(Ordering::SeqCst) {
            return Err(RecordingError::StartFailed(
                "Failed to start recording".into(),
            ));
        }

        Ok(())
    }

    async fn stop(&self) -> Result<AudioData, RecordingError> {
        if !self.is_recording.load(Ordering::SeqCst) {
            return Err(RecordingError::RecordingFailed(
                "No recording in progress".to_string(),
            ));
        }

        // Stop recording
        self.is_recording.store(false, Ordering::SeqCst);

        // Give the thread a moment to clean up
        tokio::time::sleep(TokioDuration::from_millis(100)).await;

        // Get sample rate
        let sample_rate = self.device_sample_rate.load(Ordering::SeqCst);
        if sample_rate == 0 {
            return Err(RecordingError::ReadFailed("Sample rate not set".into()));
        }

        // Get the recorded samples
        let samples = {
            let mut buffer = self.audio_buffer.lock().unwrap();
            std::mem::take(&mut *buffer)
        };

        if samples.is_empty() {
            return Err(RecordingError::ReadFailed(
                "No audio data captured".to_string(),
            ));
        }

        // Encode to Opus OGG
        let encoded =
            tokio::task::spawn_blocking(move || Self::encode_audio(&samples, sample_rate))
                .await
                .map_err(|e| {
                    RecordingError::RecordingFailed(format!("Encode task error: {}", e))
                })??;

        Ok(encoded)
    }

    async fn cancel(&self) -> Result<(), RecordingError> {
        // Stop recording
        self.is_recording.store(false, Ordering::SeqCst);

        // Give the thread a moment to clean up
        tokio::time::sleep(TokioDuration::from_millis(100)).await;

        // Clear buffer
        {
            let mut buffer = self.audio_buffer.lock().unwrap();
            buffer.clear();
        }

        // Reset elapsed time
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
        let result = CpalRecorder::stereo_to_mono(&mono, 1);
        assert_eq!(result, mono);
    }

    #[test]
    fn stereo_to_mono_two_channels() {
        let stereo = vec![100i16, 200, 300, 400];
        let result = CpalRecorder::stereo_to_mono(&stereo, 2);
        assert_eq!(result, vec![150, 350]); // Average of each pair
    }

    #[test]
    fn recorder_default_state() {
        let recorder = CpalRecorder::new();
        assert!(!recorder.is_recording());
        assert_eq!(recorder.elapsed_ms(), 0);
    }
}
