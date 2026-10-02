use gloo_net::http::{Request, RequestBuilder, Response};
use player_types::{ErrorResponse, FieldErrors, SessionParams, wrap};
use serde::de::DeserializeOwned;
use serde_json::Value;

#[derive(Debug, Clone)]
pub enum ApiError {
    Unauthorized,
    Invalid(Value),
    Failed(String),
}

impl ApiError {
    /// The `{errors, message}` body of a 422, when it has that shape.
    pub fn field_errors(&self) -> Option<ErrorResponse> {
        match self {
            ApiError::Invalid(body) => serde_json::from_value(body.clone()).ok(),
            _ => None,
        }
    }

    pub fn message(&self) -> String {
        match self {
            ApiError::Unauthorized => "Please log in".to_string(),
            ApiError::Invalid(Value::Array(messages)) => messages
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", "),
            ApiError::Invalid(body) => body["message"]
                .as_str()
                .unwrap_or("Request failed")
                .to_string(),
            ApiError::Failed(text) => text.clone(),
        }
    }
}

pub fn errors_for(errors: Option<&FieldErrors>, field: &str) -> String {
    errors
        .and_then(|errors| errors.get(field))
        .map(|messages| messages.join(", "))
        .unwrap_or_default()
}

async fn send(builder: RequestBuilder, body: Option<&Value>) -> Result<Response, ApiError> {
    let request = match body {
        Some(body) => builder.json(body),
        None => builder.build(),
    }
    .map_err(|error| ApiError::Failed(error.to_string()))?;

    let response = request
        .send()
        .await
        .map_err(|error| ApiError::Failed(error.to_string()))?;

    match response.status() {
        200..=299 => Ok(response),
        401 => Err(ApiError::Unauthorized),
        422 => Err(ApiError::Invalid(
            response.json::<Value>().await.unwrap_or(Value::Null),
        )),
        status => Err(ApiError::Failed(format!("Request failed ({status})"))),
    }
}

async fn parse<T: DeserializeOwned>(response: Response) -> Result<T, ApiError> {
    response
        .json::<T>()
        .await
        .map_err(|error| ApiError::Failed(error.to_string()))
}

pub async fn get<T: DeserializeOwned>(path: &str) -> Result<T, ApiError> {
    parse(send(Request::get(path), None).await?).await
}

pub async fn post<T: DeserializeOwned>(path: &str, body: &Value) -> Result<T, ApiError> {
    parse(send(Request::post(path), Some(body)).await?).await
}

pub async fn put<T: DeserializeOwned>(path: &str, body: &Value) -> Result<T, ApiError> {
    parse(send(Request::put(path), Some(body)).await?).await
}

pub async fn delete<T: DeserializeOwned>(path: &str) -> Result<T, ApiError> {
    parse(send(Request::delete(path), None).await?).await
}

/// A GET whose body is not needed, such as starting a scan.
pub async fn touch(path: &str) -> Result<(), ApiError> {
    send(Request::get(path), None).await.map(|_| ())
}

pub async fn active() -> bool {
    touch("/api/sessions/active").await.is_ok()
}

pub async fn login(username: String, password: String) -> bool {
    let body = wrap(&SessionParams { username, password });

    post::<Value>("/api/sessions", &body).await.is_ok()
}
