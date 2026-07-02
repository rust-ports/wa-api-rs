//! Wared-focused WhatsApp Cloud API client.
//!
//! This crate ports the WhatsApp behavior used by Wared. The current Dart SDK
//! and backend routes define Wared compatibility, the original JS SDK is a
//! design reference, and Meta's WhatsApp Cloud API docs remain the API
//! correctness source of truth.

mod client;
mod config;
mod error;
mod media;
mod messages;
mod recipient;
mod webhook;

pub use client::{GraphJsonRequest, MediaUploadRequest, SendMessageOptions, WhatsAppApiClient};
pub use config::{DEFAULT_GRAPH_API_VERSION, WhatsAppApiConfig};
pub use error::{MetaError, Result, WhatsAppApiError};
pub use media::{
    MediaMetadataResponse, MediaUploadResponse, SendMessageResponse, SentMessageRef,
    normalize_mime_type,
};
pub use messages::{
    AudioMessage, DocumentMessage, ImageMessage, MediaReference, TextMessage, VideoMessage,
    WhatsAppMessage,
};
pub use recipient::Recipient;
pub use webhook::{WebhookChallengeParams, verify_webhook_challenge};
