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

    #[error("{field}: {message}")]
    Validation {
        field: &'static str,
        message: &'static str,
    },

    #[error("unsupported WhatsApp API operation: {0}")]
    Unsupported(&'static str),
}
