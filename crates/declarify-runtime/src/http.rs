//! Shared, resource-agnostic HTTP building blocks for generated REST
//! handlers and the generated typed client. See
//! `docs/05-rest-client-cli.md`.

use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Json, Response};
use serde::{Deserialize, Serialize};

/// Query parameters accepted by every generated `list` route. Shared across
/// resources since `docs/01-resource-model.md`'s `PageRequest` shape never
/// varies per resource.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ListQuery {
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

/// The stable REST error envelope ("Errors" in `docs/05-rest-client-cli.md`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiErrorBody {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub details: Option<serde_json::Value>,
}

/// Wraps a `declarify_core::Error` so a generated handler can return it
/// directly as an axum response, with the exact status/code mapping
/// `docs/05-rest-client-cli.md` requires.
#[derive(Debug)]
pub struct ApiError(pub declarify_core::Error);

impl From<declarify_core::Error> for ApiError {
    fn from(err: declarify_core::Error) -> Self {
        ApiError(err)
    }
}

impl ApiError {
    pub fn code(&self) -> &'static str {
        match &self.0 {
            declarify_core::Error::InvalidArgument(_) => "invalid_argument",
            declarify_core::Error::NotFound => "not_found",
            declarify_core::Error::Conflict => "conflict",
            declarify_core::Error::Internal(_) => "internal",
        }
    }

    pub fn status(&self) -> StatusCode {
        match &self.0 {
            declarify_core::Error::InvalidArgument(_) => StatusCode::BAD_REQUEST,
            declarify_core::Error::NotFound => StatusCode::NOT_FOUND,
            declarify_core::Error::Conflict => StatusCode::CONFLICT,
            declarify_core::Error::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = ApiErrorBody {
            code: self.code().to_string(),
            message: self.0.to_string(),
            details: None,
        };
        (self.status(), Json(body)).into_response()
    }
}

/// Parses the canonical `If-Match: "<revision>"` precondition header
/// required on replace-spec/update-status/delete. Missing or malformed
/// headers are `invalid_argument`, exactly as `docs/05-rest-client-cli.md`
/// specifies.
pub fn require_if_match(headers: &HeaderMap) -> Result<u64, ApiError> {
    let raw = headers
        .get(axum::http::header::IF_MATCH)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| {
            ApiError(declarify_core::Error::InvalidArgument(
                "missing required If-Match header".to_string(),
            ))
        })?;
    raw.trim().trim_matches('"').parse::<u64>().map_err(|_| {
        ApiError(declarify_core::Error::InvalidArgument(format!(
            "invalid If-Match header: {raw}"
        )))
    })
}

/// Errors from the generated typed client: kept separate from the server's
/// `declarify_core::Error` because a client-side failure can also be a
/// transport or decoding problem that never reached the server at all
/// (`docs/05-rest-client-cli.md`: "preserves structured API errors
/// separately from transport and decoding errors").
#[derive(Debug)]
pub enum ClientError {
    /// The server returned a structured API error.
    Api { code: String, message: String },
    /// The request could not be sent, or no response was received.
    Transport(reqwest::Error),
    /// A response body could not be decoded as the expected JSON shape.
    Decode(reqwest::Error),
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientError::Api { code, message } => write!(f, "api error [{code}]: {message}"),
            ClientError::Transport(err) => write!(f, "transport error: {err}"),
            ClientError::Decode(err) => write!(f, "decode error: {err}"),
        }
    }
}

impl std::error::Error for ClientError {}

/// Parses a successful (2xx) response as `T`, or a non-2xx response as the
/// stable API error envelope. Shared by every generated client method that
/// returns a body.
pub async fn parse_json_response<T>(response: reqwest::Response) -> Result<T, ClientError>
where
    T: for<'de> Deserialize<'de>,
{
    if response.status().is_success() {
        response.json::<T>().await.map_err(ClientError::Decode)
    } else {
        parse_error_body(response).await
    }
}

/// Expects an empty successful response (e.g. `204 No Content` from
/// delete), or a non-2xx response as the stable API error envelope.
pub async fn expect_no_content(response: reqwest::Response) -> Result<(), ClientError> {
    if response.status().is_success() {
        Ok(())
    } else {
        parse_error_body(response).await
    }
}

async fn parse_error_body<T>(response: reqwest::Response) -> Result<T, ClientError> {
    match response.json::<ApiErrorBody>().await {
        Ok(body) => Err(ClientError::Api {
            code: body.code,
            message: body.message,
        }),
        Err(err) => Err(ClientError::Decode(err)),
    }
}
