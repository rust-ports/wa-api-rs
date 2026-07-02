use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct MediaUploadResponse {
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct MediaMetadataResponse {
    pub url: String,
    pub mime_type: Option<String>,
    pub sha256: Option<String>,
    pub file_size: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SendMessageResponse {
    #[serde(default)]
    pub messages: Vec<SentMessageRef>,
}

impl SendMessageResponse {
    pub fn first_message_id(&self) -> Option<&str> {
        self.messages.first().map(|message| message.id.as_str())
    }

    pub fn held_for_quality_assessment(&self) -> bool {
        self.messages
            .first()
            .and_then(|message| message.message_status.as_deref())
            == Some("held_for_quality_assessment")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SentMessageRef {
    pub id: String,
    pub message_status: Option<String>,
}

pub fn normalize_mime_type(mime_type: &str) -> String {
    let trimmed = mime_type.trim();
    if trimmed.is_empty() {
        return "application/octet-stream".to_string();
    }

    let Some((kind, rest)) = trimmed.split_once('/') else {
        return format!("{trimmed}/octet-stream");
    };

    let kind = kind.trim();
    if kind.is_empty() {
        return "application/octet-stream".to_string();
    }

    let subtype = rest.split('/').next().unwrap_or("").trim();
    if subtype.is_empty() {
        return format!("{kind}/octet-stream");
    }

    format!("{kind}/{subtype}")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn send_response_maps_first_message_id_and_quality_hold() {
        let response: SendMessageResponse = serde_json::from_value(json!({
            "messages": [
                {
                    "id": "wamid.1",
                    "message_status": "held_for_quality_assessment"
                }
            ]
        }))
        .unwrap();

        assert_eq!(response.first_message_id(), Some("wamid.1"));
        assert!(response.held_for_quality_assessment());
    }

    #[test]
    fn mime_normalization_matches_dart_sdk_safe_defaults() {
        assert_eq!(normalize_mime_type("   "), "application/octet-stream");
        assert_eq!(normalize_mime_type("text"), "text/octet-stream");
        assert_eq!(normalize_mime_type("image/png"), "image/png");
        assert_eq!(normalize_mime_type("image/png/extra"), "image/png");
        assert_eq!(normalize_mime_type("/png"), "application/octet-stream");
    }
}
