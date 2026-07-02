//! Webhook verification and parser helpers.
//!
//! This module validates Meta webhook challenges/signatures and converts raw
//! WhatsApp webhook JSON into typed events that the backend can process
//! idempotently.

use hmac::{Hmac, Mac};
use serde_json::{Map, Value};
use sha2::Sha256;

use crate::{Result, WhatsAppApiError};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WebhookChallengeParams<'a> {
    pub mode: Option<&'a str>,
    pub verify_token: Option<&'a str>,
    pub challenge: Option<&'a str>,
}

pub fn verify_webhook_challenge(
    params: WebhookChallengeParams<'_>,
    expected_verify_token: Option<&str>,
) -> Result<String> {
    let expected_verify_token =
        expected_verify_token.ok_or(WhatsAppApiError::MissingVerifyToken)?;
    let mode = params.mode.ok_or(WhatsAppApiError::MissingSearchParams)?;
    let verify_token = params
        .verify_token
        .ok_or(WhatsAppApiError::MissingSearchParams)?;

    if mode == "subscribe" && verify_token == expected_verify_token {
        return Ok(params.challenge.unwrap_or("").to_string());
    }

    Err(WhatsAppApiError::FailedToVerifyToken)
}

pub fn verify_request_signature(
    raw_body: Option<&str>,
    signature: Option<&str>,
    app_secret: Option<&str>,
) -> Result<()> {
    // HMAC validation must use the exact raw body received by the HTTP route.
    // Re-serializing JSON can change whitespace or field order and invalidate a
    // legitimate Meta signature.
    let raw_body = raw_body.ok_or(WhatsAppApiError::MissingRawBody)?;
    let signature = signature
        .filter(|value| !value.trim().is_empty())
        .ok_or(WhatsAppApiError::MissingSignature)?;
    let app_secret = app_secret
        .filter(|value| !value.trim().is_empty())
        .ok_or(WhatsAppApiError::MissingAppSecret)?;

    let Some(digest) = signature.strip_prefix("sha256=") else {
        return Err(WhatsAppApiError::FailedToVerifySignature);
    };
    if digest.is_empty() {
        return Err(WhatsAppApiError::FailedToVerifySignature);
    }

    let mut mac = HmacSha256::new_from_slice(app_secret.as_bytes())
        .map_err(|_| WhatsAppApiError::MissingAppSecret)?;
    mac.update(raw_body.as_bytes());
    let expected = hex::encode(mac.finalize().into_bytes());

    if constant_time_equals(expected.as_bytes(), digest.as_bytes()) {
        Ok(())
    } else {
        Err(WhatsAppApiError::FailedToVerifySignature)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum WebhookEvent {
    Message(Box<InboundMessage>),
    Status(Box<StatusUpdate>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InboundMessageKind {
    Text,
    Image,
    Video,
    Audio,
    Document,
    Sticker,
    Reaction,
    Unsupported(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct InboundMessage {
    pub phone_number_id: String,
    pub from: String,
    pub message_id: Option<String>,
    pub timestamp: Option<String>,
    pub kind: InboundMessageKind,
    pub content: Option<String>,
    pub preview: String,
    pub media: Option<InboundMedia>,
    pub reaction: Option<ReactionUpdate>,
    pub referral: Option<ParsedAdReferral>,
    pub context_message_id: Option<String>,
    pub contact_name: Option<String>,
    pub contact: Option<Value>,
    pub raw_message: Value,
    pub raw: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboundMedia {
    pub media_type: String,
    pub media_id: String,
    pub caption: Option<String>,
    pub filename: String,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReactionUpdate {
    pub target_message_id: String,
    pub emoji: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedAdReferral {
    pub raw_referral: Value,
    pub parser_version: i32,
    pub source_type: Option<String>,
    pub source_id: Option<String>,
    pub source_url: Option<String>,
    pub headline: Option<String>,
    pub body: Option<String>,
    pub media_type: Option<String>,
    pub image_url: Option<String>,
    pub video_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub ctwa_clid: Option<String>,
    pub welcome_message: Option<Value>,
    pub ref_value: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StatusUpdate {
    pub phone_number_id: String,
    pub message_id: String,
    pub status: String,
    pub timestamp: Option<String>,
    pub recipient_type: String,
    pub recipient_id: Option<String>,
    pub recipient_user_id: Option<String>,
    pub parent_recipient_user_id: Option<String>,
    pub contact: Option<Value>,
    pub conversation: Option<Value>,
    pub pricing: Option<Value>,
    pub error: Option<Value>,
    pub biz_opaque_callback_data: Option<String>,
    pub raw_status: Value,
    pub raw: Value,
}

pub fn parse_webhook_event(data: &Value) -> Result<WebhookEvent> {
    // Meta wraps events inside entry/change arrays. This parser extracts the
    // first message or status event and leaves multi-event iteration to the
    // backend webhook route when it needs to process batches.
    if data.get("object").is_none() {
        return Err(unexpected_payload("Invalid payload", 400));
    }

    let entry = first_map(data.get("entry"))
        .ok_or_else(|| unexpected_payload("Unexpected payload", 200))?;
    let change = first_map(entry.get("changes"))
        .ok_or_else(|| unexpected_payload("Unexpected payload", 200))?;
    let field = string_value(change.get("field"));
    let value = change
        .get("value")
        .and_then(Value::as_object)
        .ok_or_else(|| unexpected_payload("Unexpected payload", 200))?;
    let phone_number_id = value
        .get("metadata")
        .and_then(Value::as_object)
        .and_then(|metadata| string_value(metadata.get("phone_number_id")))
        .unwrap_or_default();

    if field.as_deref() == Some("messages") {
        if let Some(message) = first_map(value.get("messages")) {
            return Ok(WebhookEvent::Message(Box::new(parse_inbound_message(
                &phone_number_id,
                message,
                value,
                data,
            ))));
        }
        if let Some(status) = first_map(value.get("statuses")) {
            return Ok(WebhookEvent::Status(Box::new(parse_status_update(
                &phone_number_id,
                status,
                value,
                data,
            ))));
        }
    }

    Err(unexpected_payload("Unexpected payload", 200))
}

fn parse_inbound_message(
    phone_number_id: &str,
    message: &Map<String, Value>,
    value: &Map<String, Value>,
    raw: &Value,
) -> InboundMessage {
    let kind_text = string_value(message.get("type"));
    let kind = message_kind(kind_text.as_deref());
    let contact = first_map(value.get("contacts")).map(|contact| Value::Object(contact.clone()));
    let contact_name = contact
        .as_ref()
        .and_then(Value::as_object)
        .and_then(|contact| contact.get("profile"))
        .and_then(Value::as_object)
        .and_then(|profile| string_value(profile.get("name")));
    let content = caption_from_message(message, kind_text.as_deref());
    let media = inbound_media_from_message(message, kind_text.as_deref(), content.as_deref());
    let reaction = reaction_from_message(message);
    let referral = parse_ad_referral_from_message(message);
    let context_message_id =
        message
            .get("context")
            .and_then(Value::as_object)
            .and_then(|context| {
                string_value(context.get("id")).or_else(|| string_value(context.get("message_id")))
            });
    let preview = preview_from_message(message, kind_text.as_deref(), content.as_deref());

    InboundMessage {
        phone_number_id: phone_number_id.to_string(),
        from: string_value(message.get("from")).unwrap_or_default(),
        message_id: string_value(message.get("id")),
        timestamp: string_value(message.get("timestamp")),
        kind,
        content,
        preview,
        media,
        reaction,
        referral,
        context_message_id,
        contact_name,
        contact,
        raw_message: Value::Object(message.clone()),
        raw: raw.clone(),
    }
}

fn parse_status_update(
    phone_number_id: &str,
    status: &Map<String, Value>,
    value: &Map<String, Value>,
    raw: &Value,
) -> StatusUpdate {
    StatusUpdate {
        phone_number_id: phone_number_id.to_string(),
        message_id: string_value(status.get("id")).unwrap_or_default(),
        status: string_value(status.get("status")).unwrap_or_default(),
        timestamp: string_value(status.get("timestamp")),
        recipient_type: string_value(status.get("recipient_type"))
            .unwrap_or_else(|| "individual".to_string()),
        recipient_id: string_value(status.get("recipient_id")),
        recipient_user_id: string_value(status.get("recipient_user_id")),
        parent_recipient_user_id: string_value(status.get("parent_recipient_user_id")),
        contact: first_map(value.get("contacts")).map(|contact| Value::Object(contact.clone())),
        conversation: status.get("conversation").cloned(),
        pricing: status.get("pricing").cloned(),
        error: first_map(status.get("errors")).map(|error| Value::Object(error.clone())),
        biz_opaque_callback_data: string_value(status.get("biz_opaque_callback_data")),
        raw_status: Value::Object(status.clone()),
        raw: raw.clone(),
    }
}

fn inbound_media_from_message(
    message: &Map<String, Value>,
    kind: Option<&str>,
    caption: Option<&str>,
) -> Option<InboundMedia> {
    let stored_media_type = stored_media_type(kind)?;
    let payload = message.get(kind?)?.as_object()?;
    let media_id = string_value(payload.get("id"))?;
    let mime_type = string_value(payload.get("mime_type"));
    Some(InboundMedia {
        media_type: stored_media_type.to_string(),
        media_id,
        caption: caption.map(str::to_string),
        filename: filename_from_webhook_payload(payload, kind.unwrap_or("document")),
        mime_type,
    })
}

fn reaction_from_message(message: &Map<String, Value>) -> Option<ReactionUpdate> {
    let reaction = message.get("reaction")?.as_object()?;
    let target_message_id = string_value(reaction.get("message_id"))?;
    if target_message_id.is_empty() {
        return None;
    }
    Some(ReactionUpdate {
        target_message_id,
        emoji: string_value(reaction.get("emoji")),
    })
}

fn parse_ad_referral_from_message(message: &Map<String, Value>) -> Option<ParsedAdReferral> {
    let referral = message.get("referral")?.as_object()?;
    Some(ParsedAdReferral {
        raw_referral: Value::Object(referral.clone()),
        parser_version: 1,
        source_type: string_value(referral.get("source_type")),
        source_id: string_value(referral.get("source_id")),
        source_url: string_value(referral.get("source_url")),
        headline: string_value(referral.get("headline")),
        body: string_value(referral.get("body")),
        media_type: string_value(referral.get("media_type")),
        image_url: string_value(referral.get("image_url")),
        video_url: string_value(referral.get("video_url")),
        thumbnail_url: string_value(referral.get("thumbnail_url")),
        ctwa_clid: string_value(referral.get("ctwa_clid")),
        welcome_message: referral.get("welcome_message").cloned(),
        ref_value: string_value(referral.get("ref")),
    })
}

fn caption_from_message(message: &Map<String, Value>, kind: Option<&str>) -> Option<String> {
    if kind == Some("text") {
        return message
            .get("text")
            .and_then(Value::as_object)
            .and_then(|text| string_value(text.get("body")));
    }
    message
        .get(kind?)
        .and_then(Value::as_object)
        .and_then(|payload| string_value(payload.get("caption")))
}

fn preview_from_message(
    message: &Map<String, Value>,
    kind: Option<&str>,
    caption: Option<&str>,
) -> String {
    if let Some(caption) = caption.filter(|caption| !caption.is_empty()) {
        return caption.to_string();
    }

    match kind {
        Some("text") => caption_from_message(message, kind).unwrap_or_default(),
        Some("image") | Some("sticker") => "[Image]".to_string(),
        Some("video") => "[Video]".to_string(),
        Some("audio") => "[Audio]".to_string(),
        Some("document") => {
            let payload = message
                .get("document")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            filename_from_webhook_payload(&payload, "document")
        }
        Some(kind) => format!("[{kind} message]"),
        None => "[Unsupported message]".to_string(),
    }
}

fn message_kind(kind: Option<&str>) -> InboundMessageKind {
    match kind {
        Some("text") => InboundMessageKind::Text,
        Some("image") => InboundMessageKind::Image,
        Some("video") => InboundMessageKind::Video,
        Some("audio") => InboundMessageKind::Audio,
        Some("document") => InboundMessageKind::Document,
        Some("sticker") => InboundMessageKind::Sticker,
        Some("reaction") => InboundMessageKind::Reaction,
        Some(kind) => InboundMessageKind::Unsupported(kind.to_string()),
        None => InboundMessageKind::Unsupported(String::new()),
    }
}

fn stored_media_type(kind: Option<&str>) -> Option<&'static str> {
    match kind {
        Some("image") | Some("sticker") => Some("image"),
        Some("video") => Some("video"),
        Some("audio") => Some("audio"),
        Some("document") => Some("document"),
        _ => None,
    }
}

fn filename_from_webhook_payload(payload: &Map<String, Value>, kind: &str) -> String {
    if let Some(filename) = string_value(payload.get("filename"))
        && !filename.is_empty()
    {
        return filename;
    }

    let extension = match string_value(payload.get("mime_type")).as_deref() {
        Some("image/jpeg") => ".jpg",
        Some("image/png") => ".png",
        Some("image/webp") => ".webp",
        Some("video/mp4") => ".mp4",
        Some("audio/ogg") => ".ogg",
        Some("audio/mpeg") => ".mp3",
        Some("application/pdf") => ".pdf",
        _ => "",
    };

    match kind {
        "image" | "sticker" => format!("image{extension}"),
        "video" => format!("video{extension}"),
        "audio" => format!("audio{extension}"),
        _ => format!("document{extension}"),
    }
}

fn first_map(value: Option<&Value>) -> Option<&Map<String, Value>> {
    value?
        .as_array()
        .and_then(|items| items.first())
        .and_then(Value::as_object)
}

fn string_value(value: Option<&Value>) -> Option<String> {
    let text = match value? {
        Value::String(value) => value.trim().to_string(),
        Value::Null => return None,
        value => value.to_string().trim().to_string(),
    };
    (!text.is_empty()).then_some(text)
}

fn unexpected_payload(message: &'static str, http_status: u16) -> WhatsAppApiError {
    WhatsAppApiError::UnexpectedWebhookPayload {
        message,
        http_status,
    }
}

fn constant_time_equals(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut result = 0_u8;
    for (left, right) in left.iter().zip(right.iter()) {
        result |= left ^ right;
    }
    result == 0
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn webhook_challenge_matches_current_dart_sdk_behavior() {
        assert_eq!(
            verify_webhook_challenge(
                WebhookChallengeParams {
                    mode: Some("subscribe"),
                    verify_token: Some("VERIFY"),
                    challenge: Some("CHALLENGE"),
                },
                Some("VERIFY"),
            ),
            Ok("CHALLENGE".to_string())
        );
    }

    #[test]
    fn webhook_challenge_maps_missing_and_mismatch_errors() {
        assert_eq!(
            verify_webhook_challenge(
                WebhookChallengeParams {
                    mode: None,
                    verify_token: Some("VERIFY"),
                    challenge: None,
                },
                Some("VERIFY"),
            ),
            Err(WhatsAppApiError::MissingSearchParams)
        );
        assert_eq!(
            verify_webhook_challenge(
                WebhookChallengeParams {
                    mode: Some("subscribe"),
                    verify_token: Some("WRONG"),
                    challenge: None,
                },
                Some("VERIFY"),
            ),
            Err(WhatsAppApiError::FailedToVerifyToken)
        );
    }

    #[test]
    fn signature_validation_matches_current_dart_sdk_behavior() {
        let body = r#"{"object":"whatsapp_business_account"}"#;
        let mut mac = HmacSha256::new_from_slice(b"SECRET").unwrap();
        mac.update(body.as_bytes());
        let signature = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));

        assert_eq!(
            verify_request_signature(Some(body), Some(&signature), Some("SECRET")),
            Ok(())
        );
        assert_eq!(
            verify_request_signature(Some(body), Some("sha256=wrong"), Some("SECRET")),
            Err(WhatsAppApiError::FailedToVerifySignature)
        );
        assert_eq!(
            verify_request_signature(Some(body), None, Some("SECRET")),
            Err(WhatsAppApiError::MissingSignature)
        );
        assert_eq!(
            verify_request_signature(None, Some(&signature), Some("SECRET")),
            Err(WhatsAppApiError::MissingRawBody)
        );
    }

    #[test]
    fn parses_inbound_text_message() {
        let event = parse_webhook_event(&json!({
            "object": "whatsapp_business_account",
            "entry": [{
                "changes": [{
                    "field": "messages",
                    "value": {
                        "metadata": {"phone_number_id": "123"},
                        "contacts": [{
                            "profile": {"name": "Test Customer"},
                            "wa_id": "456"
                        }],
                        "messages": [{
                            "id": "wamid.1",
                            "from": "456",
                            "timestamp": "1710000000",
                            "type": "text",
                            "text": {"body": "Hello"}
                        }]
                    }
                }]
            }]
        }))
        .unwrap();

        let WebhookEvent::Message(message) = event else {
            panic!("expected message event");
        };
        assert_eq!(message.phone_number_id, "123");
        assert_eq!(message.from, "456");
        assert_eq!(message.message_id.as_deref(), Some("wamid.1"));
        assert_eq!(message.kind, InboundMessageKind::Text);
        assert_eq!(message.content.as_deref(), Some("Hello"));
        assert_eq!(message.preview, "Hello");
        assert_eq!(message.contact_name.as_deref(), Some("Test Customer"));
    }

    #[test]
    fn parses_media_message_with_ad_referral() {
        let event = parse_webhook_event(&json!({
            "object": "whatsapp_business_account",
            "entry": [{
                "changes": [{
                    "field": "messages",
                    "value": {
                        "metadata": {"phone_number_id": "123"},
                        "messages": [{
                            "id": "wamid.image",
                            "from": "456",
                            "type": "image",
                            "image": {
                                "id": "media-1",
                                "mime_type": "image/jpeg",
                                "caption": "A nice thing"
                            },
                            "referral": {
                                "source_type": "ad",
                                "source_id": "source-1",
                                "source_url": "https://example.com/ad",
                                "headline": "Headline",
                                "body": "Body",
                                "media_type": "image",
                                "image_url": "https://example.com/image.jpg",
                                "thumbnail_url": "https://example.com/thumb.jpg",
                                "ctwa_clid": "clid",
                                "welcome_message": {"text": "hi"},
                                "ref": "campaign"
                            }
                        }]
                    }
                }]
            }]
        }))
        .unwrap();

        let WebhookEvent::Message(message) = event else {
            panic!("expected message event");
        };
        assert_eq!(message.kind, InboundMessageKind::Image);
        assert_eq!(message.preview, "A nice thing");
        let media = message.media.unwrap();
        assert_eq!(media.media_type, "image");
        assert_eq!(media.media_id, "media-1");
        assert_eq!(media.filename, "image.jpg");
        assert_eq!(media.mime_type.as_deref(), Some("image/jpeg"));

        let referral = message.referral.unwrap();
        assert_eq!(referral.parser_version, 1);
        assert_eq!(referral.source_type.as_deref(), Some("ad"));
        assert_eq!(referral.ctwa_clid.as_deref(), Some("clid"));
        assert_eq!(referral.ref_value.as_deref(), Some("campaign"));
        assert_eq!(referral.welcome_message, Some(json!({"text": "hi"})));
    }

    #[test]
    fn parses_reaction_message() {
        let event = parse_webhook_event(&json!({
            "object": "whatsapp_business_account",
            "entry": [{
                "changes": [{
                    "field": "messages",
                    "value": {
                        "metadata": {"phone_number_id": "123"},
                        "messages": [{
                            "id": "wamid.reaction",
                            "from": "456",
                            "type": "reaction",
                            "reaction": {
                                "message_id": "wamid.target",
                                "emoji": "👍"
                            }
                        }]
                    }
                }]
            }]
        }))
        .unwrap();

        let WebhookEvent::Message(message) = event else {
            panic!("expected message event");
        };
        assert_eq!(message.kind, InboundMessageKind::Reaction);
        let reaction = message.reaction.unwrap();
        assert_eq!(reaction.target_message_id, "wamid.target");
        assert_eq!(reaction.emoji.as_deref(), Some("👍"));
    }

    #[test]
    fn parses_status_update() {
        let event = parse_webhook_event(&json!({
            "object": "whatsapp_business_account",
            "entry": [{
                "changes": [{
                    "field": "messages",
                    "value": {
                        "metadata": {"phone_number_id": "123"},
                        "statuses": [{
                            "id": "wamid.1",
                            "status": "delivered",
                            "timestamp": "1710000001",
                            "recipient_type": "individual",
                            "recipient_id": "456",
                            "conversation": {"id": "conv"},
                            "pricing": {"billable": true},
                            "biz_opaque_callback_data": "opaque"
                        }]
                    }
                }]
            }]
        }))
        .unwrap();

        let WebhookEvent::Status(status) = event else {
            panic!("expected status event");
        };
        assert_eq!(status.phone_number_id, "123");
        assert_eq!(status.message_id, "wamid.1");
        assert_eq!(status.status, "delivered");
        assert_eq!(status.recipient_id.as_deref(), Some("456"));
        assert_eq!(status.biz_opaque_callback_data.as_deref(), Some("opaque"));
        assert_eq!(status.conversation, Some(json!({"id": "conv"})));
    }

    #[test]
    fn rejects_unexpected_payload_shapes() {
        assert_eq!(
            parse_webhook_event(&json!({})),
            Err(WhatsAppApiError::UnexpectedWebhookPayload {
                message: "Invalid payload",
                http_status: 400,
            })
        );
        assert!(matches!(
            parse_webhook_event(&json!({
                "object": "whatsapp_business_account",
                "entry": [{"changes": [{"field": "messages", "value": {}}]}]
            })),
            Err(WhatsAppApiError::UnexpectedWebhookPayload { .. })
        ));
    }
}
