//! SDK-level error types.
//!
//! Meta/Graph errors are preserved with status code and decoded error details
//! for application logging, while callers can still map them into their own public
//! API error envelopes.

use serde::Deserialize;
use serde_json::Value;

pub type Result<T> = std::result::Result<T, WhatsAppApiError>;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum WhatsAppApiError {
    #[error("app_secret is required when secure mode is enabled")]
    MissingAppSecret,

    #[error("webhook_verify_token is required for webhook verification")]
    MissingVerifyToken,

    #[error("webhook verification mode and token are required")]
    MissingSearchParams,

    #[error("webhook verify token did not match")]
    FailedToVerifyToken,

    #[error("raw body is required when secure webhook validation is enabled")]
    MissingRawBody,

    #[error("x-hub-signature-256 is required for webhook validation")]
    MissingSignature,

    #[error("webhook signature verification failed")]
    FailedToVerifySignature,

    #[error("unexpected webhook payload: {message}")]
    UnexpectedWebhookPayload {
        message: &'static str,
        http_status: u16,
    },

    #[error("{field}: {message}")]
    Validation {
        field: &'static str,
        message: &'static str,
    },

    #[error("Graph API request failed with status {status_code}")]
    GraphApi {
        status_code: u16,
        body: Box<Value>,
        meta_error: Option<Box<MetaError>>,
    },

    #[error("HTTP request failed: {message}")]
    Http { message: String },

    #[error("failed to decode Graph API response: {message}")]
    Decode { message: String },

    #[error("unsupported WhatsApp API operation: {0}")]
    Unsupported(&'static str),
}

impl WhatsAppApiError {
    pub(crate) fn http(error: reqwest::Error) -> Self {
        Self::Http {
            message: error.to_string(),
        }
    }

    pub(crate) fn decode(error: impl std::fmt::Display) -> Self {
        Self::Decode {
            message: error.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct MetaError {
    pub message: Option<String>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub code: Option<i64>,
    pub error_subcode: Option<i64>,
    pub fbtrace_id: Option<String>,
}
