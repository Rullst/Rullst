#![cfg(feature = "ai")]
#![allow(clippy::expect_used)]

use rullst::ai::{
    AiCancellation, AiError, StreamingAiClient,
    providers::openai_compatible::{OpenAiCompatibleCapabilities, OpenAiCompatibleProvider},
};

#[tokio::test]
async fn umbrella_ai_feature_exposes_bounded_streaming_and_cancellation() {
    let provider = OpenAiCompatibleProvider::mock("facade-model")
        .expect("valid offline provider")
        .with_capabilities(OpenAiCompatibleCapabilities::chat_only().with_streaming());
    let client = StreamingAiClient::new(provider);
    let cancellation = AiCancellation::new();
    let mut text = String::new();
    let summary = client
        .stream_prompt("hello", &cancellation, &mut |chunk: &str| {
            text.push_str(chunk);
            Ok(())
        })
        .await
        .expect("facade stream succeeds");
    assert!(!text.is_empty());
    assert_eq!(summary.chunks(), 1);
    assert_eq!(summary.output_bytes(), text.len());

    cancellation.cancel();
    let mut discard = |_: &str| -> Result<(), AiError> { Ok(()) };
    assert!(matches!(
        client
            .stream_prompt("hello", &cancellation, &mut discard)
            .await,
        Err(AiError::Cancelled)
    ));
}
