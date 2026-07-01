//! Wared-focused WhatsApp Cloud API client.
//!
//! This crate will port only the WhatsApp behavior used by Wared. The current
//! Dart SDK and backend routes define Wared compatibility, the original JS SDK
//! is a design reference, and Meta's WhatsApp Cloud API docs remain the API
//! correctness source of truth.

pub const DEFAULT_GRAPH_API_VERSION: &str = "v24.0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WhatsAppApiConfig {
    pub graph_api_version: String,
    pub phone_number_id: String,
    pub access_token: String,
}

impl WhatsAppApiConfig {
    pub fn new(
        graph_api_version: impl Into<String>,
        phone_number_id: impl Into<String>,
        access_token: impl Into<String>,
    ) -> Self {
        Self {
            graph_api_version: graph_api_version.into(),
            phone_number_id: phone_number_id.into(),
            access_token: access_token.into(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WhatsAppApiError {
    #[error("unsupported WhatsApp API operation: {0}")]
    Unsupported(&'static str),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_api_version_matches_current_backend_default() {
        assert_eq!(DEFAULT_GRAPH_API_VERSION, "v24.0");
    }
}
