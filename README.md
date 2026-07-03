# whatsapp_api_rust

`whatsapp_api_rust` is a typed Rust client and webhook utility crate for the WhatsApp Cloud API.

The crate focuses on the platform boundary:

- Building typed outbound message requests for text and media messages.
- Uploading media and fetching media metadata/bytes through the Graph API.
- Decoding Graph API success and error responses.
- Verifying webhook challenges and `X-Hub-Signature-256` request signatures.
- Parsing common inbound webhook events such as messages, media, reactions, statuses, and ad referrals.

## Status

This crate is an early Rust port of an existing WhatsApp Cloud API integration. It is intended to be small, explicit, and testable rather than a full Meta API surface.

## Install

```toml
[dependencies]
whatsapp_api_rust = { git = "https://github.com/rust-ports/wa-api-rs.git", branch = "api-sdk" }
```

## Example

```rust
use whatsapp_api_rust::{
    Recipient, SendMessageOptions, TextMessage, WhatsAppApiClient, WhatsAppApiConfig,
};

#[tokio::main]
async fn main() -> whatsapp_api_rust::Result<()> {
    let config = WhatsAppApiConfig::new("v24.0", "PHONE_NUMBER_ID", "ACCESS_TOKEN");
    let client = WhatsAppApiClient::new(config)?;

    let message = TextMessage::new("Hello from Rust")?;
    client
        .send_message(
            &Recipient::phone("201000000000"),
            &message.into(),
            SendMessageOptions::default(),
        )
        .await?;

    Ok(())
}
```

## Webhook Verification

```rust
use whatsapp_api_rust::{verify_request_signature, verify_webhook_challenge};

let challenge = verify_webhook_challenge(
    "subscribe",
    "expected_verify_token",
    "expected_verify_token",
    "challenge-value",
)?;

verify_request_signature(
    "app-secret",
    br#"{"object":"whatsapp_business_account"}"#,
    "sha256=signature-from-meta",
)?;
```

## Design

The crate prefers typed request and response structs over ad-hoc JSON maps. Raw `serde_json::Value` is used only where the Graph API returns flexible error bodies or where diagnostics need to retain the original response shape.

## Tests

```powershell
cargo test
```
