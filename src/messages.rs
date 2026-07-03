//! Typed WhatsApp outbound message payloads.
//!
//! These types model only the message kinds currently needed by the backend and
//! serialize to the nested JSON objects expected by the Cloud API.

use serde::{
    Serialize, Serializer,
    ser::{Error as _, SerializeMap},
};

use crate::{Result, WhatsAppApiError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TextMessage {
    pub body: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_url: Option<bool>,
}

impl TextMessage {
    pub fn new(body: impl Into<String>) -> Result<Self> {
        let body = body.into();
        if body.chars().count() > 4096 {
            return Err(WhatsAppApiError::Validation {
                field: "text.body",
                message: "must be 4096 characters or less",
            });
        }
        Ok(Self {
            body,
            preview_url: None,
        })
    }

    pub fn with_preview_url(mut self, preview_url: bool) -> Self {
        self.preview_url = Some(preview_url);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaReference {
    Id(String),
    Link(String),
}

impl MediaReference {
    pub fn id(id: impl Into<String>) -> Self {
        Self::Id(id.into())
    }

    pub fn link(link: impl Into<String>) -> Self {
        Self::Link(link.into())
    }
}

impl Serialize for MediaReference {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        match self {
            Self::Id(id) => map.serialize_entry("id", id)?,
            Self::Link(link) => map.serialize_entry("link", link)?,
        }
        map.end()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImageMessage {
    #[serde(flatten)]
    pub reference: MediaReference,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
}

impl ImageMessage {
    pub fn id(id: impl Into<String>) -> Self {
        Self {
            reference: MediaReference::id(id),
            caption: None,
        }
    }

    pub fn link(link: impl Into<String>) -> Self {
        Self {
            reference: MediaReference::link(link),
            caption: None,
        }
    }

    pub fn with_caption(mut self, caption: impl Into<String>) -> Self {
        self.caption = Some(caption.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VideoMessage {
    #[serde(flatten)]
    pub reference: MediaReference,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
}

impl VideoMessage {
    pub fn id(id: impl Into<String>) -> Self {
        Self {
            reference: MediaReference::id(id),
            caption: None,
        }
    }

    pub fn with_caption(mut self, caption: impl Into<String>) -> Self {
        self.caption = Some(caption.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AudioMessage {
    #[serde(flatten)]
    pub reference: MediaReference,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<bool>,
}

impl AudioMessage {
    pub fn id(id: impl Into<String>) -> Self {
        Self {
            reference: MediaReference::id(id),
            voice: None,
        }
    }

    pub fn with_voice(mut self, voice: bool) -> Self {
        self.voice = Some(voice);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DocumentMessage {
    #[serde(flatten)]
    pub reference: MediaReference,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
}

impl DocumentMessage {
    pub fn id(id: impl Into<String>) -> Self {
        Self {
            reference: MediaReference::id(id),
            caption: None,
            filename: None,
        }
    }

    pub fn with_caption(mut self, caption: impl Into<String>) -> Self {
        self.caption = Some(caption.into());
        self
    }

    pub fn with_filename(mut self, filename: impl Into<String>) -> Self {
        self.filename = Some(filename.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WhatsAppMessage {
    Text(TextMessage),
    Image(ImageMessage),
    Video(VideoMessage),
    Audio(AudioMessage),
    Document(DocumentMessage),
}

impl WhatsAppMessage {
    pub fn text(body: impl Into<String>) -> Result<Self> {
        Ok(Self::Text(TextMessage::new(body)?))
    }

    pub fn message_type(&self) -> &'static str {
        match self {
            Self::Text(_) => "text",
            Self::Image(_) => "image",
            Self::Video(_) => "video",
            Self::Audio(_) => "audio",
            Self::Document(_) => "document",
        }
    }
}

impl Serialize for WhatsAppMessage {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Text(message) => message.serialize(serializer),
            Self::Image(message) => message.serialize(serializer),
            Self::Video(message) => message.serialize(serializer),
            Self::Audio(message) => message.serialize(serializer),
            Self::Document(message) => message.serialize(serializer),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecipientType {
    Individual,
    Group,
}

impl RecipientType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Individual => "individual",
            Self::Group => "group",
        }
    }
}

impl Serialize for RecipientType {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MessageContext {
    pub message_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendMessageRequest {
    pub recipient_type: RecipientType,
    pub to: Option<String>,
    pub recipient: Option<String>,
    pub message: WhatsAppMessage,
    pub context: Option<MessageContext>,
    pub biz_opaque_callback_data: Option<String>,
}

impl SendMessageRequest {
    pub fn new(
        recipient_type: RecipientType,
        to: Option<String>,
        recipient: Option<String>,
        message: WhatsAppMessage,
        context: Option<MessageContext>,
        biz_opaque_callback_data: Option<String>,
    ) -> Self {
        Self {
            recipient_type,
            to,
            recipient,
            message,
            context,
            biz_opaque_callback_data,
        }
    }
}

impl Serialize for SendMessageRequest {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.to.is_none() && self.recipient.is_none() {
            return Err(S::Error::custom(
                "send message request requires `to` or `recipient`",
            ));
        }

        let mut field_count = 4;
        field_count += usize::from(self.to.is_some());
        field_count += usize::from(self.recipient.is_some());
        field_count += usize::from(self.context.is_some());
        field_count += usize::from(self.biz_opaque_callback_data.is_some());

        let mut map = serializer.serialize_map(Some(field_count))?;
        map.serialize_entry("messaging_product", "whatsapp")?;
        map.serialize_entry("recipient_type", &self.recipient_type)?;
        if let Some(to) = self.to.as_deref() {
            map.serialize_entry("to", to)?;
        }
        if let Some(recipient) = self.recipient.as_deref() {
            map.serialize_entry("recipient", recipient)?;
        }
        let message_type = self.message.message_type();
        map.serialize_entry("type", message_type)?;
        map.serialize_entry(message_type, &self.message)?;
        if let Some(context) = &self.context {
            map.serialize_entry("context", context)?;
        }
        if let Some(callback_data) = self.biz_opaque_callback_data.as_deref() {
            map.serialize_entry("biz_opaque_callback_data", callback_data)?;
        }
        map.end()
    }
}

impl From<TextMessage> for WhatsAppMessage {
    fn from(message: TextMessage) -> Self {
        Self::Text(message)
    }
}

impl From<ImageMessage> for WhatsAppMessage {
    fn from(message: ImageMessage) -> Self {
        Self::Image(message)
    }
}

impl From<VideoMessage> for WhatsAppMessage {
    fn from(message: VideoMessage) -> Self {
        Self::Video(message)
    }
}

impl From<AudioMessage> for WhatsAppMessage {
    fn from(message: AudioMessage) -> Self {
        Self::Audio(message)
    }
}

impl From<DocumentMessage> for WhatsAppMessage {
    fn from(message: DocumentMessage) -> Self {
        Self::Document(message)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn text_message_serializes_to_cloud_api_shape() {
        assert_eq!(
            serde_json::to_value(TextMessage::new("hello").unwrap()).unwrap(),
            json!({"body": "hello"})
        );
        assert_eq!(
            serde_json::to_value(TextMessage::new("hello").unwrap().with_preview_url(false))
                .unwrap(),
            json!({"body": "hello", "preview_url": false})
        );
    }

    #[test]
    fn media_message_serializes_by_id_or_link() {
        assert_eq!(
            serde_json::to_value(ImageMessage::link("https://example.com/a.png").with_caption("A"))
                .unwrap(),
            json!({"link": "https://example.com/a.png", "caption": "A"})
        );
        assert_eq!(
            serde_json::to_value(AudioMessage::id("media-1").with_voice(true)).unwrap(),
            json!({"id": "media-1", "voice": true})
        );
        assert_eq!(
            serde_json::to_value(
                DocumentMessage::id("media-2")
                    .with_caption("Doc")
                    .with_filename("doc.pdf")
            )
            .unwrap(),
            json!({"id": "media-2", "caption": "Doc", "filename": "doc.pdf"})
        );
    }

    #[test]
    fn full_send_message_request_serializes_dynamic_message_key() {
        let request = SendMessageRequest::new(
            RecipientType::Individual,
            Some("456".to_string()),
            None,
            WhatsAppMessage::from(TextMessage::new("hello").unwrap()),
            Some(MessageContext {
                message_id: "wamid.original".to_string(),
            }),
            Some("callback".to_string()),
        );

        assert_eq!(
            serde_json::to_value(request).unwrap(),
            json!({
                "messaging_product": "whatsapp",
                "recipient_type": "individual",
                "to": "456",
                "type": "text",
                "text": {"body": "hello"},
                "context": {"message_id": "wamid.original"},
                "biz_opaque_callback_data": "callback"
            })
        );
    }

    #[test]
    fn text_body_matches_cloud_api_max_length() {
        assert!(TextMessage::new("x".repeat(4096)).is_ok());
        assert!(TextMessage::new("م".repeat(4096)).is_ok());
        assert!(matches!(
            TextMessage::new("x".repeat(4097)),
            Err(WhatsAppApiError::Validation {
                field: "text.body",
                ..
            })
        ));
    }
}
