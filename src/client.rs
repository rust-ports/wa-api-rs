use serde::de::DeserializeOwned;
use serde_json::{Map, Value, json};

use crate::{
    MediaMetadataResponse, MediaUploadResponse, Result, SendMessageResponse, WhatsAppApiConfig,
    WhatsAppApiError, WhatsAppMessage, error::MetaError, media::normalize_mime_type,
    recipient::Recipient,
};

#[derive(Clone)]
pub struct WhatsAppApiClient {
    config: WhatsAppApiConfig,
    http: reqwest::Client,
}

impl WhatsAppApiClient {
    pub fn new(config: WhatsAppApiConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            config,
            http: reqwest::Client::new(),
        })
    }

    pub fn with_http_client(config: WhatsAppApiConfig, http: reqwest::Client) -> Result<Self> {
        config.validate()?;
        Ok(Self { config, http })
    }

    pub fn config(&self) -> &WhatsAppApiConfig {
        &self.config
    }

    pub fn send_message_request(
        &self,
        recipient: &Recipient,
        message: &WhatsAppMessage,
        options: SendMessageOptions,
    ) -> GraphJsonRequest {
        let message_type = message.message_type();
        let mut body = Map::new();
        body.insert("messaging_product".to_string(), json!("whatsapp"));
        body.insert(
            "recipient_type".to_string(),
            json!(if recipient.is_group() {
                "group"
            } else {
                "individual"
            }),
        );
        if let Some(to) = recipient.send_to() {
            body.insert("to".to_string(), json!(to));
        }
        if let Some(recipient_id) = recipient.business_scoped_user_id() {
            body.insert("recipient".to_string(), json!(recipient_id));
        }
        body.insert("type".to_string(), json!(message_type));
        body.insert(message_type.to_string(), message.to_value());
        if let Some(context_message_id) = options.context_message_id {
            body.insert(
                "context".to_string(),
                json!({"message_id": context_message_id}),
            );
        }
        if let Some(callback_data) = options.biz_opaque_callback_data {
            body.insert("biz_opaque_callback_data".to_string(), json!(callback_data));
        }

        GraphJsonRequest {
            method: "POST",
            url: self
                .config
                .graph_url(format!("{}/messages", self.config.phone_number_id)),
            authorization_header: self.config.authorization_header(),
            body: Value::Object(body),
        }
    }

    pub async fn send_message(
        &self,
        recipient: &Recipient,
        message: &WhatsAppMessage,
        options: SendMessageOptions,
    ) -> Result<SendMessageResponse> {
        self.execute_json(self.send_message_request(recipient, message, options))
            .await
    }

    pub async fn send_text_message(
        &self,
        recipient: &Recipient,
        body: impl Into<String>,
        options: SendMessageOptions,
    ) -> Result<SendMessageResponse> {
        let message = WhatsAppMessage::text(body)?;
        self.send_message(recipient, &message, options).await
    }

    pub fn upload_media_request(
        &self,
        bytes: impl Into<Vec<u8>>,
        filename: impl Into<String>,
        mime_type: impl AsRef<str>,
    ) -> MediaUploadRequest {
        let mime_type = normalize_mime_type(mime_type.as_ref());
        MediaUploadRequest {
            method: "POST",
            url: self
                .config
                .graph_url(format!("{}/media", self.config.phone_number_id)),
            authorization_header: self.config.authorization_header(),
            messaging_product: "whatsapp",
            media_type: mime_type,
            filename: filename.into(),
            bytes: bytes.into(),
        }
    }

    pub async fn upload_media(
        &self,
        bytes: impl Into<Vec<u8>>,
        filename: impl Into<String>,
        mime_type: impl AsRef<str>,
    ) -> Result<MediaUploadResponse> {
        let request = self.upload_media_request(bytes, filename, mime_type);
        let part = reqwest::multipart::Part::bytes(request.bytes)
            .file_name(request.filename)
            .mime_str(&request.media_type)
            .map_err(WhatsAppApiError::decode)?;
        let form = reqwest::multipart::Form::new()
            .text("messaging_product", request.messaging_product)
            .text("type", request.media_type)
            .part("file", part);

        let response = self
            .http
            .post(request.url)
            .header(reqwest::header::AUTHORIZATION, request.authorization_header)
            .multipart(form)
            .send()
            .await
            .map_err(WhatsAppApiError::http)?;

        decode_graph_json_response(response.status().as_u16(), &response_text(response).await?)
    }

    pub async fn send_media_message(
        &self,
        recipient: &Recipient,
        message: &WhatsAppMessage,
        options: SendMessageOptions,
    ) -> Result<SendMessageResponse> {
        self.send_message(recipient, message, options).await
    }

    pub fn retrieve_media_request(&self, media_id: impl AsRef<str>) -> GraphJsonRequest {
        GraphJsonRequest {
            method: "GET",
            url: format!(
                "{}?phone_number_id={}",
                self.config.graph_url(media_id.as_ref()),
                self.config.phone_number_id
            ),
            authorization_header: self.config.authorization_header(),
            body: Value::Null,
        }
    }

    pub async fn retrieve_media_metadata(
        &self,
        media_id: impl AsRef<str>,
    ) -> Result<MediaMetadataResponse> {
        self.execute_json(self.retrieve_media_request(media_id))
            .await
    }

    pub async fn fetch_media_bytes(&self, url: impl AsRef<str>) -> Result<Vec<u8>> {
        let response = self
            .http
            .get(url.as_ref())
            .header(
                reqwest::header::AUTHORIZATION,
                self.config.authorization_header(),
            )
            .header(
                reqwest::header::USER_AGENT,
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            )
            .send()
            .await
            .map_err(WhatsAppApiError::http)?;

        let status_code = response.status().as_u16();
        let bytes = response.bytes().await.map_err(WhatsAppApiError::http)?;
        if !(200..300).contains(&status_code) {
            return Err(WhatsAppApiError::GraphApi {
                status_code,
                body: Value::String(String::from_utf8_lossy(&bytes).to_string()),
                meta_error: None,
            });
        }
        Ok(bytes.to_vec())
    }

    async fn execute_json<T: DeserializeOwned>(&self, request: GraphJsonRequest) -> Result<T> {
        let authorization_header = request.authorization_header;
        let response = match request.method {
            "POST" => {
                self.http
                    .post(request.url)
                    .header(reqwest::header::AUTHORIZATION, authorization_header)
                    .json(&request.body)
                    .send()
                    .await
            }
            "DELETE" => {
                let builder = self
                    .http
                    .delete(request.url)
                    .header(reqwest::header::AUTHORIZATION, authorization_header);
                if request.body.is_null() {
                    builder.send().await
                } else {
                    builder.json(&request.body).send().await
                }
            }
            _ => {
                self.http
                    .get(request.url)
                    .header(reqwest::header::AUTHORIZATION, authorization_header)
                    .send()
                    .await
            }
        }
        .map_err(WhatsAppApiError::http)?;

        decode_graph_json_response(response.status().as_u16(), &response_text(response).await?)
    }
}

