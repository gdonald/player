use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use player_types::{ErrorResponse, FieldErrors};
use serde_json::json;

#[derive(Debug)]
pub enum AppError {
    Unauthorized,
    NotFound,
    BadRequest(String),
    Invalid(ErrorResponse),
    InvalidMessages(Vec<String>),
    Internal(anyhow::Error),
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn invalid(errors: FieldErrors, message: &str) -> Self {
        AppError::Invalid(ErrorResponse {
            errors,
            message: message.to_string(),
        })
    }

    pub fn field(field: &str, error: &str, message: &str) -> Self {
        let mut errors = FieldErrors::new();
        errors.insert(field.to_string(), vec![error.to_string()]);

        AppError::invalid(errors, message)
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Unauthorized => write!(formatter, "unauthorized"),
            AppError::NotFound => write!(formatter, "not found"),
            AppError::BadRequest(message) => write!(formatter, "bad request: {message}"),
            AppError::Invalid(body) => {
                let mut details = Vec::new();

                for (field, messages) in &body.errors {
                    for message in messages {
                        details.push(format!("{field} {message}"));
                    }
                }

                write!(formatter, "{}: {}", body.message, details.join(", "))
            }
            AppError::InvalidMessages(messages) => write!(formatter, "{}", messages.join(", ")),
            AppError::Internal(error) => write!(formatter, "{error:#}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        AppError::Internal(error.into())
    }
}

impl From<anyhow::Error> for AppError {
    fn from(error: anyhow::Error) -> Self {
        AppError::Internal(error)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, Json(json!({}))).into_response(),
            AppError::NotFound => {
                (StatusCode::NOT_FOUND, Json(json!({"message": "Not found"}))).into_response()
            }
            AppError::BadRequest(message) => {
                (StatusCode::BAD_REQUEST, Json(json!({"message": message}))).into_response()
            }
            AppError::Invalid(body) => {
                (StatusCode::UNPROCESSABLE_ENTITY, Json(body)).into_response()
            }
            AppError::InvalidMessages(messages) => {
                (StatusCode::UNPROCESSABLE_ENTITY, Json(messages)).into_response()
            }
            AppError::Internal(error) => {
                tracing::error!("{error:#}");

                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"message": "Internal server error"})),
                )
                    .into_response()
            }
        }
    }
}

/// Collects Rails-style `{field: [messages]}` validation errors.
#[derive(Debug, Default)]
pub struct Validation {
    errors: FieldErrors,
}

impl Validation {
    pub fn add(&mut self, field: &str, message: &str) {
        self.errors
            .entry(field.to_string())
            .or_default()
            .push(message.to_string());
    }

    pub fn require(&mut self, field: &str, value: Option<&str>) {
        if value.is_none_or(|text| text.trim().is_empty()) {
            self.add(field, "can't be blank");
        }
    }

    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn into_result(self, message: &str) -> AppResult<()> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(AppError::invalid(self.errors, message))
        }
    }
}

#[cfg(test)]
mod tests {
    use http_body_util::BodyExt;

    use super::*;

    async fn render(error: AppError) -> (StatusCode, serde_json::Value) {
        let response = error.into_response();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();

        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn unauthorized_is_an_empty_object() {
        assert_eq!(
            render(AppError::Unauthorized).await,
            (StatusCode::UNAUTHORIZED, json!({}))
        );
    }

    #[tokio::test]
    async fn not_found_is_json() {
        assert_eq!(
            render(AppError::NotFound).await,
            (StatusCode::NOT_FOUND, json!({"message": "Not found"}))
        );
    }

    #[tokio::test]
    async fn bad_request_carries_its_message() {
        assert_eq!(
            render(AppError::BadRequest("bad body".to_string())).await,
            (StatusCode::BAD_REQUEST, json!({"message": "bad body"}))
        );
    }

    #[tokio::test]
    async fn invalid_messages_render_as_an_array() {
        assert_eq!(
            render(AppError::InvalidMessages(vec![
                "Mp3 must exist".to_string()
            ]))
            .await,
            (StatusCode::UNPROCESSABLE_ENTITY, json!(["Mp3 must exist"]))
        );
    }

    #[tokio::test]
    async fn database_errors_become_internal_errors() {
        assert_eq!(
            render(AppError::from(sqlx::Error::RowNotFound)).await,
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"message": "Internal server error"})
            )
        );
    }

    #[tokio::test]
    async fn anyhow_errors_become_internal_errors() {
        let (status, _) = render(AppError::from(anyhow::anyhow!("broken"))).await;

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn validation_collects_messages_per_field() {
        let mut validation = Validation::default();
        validation.require("name", Some("  "));
        validation.add("name", "has already been taken");

        assert!(!validation.is_empty());
        assert_eq!(
            validation.into_result("Failed").unwrap_err().to_string(),
            "Failed: name can't be blank, name has already been taken"
        );
    }

    #[test]
    fn display_describes_each_error() {
        assert_eq!(AppError::Unauthorized.to_string(), "unauthorized");
        assert_eq!(AppError::NotFound.to_string(), "not found");
        assert_eq!(
            AppError::BadRequest("no body".to_string()).to_string(),
            "bad request: no body"
        );
        assert_eq!(
            AppError::field("name", "can't be blank", "Failed to save").to_string(),
            "Failed to save: name can't be blank"
        );
        assert_eq!(
            AppError::InvalidMessages(vec!["a".to_string(), "b".to_string()]).to_string(),
            "a, b"
        );
        assert_eq!(
            AppError::Internal(anyhow::anyhow!("broken")).to_string(),
            "broken"
        );
    }

    #[test]
    fn validation_without_errors_is_ok() {
        let mut validation = Validation::default();
        validation.require("name", Some("Mix"));

        assert!(validation.into_result("Failed").is_ok());
    }
}
