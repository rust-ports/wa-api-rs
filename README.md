# wa-api-rs

`wa-api-rs` is a typed Rust client and webhook helper crate for the WhatsApp Cloud API.

It focuses on the core API surface needed by server applications:

- Typed configuration for Graph API version, phone number ID, access token, app secret, and webhook verification token.
- Typed request builders for text and media messages.
- Media upload, media metadata retrieval, and media byte download helpers.
- Meta/Graph API success/error response decoding.
- Webhook challenge verification.
- `X-Hub-Signature-256` request signature verification.
- Parsers for inbound messages, media messages, reactions, status updates, and click-to-WhatsApp referral fields.

This crate is intentionally application-neutral. It does not include persistence, identity middleware, job queues, tenant logic, or HTTP server routing.

## Status

This is an early Rust port of a focused WhatsApp Cloud API subset. Treat the public API as unstable until the crate reaches a `1.0.0` release.

## Installation

Until the crate is published, depend on the Git branch:

```toml
[dependencies]
wa-api-rs = { git = "https://github.com/rust-ports/wa-api-rs.git", branch = "api-sdk" }
```

## Example

```rust
use wa_api_rs::{
    Recipient, SendMessageOptions, TextMessage, WhatsAppApiClient, WhatsAppApiConfig,
};

#[tokio::main]
async fn main() -> wa_api_rs::Result<()> {
    let config = WhatsAppApiConfig::for_phone_number("PHONE_NUMBER_ID", "ACCESS_TOKEN")
        .with_app_secret("APP_SECRET");
    let client = WhatsAppApiClient::new(config)?;

    let message = TextMessage::new("Hello from Rust")?;
    let response = client
        .send_message(
            &Recipient::phone("201000000000"),
            &message.into(),
            SendMessageOptions::default(),
        )
        .await?;

    println!("sent message id: {:?}", response.first_message_id());

    Ok(())
}
```

## Webhook Verification

```rust
use wa_api_rs::{
    WebhookChallengeParams, verify_request_signature, verify_webhook_challenge,
};

let challenge = verify_webhook_challenge(
    WebhookChallengeParams {
        mode: Some("subscribe"),
        verify_token: Some("VERIFY_TOKEN_FROM_QUERY"),
        challenge: Some("CHALLENGE_FROM_QUERY"),
    },
    Some("EXPECTED_VERIFY_TOKEN"),
)?;

verify_request_signature(
    Some(r#"{"object":"whatsapp_business_account"}"#),
    Some("sha256=signature-from-meta"),
    Some("APP_SECRET"),
)?;
# Ok::<(), wa_api_rs::WhatsAppApiError>(())
```

## Design

The crate prefers typed request and response structs over ad-hoc JSON maps. Raw `serde_json::Value` is used only where the Graph API returns flexible error bodies or where diagnostics need to retain the original response shape.

## Tests

```powershell
cargo test
```

## Notes

- Access tokens and app secrets are redacted from `Debug` output.
- The default Graph base URL is `https://graph.facebook.com`.
- API version defaults to `v24.0`.
- This project is not affiliated with Meta or WhatsApp.
