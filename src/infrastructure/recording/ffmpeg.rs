//! FFmpeg-based audio recorder adapter

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use tokio::fs;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio::time::{interval, Duration as TokioDuration};

use crate::application::ports::{
    AudioRecorder, ProgressCallback, RecordingError, UnboundedRecorder,
};
use crate::domain::recording::Duration;
use crate::domain::transcription::{AudioData, AudioMimeType};

/// Temp file for audio recording
struct TempAudioFile {
    path: PathBuf,
}

impl TempAudioFile {
    fn new() -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);

        let path = PathBuf::from(format!("/tmp/smartscribe-{}.ogg", timestamp));
        Self { path }
    }

    fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl Drop for TempAudioFile {
    fn drop(&mut self) {
        // Best-effort cleanup
        let _ = std::fs::remove_file(&self.path);
    }
}

/// FFmpeg recorder for both bounded and unbounded recording
pub struct FfmpegRecorder {
    /// Current FFmpeg process (for unbounded recording)
    process: Arc<Mutex<Option<Child>>>,
    /// Current temp file path
    output_path: Arc<Mutex<Option<PathBuf>>>,
    /// Recording state
    is_recording: Arc<AtomicBool>,
    /// Recording start time (for elapsed tracking)
    start_time: Arc<Mutex<Option<Instant>>>,
    /// Elapsed time in milliseconds
    elapsed_ms: Arc<AtomicU64>,
}

impl FfmpegRecorder {
    /// Create a new FFmpeg recorder
    pub fn new() -> Self {
        Self {
            process: Arc::new(Mutex::new(None)),
            output_path: Arc::new(Mutex::new(None)),
            is_recording: Arc::new(AtomicBool::new(false)),
            start_time: Arc::new(Mutex::new(None)),
            elapsed_ms: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Build FFmpeg args for recording
    fn build_ffmpeg_args(output_path: &Path, duration_secs: Option<u64>) -> Vec<String> {
        let mut args = vec![
            "-f".to_string(),
            "pulse".to_string(),
            "-i".to_string(),
            "default".to_string(),
        ];

        // Add duration if bounded recording
        if let Some(secs) = duration_secs {
            args.push("-t".to_string());
            args.push(secs.to_string());
        }

        // Audio encoding settings (optimized for speech)
        args.extend([
            "-ar".to_string(),
            "16000".to_string(), // 16kHz sample rate
            "-ac".to_string(),
            "1".to_string(), // Mono
            "-c:a".to_string(),
            "libopus".to_string(), // Opus codec
            "-b:a".to_string(),
            "16k".to_string(), // 16kbps bitrate
            "-application".to_string(),
            "voip".to_string(), // Optimize for voice
            "-y".to_string(),   // Overwrite output
            output_path.to_string_lossy().to_string(),
        ]);

        args
    }

    /// Spawn FFmpeg process
    async fn spawn_ffmpeg(args: Vec<String>) -> Result<Child, RecordingError> {
        Command::new("ffmpeg")
            .args(&args)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    RecordingError::FfmpegNotFound
                } else {
                    RecordingError::StartFailed(e.to_string())
                }
            })
    }

    /// Read recorded audio file
    async fn read_audio_file(path: &PathBuf) -> Result<AudioData, RecordingError> {
        let data = fs::read(path)
            .await
            .map_err(|e| RecordingError::ReadFailed(e.to_string()))?;

        if data.is_empty() {
            return Err(RecordingError::ReadFailed(
                "Recording file is empty".to_string(),
            ));
        }

        Ok(AudioData::new(data, AudioMimeType::Ogg))
    }

    /// Send signal to FFmpeg process
    fn send_signal(child: &Child, sig: Signal) -> Result<(), RecordingError> {
        if let Some(id) = child.id() {
            signal::kill(Pid::from_raw(id as i32), sig)
                .map_err(|e| RecordingError::RecordingFailed(format!("Signal failed: {}", e)))?;
        }
        Ok(())
    }
}

