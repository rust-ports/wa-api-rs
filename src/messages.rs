use serde_json::{Map, Value, json};

use crate::{Result, WhatsAppApiError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextMessage {
    pub body: String,
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

    fn to_value(&self) -> Value {
        let mut object = Map::new();
        object.insert("body".to_string(), json!(self.body));
        if let Some(preview_url) = self.preview_url {
            object.insert("preview_url".to_string(), json!(preview_url));
        }
        Value::Object(object)
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

    fn write_json(&self, object: &mut Map<String, Value>) {
        match self {
            Self::Id(id) => {
                object.insert("id".to_string(), json!(id));
            }
            Self::Link(link) => {
                object.insert("link".to_string(), json!(link));
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageMessage {
    pub reference: MediaReference,
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

    fn to_value(&self) -> Value {
        media_value(&self.reference, [("caption", self.caption.as_deref())])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoMessage {
    pub reference: MediaReference,
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

    fn to_value(&self) -> Value {
        media_value(&self.reference, [("caption", self.caption.as_deref())])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioMessage {
    pub reference: MediaReference,
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

    fn to_value(&self) -> Value {
        let mut object = Map::new();
        self.reference.write_json(&mut object);
        if let Some(voice) = self.voice {
            object.insert("voice".to_string(), json!(voice));
        }
        Value::Object(object)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentMessage {
    pub reference: MediaReference,
    pub caption: Option<String>,
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

    fn to_value(&self) -> Value {
        let mut object = Map::new();
        self.reference.write_json(&mut object);
        if let Some(caption) = self.caption.as_deref() {
            object.insert("caption".to_string(), json!(caption));
        }
        if let Some(filename) = self.filename.as_deref() {
            object.insert("filename".to_string(), json!(filename));
        }
        Value::Object(object)
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

    pub fn to_value(&self) -> Value {
        match self {
            Self::Text(message) => message.to_value(),
            Self::Image(message) => message.to_value(),
            Self::Video(message) => message.to_value(),
            Self::Audio(message) => message.to_value(),
            Self::Document(message) => message.to_value(),
        }
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

fn media_value<const N: usize>(
    reference: &MediaReference,
    optional_strings: [(&str, Option<&str>); N],
) -> Value {
    let mut object = Map::new();
    reference.write_json(&mut object);
    for (key, value) in optional_strings {
        if let Some(value) = value {
            object.insert(key.to_string(), json!(value));
        }
    }
    Value::Object(object)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_message_serializes_like_current_dart_sdk() {
        assert_eq!(
            TextMessage::new("hello").unwrap().to_value(),
            json!({"body": "hello"})
        );
        assert_eq!(
            TextMessage::new("hello")
                .unwrap()
                .with_preview_url(false)
                .to_value(),
            json!({"body": "hello", "preview_url": false})
        );
    }

    #[test]
    fn media_message_serializes_by_id_or_link() {
        assert_eq!(
            ImageMessage::link("https://example.com/a.png")
                .with_caption("A")
                .to_value(),
            json!({"link": "https://example.com/a.png", "caption": "A"})
        );
        assert_eq!(
            AudioMessage::id("media-1").with_voice(true).to_value(),
            json!({"id": "media-1", "voice": true})
        );
        assert_eq!(
            DocumentMessage::id("media-2")
                .with_caption("Doc")
                .with_filename("doc.pdf")
                .to_value(),
            json!({"id": "media-2", "caption": "Doc", "filename": "doc.pdf"})
        );
    }

    #[test]
    fn text_body_matches_current_max_length() {
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
