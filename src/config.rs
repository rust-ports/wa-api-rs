//! Per-account WhatsApp API configuration.
//!
//! The backend creates one config from the tenant's connected WhatsApp account:
//! Graph API version, phone number id, decrypted access token, app secret, and
//! optional test Graph base URL.

use std::fmt;

use crate::{Result, WhatsAppApiError};

pub const DEFAULT_GRAPH_API_VERSION: &str = "v24.0";

#[derive(Clone, PartialEq, Eq)]
pub struct WhatsAppApiConfig {
    pub graph_api_version: String,
    pub phone_number_id: String,
    pub access_token: String,
    pub app_secret: Option<String>,
    pub webhook_verify_token: Option<String>,
    pub secure: bool,
    pub graph_base_url: String,
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
            app_secret: None,
            webhook_verify_token: None,
            secure: true,
            graph_base_url: "https://graph.facebook.com".to_string(),
        }
    }

    pub fn for_phone_number(
        phone_number_id: impl Into<String>,
        access_token: impl Into<String>,
    ) -> Self {
        Self::new(DEFAULT_GRAPH_API_VERSION, phone_number_id, access_token)
    }

    pub fn with_app_secret(mut self, app_secret: impl Into<String>) -> Self {
        self.app_secret = Some(app_secret.into());
        self
    }

    pub fn with_webhook_verify_token(mut self, token: impl Into<String>) -> Self {
        self.webhook_verify_token = Some(token.into());
        self
    }

    pub fn with_secure(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }

    pub fn with_graph_base_url(mut self, graph_base_url: impl Into<String>) -> Self {
        self.graph_base_url = graph_base_url.into();
        self
    }

    pub fn validate(&self) -> Result<()> {
        validate_not_blank("graph_api_version", &self.graph_api_version)?;
        validate_not_blank("phone_number_id", &self.phone_number_id)?;
        validate_not_blank("access_token", &self.access_token)?;
        validate_not_blank("graph_base_url", &self.graph_base_url)?;
        if self.secure && self.app_secret.as_deref().unwrap_or("").trim().is_empty() {
            return Err(WhatsAppApiError::MissingAppSecret);
        }
        Ok(())
    }

    pub(crate) fn graph_url(&self, path: impl AsRef<str>) -> String {
        format!(
            "{}/{}/{}",
            self.graph_base_url.trim_end_matches('/'),
            self.graph_api_version.trim_matches('/'),
            path.as_ref().trim_start_matches('/')
        )
    }

    pub(crate) fn authorization_header(&self) -> String {
        format!("Bearer {}", self.access_token)
    }
}

impl fmt::Debug for WhatsAppApiConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Config often appears in startup diagnostics; redact every field that
        // can authenticate or verify a platform account.
        formatter
            .debug_struct("WhatsAppApiConfig")
            .field("graph_api_version", &self.graph_api_version)
            .field("phone_number_id", &self.phone_number_id)
            .field("access_token", &"[redacted]")
            .field(
                "app_secret",
                &self.app_secret.as_ref().map(|_| "[redacted]"),
            )
            .field(
                "webhook_verify_token",
                &self.webhook_verify_token.as_ref().map(|_| "[redacted]"),
            )
            .field("secure", &self.secure)
            .field("graph_base_url", &self.graph_base_url)
            .finish()
    }
}

fn validate_not_blank(field: &'static str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(WhatsAppApiError::Validation {
            field,
            message: "must not be blank",
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_api_version_matches_current_backend_default() {
        assert_eq!(DEFAULT_GRAPH_API_VERSION, "v24.0");
        assert_eq!(
            WhatsAppApiConfig::for_phone_number("phone-id", "token").graph_api_version,
            "v24.0"
        );
    }

    #[test]
    fn secure_mode_requires_app_secret() {
        assert_eq!(
            WhatsAppApiConfig::for_phone_number("phone-id", "token").validate(),
            Err(WhatsAppApiError::MissingAppSecret)
        );
        assert!(
            WhatsAppApiConfig::for_phone_number("phone-id", "token")
                .with_app_secret("secret")
                .validate()
                .is_ok()
        );
        assert!(
            WhatsAppApiConfig::for_phone_number("phone-id", "token")
                .with_secure(false)
                .validate()
                .is_ok()
        );
    }

    #[test]
    fn debug_redacts_secrets() {
        let debug = format!(
            "{:?}",
            WhatsAppApiConfig::for_phone_number("phone-id", "ACCESS_TOKEN_VALUE")
                .with_app_secret("APP_SECRET_VALUE")
                .with_webhook_verify_token("VERIFY_TOKEN_VALUE")
        );
        assert!(debug.contains("[redacted]"));
        assert!(!debug.contains("ACCESS_TOKEN_VALUE"));
        assert!(!debug.contains("APP_SECRET_VALUE"));
        assert!(!debug.contains("VERIFY_TOKEN_VALUE"));
    }

    #[test]
    fn graph_base_url_can_point_to_mock_graph_server() {
        let config = WhatsAppApiConfig::for_phone_number("phone-id", "token")
            .with_secure(false)
            .with_graph_base_url("http://127.0.0.1:9876");

        assert!(config.validate().is_ok());
        assert_eq!(
            config.graph_url("phone-id/messages"),
            "http://127.0.0.1:9876/v24.0/phone-id/messages"
        );
    }
}
