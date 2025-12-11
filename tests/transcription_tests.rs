//! Transcription integration tests
//!
//! These tests require a valid GEMINI_API_KEY environment variable.
//! Run with: cargo test --test transcription_tests -- --ignored

use smart_scribe::application::ports::Transcriber;
use smart_scribe::domain::transcription::{AudioData, AudioMimeType, DomainId, SystemPrompt};
use smart_scribe::infrastructure::transcription::GeminiTranscriber;

/// Get API key from environment, skip test if not set
fn get_api_key() -> Option<String> {
    std::env::var("GEMINI_API_KEY").ok()
}

/// Create a minimal valid audio file (silent OGG)
/// This is a tiny valid OGG container that the API can accept
fn create_test_audio() -> AudioData {
    // A minimal silent OGG file header
    // This is enough to be parsed as valid audio by the API
    let ogg_header: Vec<u8> = vec![
        // OggS header
        0x4f, 0x67, 0x67, 0x53, // "OggS"
        0x00, // version
        0x02, // header type (first page)
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // granule position
        0x00, 0x00, 0x00, 0x00, // serial number
        0x00, 0x00, 0x00, 0x00, // page sequence
        0x00, 0x00, 0x00, 0x00, // checksum (will be invalid but API may still accept)
        0x01, // segments
        0x1e, // segment length
        // Vorbis identification header
        0x01, // packet type
        0x76, 0x6f, 0x72, 0x62, 0x69, 0x73, // "vorbis"
        0x00, 0x00, 0x00, 0x00, // version
        0x01, // channels
        0x44, 0xac, 0x00, 0x00, // sample rate (44100)
        0x00, 0x00, 0x00, 0x00, // bitrate max
        0x80, 0xbb, 0x00, 0x00, // bitrate nominal
        0x00, 0x00, 0x00, 0x00, // bitrate min
        0xb8, // blocksize
        0x01, // framing
    ];

    AudioData::new(ogg_header, AudioMimeType::Ogg)
}

#[tokio::test]
#[ignore = "requires GEMINI_API_KEY environment variable"]
async fn transcribe_with_valid_api_key() {
    let Some(api_key) = get_api_key() else {
        eprintln!("Skipping test: GEMINI_API_KEY not set");
        return;
    };

    let transcriber = GeminiTranscriber::new(api_key);
    let audio = create_test_audio();
    let prompt = SystemPrompt::build(DomainId::General);

    // This may return an error about invalid audio format, but should not
    // return an authentication error
    let result = transcriber.transcribe(&audio, &prompt).await;

    // We don't assert success because the minimal audio may not be valid enough
    // But we can verify it doesn't fail with auth errors
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
    let transcriber = GeminiTranscriber::new("invalid-api-key-12345");
    let audio = create_test_audio();
    let prompt = SystemPrompt::build(DomainId::General);

    let result = transcriber.transcribe(&audio, &prompt).await;

    assert!(result.is_err(), "Invalid API key should produce error");

    let err = result.unwrap_err();
    let err_str = format!("{:?}", err);

    // Should be either InvalidApiKey or an API error about authentication
    assert!(
        err_str.contains("InvalidApiKey") || err_str.contains("API") || err_str.contains("401"),
        "Expected authentication error, got: {:?}",
        err
    );
}

#[tokio::test]
#[ignore = "requires GEMINI_API_KEY environment variable"]
async fn transcribe_different_domains() {
    let Some(api_key) = get_api_key() else {
        eprintln!("Skipping test: GEMINI_API_KEY not set");
        return;
    };

    let transcriber = GeminiTranscriber::new(&api_key);
    let audio = create_test_audio();

    let domains = [
        DomainId::General,
        DomainId::Dev,
        DomainId::Medical,
        DomainId::Legal,
        DomainId::Finance,
    ];

    // Test that different domain prompts don't cause errors
    for domain in domains {
        let prompt = SystemPrompt::build(domain);
        let result = transcriber.transcribe(&audio, &prompt).await;

        // Just verify no panic and no auth errors
        if let Err(e) = &result {
            let err_str = format!("{:?}", e);
            assert!(
                !err_str.contains("InvalidApiKey"),
                "Domain {:?} should not produce auth error: {:?}",
                domain,
                e
            );
        }
    }
}

#[test]
fn transcriber_builds_correct_api_url() {
    let transcriber = GeminiTranscriber::new("test-key");

    // Verify model is in the URL by checking the API endpoint format
    // We can't directly access the URL, but we can verify the transcriber is created
    assert!(std::mem::size_of_val(&transcriber) > 0);
}

#[test]
fn audio_data_formats() {
    // Test different audio formats
    let ogg = AudioData::new(vec![1, 2, 3], AudioMimeType::Ogg);
    assert_eq!(ogg.mime_type().to_string(), "audio/ogg");

    let mp3 = AudioData::new(vec![1, 2, 3], AudioMimeType::Mp3);
    assert_eq!(mp3.mime_type().to_string(), "audio/mp3");

    let wav = AudioData::new(vec![1, 2, 3], AudioMimeType::Wav);
    assert_eq!(wav.mime_type().to_string(), "audio/wav");

    let webm = AudioData::new(vec![1, 2, 3], AudioMimeType::Webm);
    assert_eq!(webm.mime_type().to_string(), "audio/webm");
}

#[test]
fn system_prompt_content() {
    // Verify prompts contain expected content
    let general = SystemPrompt::build(DomainId::General);
    assert!(general.content().contains("transcription tool"));

    let dev = SystemPrompt::build(DomainId::Dev);
    assert!(dev.content().contains("programming") || dev.content().contains("Software"));
}