async fn response_text(response: reqwest::Response) -> Result<String> {
    response.text().await.map_err(WhatsAppApiError::http)
}

fn decode_graph_json_response<T: DeserializeOwned>(status_code: u16, body: &str) -> Result<T> {
    let value = decode_graph_json_value(status_code, body)?;
    serde_json::from_value(value).map_err(WhatsAppApiError::decode)
}

fn decode_graph_json_value(status_code: u16, body: &str) -> Result<Value> {
    let value = if body.trim().is_empty() {
        Value::Object(Map::new())
    } else {
        serde_json::from_str::<Value>(body).map_err(WhatsAppApiError::decode)?
    };

    if !(200..300).contains(&status_code) {
        let meta_error = value
            .get("error")
            .cloned()
            .and_then(|error| serde_json::from_value::<MetaError>(error).ok());
        return Err(WhatsAppApiError::GraphApi {
            status_code,
            body: value,
            meta_error,
        });
    }

    Ok(value)
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SendMessageOptions {
    pub context_message_id: Option<String>,
    pub biz_opaque_callback_data: Option<String>,
}

impl SendMessageOptions {
    pub fn reply_to(context_message_id: impl Into<String>) -> Self {
        Self {
            context_message_id: Some(context_message_id.into()),
            biz_opaque_callback_data: None,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct GraphJsonRequest {
    pub method: &'static str,
    pub url: String,
    authorization_header: String,
    pub body: Value,
}

impl GraphJsonRequest {
    pub fn authorization_header(&self) -> &str {
        &self.authorization_header
    }
}

impl std::fmt::Debug for GraphJsonRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("GraphJsonRequest")
            .field("method", &self.method)
            .field("url", &self.url)
            .field("authorization_header", &"[redacted]")
            .field("body", &self.body)
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct MediaUploadRequest {
    pub method: &'static str,
    pub url: String,
    authorization_header: String,
    pub messaging_product: &'static str,
    pub media_type: String,
    pub filename: String,
    pub bytes: Vec<u8>,
}

impl MediaUploadRequest {
    pub fn authorization_header(&self) -> &str {
        &self.authorization_header
    }
}

impl std::fmt::Debug for MediaUploadRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MediaUploadRequest")
            .field("method", &self.method)
            .field("url", &self.url)
            .field("authorization_header", &"[redacted]")
            .field("messaging_product", &self.messaging_product)
            .field("media_type", &self.media_type)
            .field("filename", &self.filename)
            .field("bytes_len", &self.bytes.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::{AudioMessage, DocumentMessage, ImageMessage, TextMessage, VideoMessage};

    fn client() -> WhatsAppApiClient {
        WhatsAppApiClient::new(
            WhatsAppApiConfig::new("v24.0", "123", "TOKEN").with_app_secret("SECRET"),
        )
        .unwrap()
    }

    #[test]
    fn text_send_request_matches_current_dart_sdk_shape() {
        let request = client().send_message_request(
            &Recipient::phone("456"),
            &TextMessage::new("Hi").unwrap().into(),
            SendMessageOptions::default(),
        );

        assert_eq!(request.method, "POST");
        assert_eq!(request.url, "https://graph.facebook.com/v24.0/123/messages");
        assert_eq!(request.authorization_header(), "Bearer TOKEN");
        assert_eq!(
            request.body,
            json!({
                "messaging_product": "whatsapp",
                "recipient_type": "individual",
                "to": "456",
                "type": "text",
                "text": {"body": "Hi"}
            })
        );
    }

    #[test]
    fn reply_context_matches_current_dart_sdk_shape() {
        let request = client().send_message_request(
            &Recipient::phone("456"),
            &TextMessage::new("Reply").unwrap().into(),
            SendMessageOptions::reply_to("wamid.original"),
        );

        assert_eq!(
            request.body["context"],
            json!({"message_id": "wamid.original"})
        );
    }

    #[test]
    fn media_send_request_shapes_match_gateway_mapping() {
        let cases = [
            (
                ImageMessage::id("media-image")
                    .with_caption("caption")
                    .into(),
                json!({
                    "type": "image",
                    "image": {"id": "media-image", "caption": "caption"}
                }),
            ),
            (
                VideoMessage::id("media-video")
                    .with_caption("caption")
                    .into(),
                json!({
                    "type": "video",
                    "video": {"id": "media-video", "caption": "caption"}
                }),
            ),
            (
                AudioMessage::id("media-audio").with_voice(true).into(),
                json!({
                    "type": "audio",
                    "audio": {"id": "media-audio", "voice": true}
                }),
            ),
            (
                DocumentMessage::id("media-doc")
                    .with_caption("caption")
                    .with_filename("doc.pdf")
                    .into(),
                json!({
                    "type": "document",
                    "document": {
                        "id": "media-doc",
                        "caption": "caption",
                        "filename": "doc.pdf"
                    }
                }),
            ),
        ];

        for (message, expected) in cases {
            let request = client().send_message_request(
                &Recipient::phone("456"),
                &message,
                Default::default(),
            );
            assert_eq!(request.body["type"], expected["type"]);
            assert_eq!(
                request.body[expected["type"].as_str().unwrap()],
                expected[expected["type"].as_str().unwrap()]
            );
        }
    }

    #[test]
    fn media_upload_request_contains_current_dart_sdk_fields() {
        let request = client().upload_media_request([1, 2, 3], "clipboard-image.png", "image/png");

        assert_eq!(request.method, "POST");
        assert_eq!(request.url, "https://graph.facebook.com/v24.0/123/media");
        assert_eq!(request.authorization_header(), "Bearer TOKEN");
        assert_eq!(request.messaging_product, "whatsapp");
        assert_eq!(request.media_type, "image/png");
        assert_eq!(request.filename, "clipboard-image.png");
        assert_eq!(request.bytes, vec![1, 2, 3]);
    }

    #[test]
    fn media_metadata_request_uses_phone_number_query() {
        let request = client().retrieve_media_request("media-1");

        assert_eq!(request.method, "GET");
        assert_eq!(
            request.url,
            "https://graph.facebook.com/v24.0/media-1?phone_number_id=123"
        );
        assert_eq!(request.authorization_header(), "Bearer TOKEN");
    }

    #[test]
    fn request_debug_redacts_authorization() {
        let request = client().send_message_request(
            &Recipient::phone("456"),
            &TextMessage::new("Hi").unwrap().into(),
            SendMessageOptions::default(),
        );

        let debug = format!("{request:?}");
        assert!(debug.contains("[redacted]"));
        assert!(!debug.contains("TOKEN"));
    }

    #[test]
    fn successful_text_send_response_maps_meta_message_id() {
        let response: SendMessageResponse = decode_graph_json_response(
            200,
            r#"{
                "messages": [
                    {"id": "wamid.1"}
                ]
            }"#,
        )
        .unwrap();

        assert_eq!(response.first_message_id(), Some("wamid.1"));
    }

    #[test]
    fn meta_error_response_decodes_structured_error() {
        let error = decode_graph_json_response::<SendMessageResponse>(
            400,
            r#"{
                "error": {
                    "message": "Invalid parameter",
                    "type": "OAuthException",
                    "code": 100,
                    "error_subcode": 2018001,
                    "fbtrace_id": "trace-1"
                }
            }"#,
        )
        .unwrap_err();

        let WhatsAppApiError::GraphApi {
            status_code,
            body,
            meta_error,
        } = error
        else {
            panic!("expected GraphApi error");
        };

        assert_eq!(status_code, 400);
        assert_eq!(body["error"]["message"], "Invalid parameter");
        let meta_error = meta_error.unwrap();
        assert_eq!(meta_error.message.as_deref(), Some("Invalid parameter"));
        assert_eq!(meta_error.kind.as_deref(), Some("OAuthException"));
        assert_eq!(meta_error.code, Some(100));
        assert_eq!(meta_error.error_subcode, Some(2018001));
        assert_eq!(meta_error.fbtrace_id.as_deref(), Some("trace-1"));
    }

    #[test]
    fn invalid_success_json_maps_to_decode_error() {
        assert!(matches!(
            decode_graph_json_response::<SendMessageResponse>(200, "not json"),
            Err(WhatsAppApiError::Decode { .. })
        ));
    }
}
