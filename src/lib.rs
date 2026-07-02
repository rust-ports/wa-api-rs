//! Rust boundary for WhatsApp Cloud API behavior used by the backend.
//!
//! The crate intentionally stops at the platform boundary: typed Graph API
//! requests/responses, media helpers, webhook verification, and webhook parsing.
//! Persistence, tenant resolution, and product policy stay in the backend.

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
