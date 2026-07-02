use serde_json::{Map, Value, json};

use crate::{
    Result, WhatsAppApiConfig, WhatsAppMessage, media::normalize_mime_type, recipient::Recipient,
};

#[derive(Clone)]
pub struct WhatsAppApiClient {
    config: WhatsAppApiConfig,
}

impl WhatsAppApiClient {
    pub fn new(config: WhatsAppApiConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self { config })
    }

    pub fn config(&self) -> &WhatsAppApiConfig {
        &self.config
    }

    pub fn send_message_request(
        &self,
        recipient: &Recipient,
        message: &WhatsAppMessage,
        options: SendMessageOptions,
    ) -> GraphJsonRequest {
        let message_type = message.message_type();
        let mut body = Map::new();
        body.insert("messaging_product".to_string(), json!("whatsapp"));
        body.insert(
            "recipient_type".to_string(),
            json!(if recipient.is_group() {
                "group"
            } else {
                "individual"
            }),
        );
        if let Some(to) = recipient.send_to() {
            body.insert("to".to_string(), json!(to));
        }
        if let Some(recipient_id) = recipient.business_scoped_user_id() {
            body.insert("recipient".to_string(), json!(recipient_id));
        }
        body.insert("type".to_string(), json!(message_type));
        body.insert(message_type.to_string(), message.to_value());
        if let Some(context_message_id) = options.context_message_id {
            body.insert(
                "context".to_string(),
                json!({"message_id": context_message_id}),
            );
        }
        if let Some(callback_data) = options.biz_opaque_callback_data {
            body.insert("biz_opaque_callback_data".to_string(), json!(callback_data));
        }

        GraphJsonRequest {
            method: "POST",
            url: self
                .config
                .graph_url(format!("{}/messages", self.config.phone_number_id)),
            authorization_header: self.config.authorization_header(),
            body: Value::Object(body),
        }
    }

    pub fn upload_media_request(
        &self,
        bytes: impl Into<Vec<u8>>,
        filename: impl Into<String>,
        mime_type: impl AsRef<str>,
    ) -> MediaUploadRequest {
        let mime_type = normalize_mime_type(mime_type.as_ref());
        MediaUploadRequest {
            method: "POST",
            url: self
                .config
                .graph_url(format!("{}/media", self.config.phone_number_id)),
            authorization_header: self.config.authorization_header(),
            messaging_product: "whatsapp",
            media_type: mime_type,
            filename: filename.into(),
            bytes: bytes.into(),
        }
    }

    pub fn retrieve_media_request(&self, media_id: impl AsRef<str>) -> GraphJsonRequest {
        GraphJsonRequest {
            method: "GET",
            url: format!(
                "{}?phone_number_id={}",
                self.config.graph_url(media_id.as_ref()),
                self.config.phone_number_id
            ),
            authorization_header: self.config.authorization_header(),
            body: Value::Null,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SendMessageOptions {
    pub context_message_id: Option<String>,
    pub biz_opaque_callback_data: Option<String>,
}

impl SendMessageOptions {
    pub fn reply_to(context_message_id: impl Into<String>) -> Self {
        Self {
            context_message_id: Some(context_message_id.into()),
            biz_opaque_callback_data: None,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct GraphJsonRequest {
    pub method: &'static str,
    pub url: String,
    authorization_header: String,
    pub body: Value,
}

impl GraphJsonRequest {
    pub fn authorization_header(&self) -> &str {
        &self.authorization_header
    }
}

impl std::fmt::Debug for GraphJsonRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("GraphJsonRequest")
            .field("method", &self.method)
            .field("url", &self.url)
            .field("authorization_header", &"[redacted]")
            .field("body", &self.body)
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct MediaUploadRequest {
    pub method: &'static str,
    pub url: String,
    authorization_header: String,
    pub messaging_product: &'static str,
    pub media_type: String,
    pub filename: String,
    pub bytes: Vec<u8>,
}

impl MediaUploadRequest {
    pub fn authorization_header(&self) -> &str {
        &self.authorization_header
    }
}

impl std::fmt::Debug for MediaUploadRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MediaUploadRequest")
            .field("method", &self.method)
            .field("url", &self.url)
            .field("authorization_header", &"[redacted]")
            .field("messaging_product", &self.messaging_product)
            .field("media_type", &self.media_type)
            .field("filename", &self.filename)
            .field("bytes_len", &self.bytes.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::{AudioMessage, DocumentMessage, ImageMessage, TextMessage, VideoMessage};

    fn client() -> WhatsAppApiClient {
        WhatsAppApiClient::new(
            WhatsAppApiConfig::new("v24.0", "123", "TOKEN").with_app_secret("SECRET"),
        )
        .unwrap()
    }

    #[test]
    fn text_send_request_matches_current_dart_sdk_shape() {
        let request = client().send_message_request(
            &Recipient::phone("456"),
            &TextMessage::new("Hi").unwrap().into(),
            SendMessageOptions::default(),
        );

        assert_eq!(request.method, "POST");
        assert_eq!(request.url, "https://graph.facebook.com/v24.0/123/messages");
        assert_eq!(request.authorization_header(), "Bearer TOKEN");
        assert_eq!(
            request.body,
            json!({
                "messaging_product": "whatsapp",
                "recipient_type": "individual",
                "to": "456",
                "type": "text",
                "text": {"body": "Hi"}
            })
        );
    }

    #[test]
    fn reply_context_matches_current_dart_sdk_shape() {
        let request = client().send_message_request(
            &Recipient::phone("456"),
            &TextMessage::new("Reply").unwrap().into(),
            SendMessageOptions::reply_to("wamid.original"),
        );

        assert_eq!(
            request.body["context"],
            json!({"message_id": "wamid.original"})
        );
    }

    #[test]
    fn media_send_request_shapes_match_gateway_mapping() {
        let cases = [
            (
                ImageMessage::id("media-image")
                    .with_caption("caption")
                    .into(),
                json!({
                    "type": "image",
                    "image": {"id": "media-image", "caption": "caption"}
                }),
            ),
            (
                VideoMessage::id("media-video")
                    .with_caption("caption")
                    .into(),
                json!({
                    "type": "video",
                    "video": {"id": "media-video", "caption": "caption"}
                }),
            ),
            (
                AudioMessage::id("media-audio").with_voice(true).into(),
                json!({
                    "type": "audio",
                    "audio": {"id": "media-audio", "voice": true}
                }),
            ),
            (
                DocumentMessage::id("media-doc")
                    .with_caption("caption")
                    .with_filename("doc.pdf")
                    .into(),
                json!({
                    "type": "document",
                    "document": {
                        "id": "media-doc",
                        "caption": "caption",
                        "filename": "doc.pdf"
                    }
                }),
            ),
        ];

        for (message, expected) in cases {
            let request = client().send_message_request(
                &Recipient::phone("456"),
                &message,
                Default::default(),
            );
            assert_eq!(request.body["type"], expected["type"]);
            assert_eq!(
                request.body[expected["type"].as_str().unwrap()],
                expected[expected["type"].as_str().unwrap()]
            );
        }
    }

    #[test]
    fn media_upload_request_contains_current_dart_sdk_fields() {
        let request = client().upload_media_request([1, 2, 3], "clipboard-image.png", "image/png");

        assert_eq!(request.method, "POST");
        assert_eq!(request.url, "https://graph.facebook.com/v24.0/123/media");
        assert_eq!(request.authorization_header(), "Bearer TOKEN");
        assert_eq!(request.messaging_product, "whatsapp");
        assert_eq!(request.media_type, "image/png");
        assert_eq!(request.filename, "clipboard-image.png");
        assert_eq!(request.bytes, vec![1, 2, 3]);
    }

    #[test]
    fn media_metadata_request_uses_phone_number_query() {
        let request = client().retrieve_media_request("media-1");

        assert_eq!(request.method, "GET");
        assert_eq!(
            request.url,
            "https://graph.facebook.com/v24.0/media-1?phone_number_id=123"
        );
        assert_eq!(request.authorization_header(), "Bearer TOKEN");
    }

    #[test]
    fn request_debug_redacts_authorization() {
        let request = client().send_message_request(
            &Recipient::phone("456"),
            &TextMessage::new("Hi").unwrap().into(),
            SendMessageOptions::default(),
        );

        let debug = format!("{request:?}");
        assert!(debug.contains("[redacted]"));
        assert!(!debug.contains("TOKEN"));
    }
}
