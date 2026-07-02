use crate::{Result, WhatsAppApiError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WebhookChallengeParams<'a> {
    pub mode: Option<&'a str>,
    pub verify_token: Option<&'a str>,
    pub challenge: Option<&'a str>,
}

pub fn verify_webhook_challenge(
    params: WebhookChallengeParams<'_>,
    expected_verify_token: Option<&str>,
) -> Result<String> {
    let expected_verify_token =
        expected_verify_token.ok_or(WhatsAppApiError::MissingVerifyToken)?;
    let mode = params.mode.ok_or(WhatsAppApiError::MissingSearchParams)?;
    let verify_token = params
        .verify_token
        .ok_or(WhatsAppApiError::MissingSearchParams)?;

    if mode == "subscribe" && verify_token == expected_verify_token {
        return Ok(params.challenge.unwrap_or("").to_string());
    }

    Err(WhatsAppApiError::FailedToVerifyToken)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webhook_challenge_matches_current_dart_sdk_behavior() {
        assert_eq!(
            verify_webhook_challenge(
                WebhookChallengeParams {
                    mode: Some("subscribe"),
                    verify_token: Some("VERIFY"),
                    challenge: Some("CHALLENGE"),
                },
                Some("VERIFY"),
            ),
            Ok("CHALLENGE".to_string())
        );
    }

    #[test]
    fn webhook_challenge_maps_missing_and_mismatch_errors() {
        assert_eq!(
            verify_webhook_challenge(
                WebhookChallengeParams {
                    mode: None,
                    verify_token: Some("VERIFY"),
                    challenge: None,
                },
                Some("VERIFY"),
            ),
            Err(WhatsAppApiError::MissingSearchParams)
        );
        assert_eq!(
            verify_webhook_challenge(
                WebhookChallengeParams {
                    mode: Some("subscribe"),
                    verify_token: Some("WRONG"),
                    challenge: None,
                },
                Some("VERIFY"),
            ),
            Err(WhatsAppApiError::FailedToVerifyToken)
        );
    }
}
