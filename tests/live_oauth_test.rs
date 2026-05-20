//! Live end-to-end test against `chatgpt.com/backend-api/transcribe`.
//!
//! Requires:
//!   - a populated OAuth token at `<config_dir>/smart-scribe/oauth.json`
//!     (run `smart-scribe login` or `smart-scribe login --from-codex` first)
//!   - a FLAC audio file at `$SMART_SCRIBE_TEST_AUDIO`
//!
//! Run with: `cargo test --test live_oauth_test -- --ignored --nocapture`

use smart_scribe::application::ports::Transcriber;
use smart_scribe::domain::transcription::{AudioData, AudioMimeType};
use smart_scribe::infrastructure::transcription::ChatGptOAuthTranscriber;
use smart_scribe::infrastructure::OAuthStore;

#[tokio::test]
#[ignore = "live network call; requires oauth.json + SMART_SCRIBE_TEST_AUDIO"]
async fn transcribe_with_oauth_against_chatgpt_backend() {
    let store = OAuthStore::new().expect("oauth store init");
    if store.load().ok().flatten().is_none() {
        panic!(
            "no oauth token found at {} - run `smart-scribe login` first",
            store.path().display()
        );
    }

    let audio_path = std::env::var("SMART_SCRIBE_TEST_AUDIO")
        .unwrap_or_else(|_| panic!("SMART_SCRIBE_TEST_AUDIO env var must point at a FLAC file"));
    let bytes = std::fs::read(&audio_path).expect("read audio file");
    let audio = AudioData::new(bytes, AudioMimeType::Flac).with_duration_ms(5_000);

    let transcriber = ChatGptOAuthTranscriber::new(store);
    let text = transcriber
        .transcribe(&audio)
        .await
        .expect("transcribe should succeed");
    assert!(!text.is_empty(), "transcribed text must not be empty");
    eprintln!("transcribed: {text}");
}
