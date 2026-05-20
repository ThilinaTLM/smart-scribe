//! Transcription integration tests.
//!
//! These tests exercise the OpenAI API-key path (`/v1/audio/transcriptions`).
//! They are gated on `OPENAI_API_KEY` and `--ignored`.

use smart_scribe::application::ports::Transcriber;
use smart_scribe::domain::transcription::{AudioData, AudioMimeType};
use smart_scribe::infrastructure::transcription::OpenAiApiTranscriber;

fn get_api_key() -> Option<String> {
    std::env::var("OPENAI_API_KEY").ok()
}

/// Create a minimal valid audio file (silent FLAC).
fn create_test_audio() -> AudioData {
    let flac_header: Vec<u8> = vec![
        0x66, 0x4c, 0x61, 0x43, // "fLaC"
        0x80, 0x00, 0x00, 0x22, // STREAMINFO header (last=1, type=0, len=34)
        0x10, 0x00, 0x10, 0x00, // min/max block size 4096
        0x00, 0x00, 0x00, // min frame size
        0x00, 0x00, 0x00, // max frame size
        0x03, 0xe8, 0x00, // sample rate (16000) + bits + channels
        0x00, 0x00, 0x00, 0x00, 0x00, // sample rate cont + total samples
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ];
    AudioData::new(flac_header, AudioMimeType::Flac)
}

#[tokio::test]
#[ignore = "requires OPENAI_API_KEY environment variable"]
async fn transcribe_with_valid_api_key() {
    let Some(api_key) = get_api_key() else {
        eprintln!("Skipping test: OPENAI_API_KEY not set");
        return;
    };

    let transcriber = OpenAiApiTranscriber::new(api_key, "gpt-4o-mini-transcribe");
    let audio = create_test_audio();
    let result = transcriber.transcribe(&audio).await;

    // The minimal FLAC may not be valid enough to produce text, but it should
    // not produce an auth error.
    if let Err(e) = &result {
        let err_str = format!("{:?}", e);
        assert!(
            !err_str.contains("InvalidApiKey"),
            "Valid API key should not produce InvalidApiKey error: {:?}",
            e
        );
    }
}

#[tokio::test]
#[ignore = "requires network access"]
async fn transcribe_with_invalid_api_key() {
    let transcriber = OpenAiApiTranscriber::new("invalid-api-key-12345", "gpt-4o-mini-transcribe");
    let audio = create_test_audio();
    let result = transcriber.transcribe(&audio).await;

    assert!(result.is_err(), "Invalid API key should produce error");
    let err_str = format!("{:?}", result.unwrap_err());
    assert!(
        err_str.contains("InvalidApiKey") || err_str.contains("401") || err_str.contains("API"),
        "Expected authentication error, got: {err_str}",
    );
}

#[test]
fn audio_data_formats() {
    let flac = AudioData::new(vec![1, 2, 3], AudioMimeType::Flac);
    assert_eq!(flac.mime_type().to_string(), "audio/flac");

    let mp3 = AudioData::new(vec![1, 2, 3], AudioMimeType::Mp3);
    assert_eq!(mp3.mime_type().to_string(), "audio/mp3");

    let wav = AudioData::new(vec![1, 2, 3], AudioMimeType::Wav);
    assert_eq!(wav.mime_type().to_string(), "audio/wav");

    let webm = AudioData::new(vec![1, 2, 3], AudioMimeType::Webm);
    assert_eq!(webm.mime_type().to_string(), "audio/webm");
}