impl Default for FfmpegRecorder {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AudioRecorder for FfmpegRecorder {
    async fn record(
        &self,
        duration: Duration,
        on_progress: Option<ProgressCallback>,
    ) -> Result<AudioData, RecordingError> {
        let temp_file = TempAudioFile::new();
        let output_path = temp_file.path().clone();
        let duration_ms = duration.as_millis();
        let duration_secs = duration.as_secs();

        // Build and spawn FFmpeg
        let args = Self::build_ffmpeg_args(&output_path, Some(duration_secs));
        let mut child = Self::spawn_ffmpeg(args).await?;

        // Start progress reporting if callback provided
        if let Some(progress) = on_progress {
            let start = Instant::now();
            let progress_clone = Arc::clone(&progress);

            tokio::spawn(async move {
                let mut ticker = interval(TokioDuration::from_millis(100));
                loop {
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

        // Wait for FFmpeg to complete
        let status = child
            .wait()
            .await
            .map_err(|e| RecordingError::RecordingFailed(format!("FFmpeg failed: {}", e)))?;

        if !status.success() {
            // Read stderr for error message
            if let Some(mut stderr) = child.stderr.take() {
                use tokio::io::AsyncReadExt;
                let mut buf = Vec::new();
                let _ = stderr.read_to_end(&mut buf).await;
                let err_msg = String::from_utf8_lossy(&buf);
                return Err(RecordingError::RecordingFailed(format!(
                    "FFmpeg exited with error: {}",
                    err_msg.lines().last().unwrap_or("unknown error")
                )));
            }
            return Err(RecordingError::RecordingFailed(
                "FFmpeg exited with non-zero status".to_string(),
            ));
        }

        // Read the recorded file
        Self::read_audio_file(&output_path).await
    }
}

#[async_trait]
impl UnboundedRecorder for FfmpegRecorder {
    async fn start(&self) -> Result<(), RecordingError> {
        let mut process_guard = self.process.lock().await;
        if process_guard.is_some() {
            return Err(RecordingError::StartFailed(
                "Recording already in progress".to_string(),
            ));
        }

        let temp_file = TempAudioFile::new();
        let output_path = temp_file.path().clone();

        // Store output path for later (we need to prevent TempAudioFile from dropping)
        {
            let mut path_guard = self.output_path.lock().await;
            *path_guard = Some(output_path.clone());
        }

        // Build and spawn FFmpeg (no duration limit)
        let args = Self::build_ffmpeg_args(&output_path, None);
        let child = Self::spawn_ffmpeg(args).await?;

        *process_guard = Some(child);
        self.is_recording.store(true, Ordering::SeqCst);

        // Store start time
        {
            let mut start_guard = self.start_time.lock().await;
            *start_guard = Some(Instant::now());
        }

        // Start elapsed time updater
        let elapsed_ms = Arc::clone(&self.elapsed_ms);
        let start_time = Arc::clone(&self.start_time);
        let is_recording = Arc::clone(&self.is_recording);

        tokio::spawn(async move {
            let mut ticker = interval(TokioDuration::from_millis(100));
            while is_recording.load(Ordering::SeqCst) {
                ticker.tick().await;
                if let Some(start) = *start_time.lock().await {
                    elapsed_ms.store(start.elapsed().as_millis() as u64, Ordering::SeqCst);
                }
            }
        });

        // Prevent temp file cleanup - we manually handle it
        std::mem::forget(temp_file);

        Ok(())
    }

    async fn stop(&self) -> Result<AudioData, RecordingError> {
        let mut process_guard = self.process.lock().await;
        let child = process_guard.take().ok_or_else(|| {
            RecordingError::RecordingFailed("No recording in progress".to_string())
        })?;

        self.is_recording.store(false, Ordering::SeqCst);

        // Send SIGINT for graceful stop (FFmpeg will finalize the file)
        Self::send_signal(&child, Signal::SIGINT)?;

        // Wait for process to finish
        let _ = child.wait_with_output().await;

        // Get output path
        let output_path = {
            let path_guard = self.output_path.lock().await;
            path_guard
                .clone()
                .ok_or_else(|| RecordingError::ReadFailed("Output path not set".to_string()))?
        };

        // Read the file
        let result = Self::read_audio_file(&output_path).await;

        // Cleanup
        let _ = fs::remove_file(&output_path).await;
        {
            let mut path_guard = self.output_path.lock().await;
            *path_guard = None;
        }

        result
    }

    async fn cancel(&self) -> Result<(), RecordingError> {
        let mut process_guard = self.process.lock().await;
        if let Some(child) = process_guard.take() {
            self.is_recording.store(false, Ordering::SeqCst);

            // Send SIGKILL for immediate termination
            Self::send_signal(&child, Signal::SIGKILL)?;

            // Wait for process to finish
            let _ = child.wait_with_output().await;
        }

        // Cleanup output file
        let output_path = {
            let mut path_guard = self.output_path.lock().await;
            path_guard.take()
        };

        if let Some(path) = output_path {
            let _ = fs::remove_file(&path).await;
        }

        Ok(())
    }

    fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::SeqCst)
    }

    fn elapsed_ms(&self) -> u64 {
        self.elapsed_ms.load(Ordering::SeqCst)
    }
}
