//! Rust client and webhook helpers for the WhatsApp Cloud API.
//!
//! This crate provides typed request builders, response models, webhook
//! challenge verification, webhook signature verification, and parsers for
//! common WhatsApp Cloud API message and status payloads.

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
pub use webhook::{
    InboundMedia, InboundMessage, InboundMessageKind, ParsedAdReferral, ReactionUpdate,
    StatusUpdate, WebhookChallengeParams, WebhookEvent, parse_webhook_event,
    verify_request_signature, verify_webhook_challenge,
};
