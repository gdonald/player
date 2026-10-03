use axum::body::Bytes;
use axum::extract::{FromRequest, Request};
use player_types::{Wrapped, unwrap};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::AppError;

/// A JSON body accepted either wrapped in its key or bare.
#[derive(Debug)]
pub struct Params<T>(pub T);

impl<T, S> FromRequest<S> for Params<T>
where
    T: DeserializeOwned + Wrapped,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        let bytes = Bytes::from_request(request, state)
            .await
            .map_err(|rejection| AppError::BadRequest(rejection.body_text()))?;

        parse(&bytes).map(Params)
    }
}

pub fn parse<T: DeserializeOwned + Wrapped>(bytes: &[u8]) -> Result<T, AppError> {
    let body = if bytes.iter().all(u8::is_ascii_whitespace) {
        Value::Object(serde_json::Map::new())
    } else {
        serde_json::from_slice(bytes).map_err(|error| AppError::BadRequest(error.to_string()))?
    };

    serde_json::from_value(unwrap::<T>(body))
        .map_err(|error| AppError::BadRequest(error.to_string()))
}

#[cfg(test)]
mod tests {
    use player_types::{QueuedMp3Params, SessionParams, SourceParams};

    use super::*;

    #[test]
    fn wrapped_body_is_unwrapped() {
        let params: SessionParams =
            parse(br#"{"session":{"username":"gd","password":"pw"}}"#).unwrap();

        assert_eq!(params.username, "gd");
    }

    #[test]
    fn bare_body_is_accepted() {
        let params: SessionParams = parse(br#"{"username":"gd","password":"pw"}"#).unwrap();

        assert_eq!(params.password, "pw");
    }

    #[test]
    fn empty_body_is_an_empty_object() {
        let params: SourceParams = parse(b" ").unwrap();

        assert_eq!(params.path, None);
    }

    #[test]
    fn malformed_json_is_a_bad_request() {
        assert!(matches!(
            parse::<SourceParams>(b"{"),
            Err(AppError::BadRequest(_))
        ));
    }

    #[test]
    fn missing_required_field_is_a_bad_request() {
        assert!(matches!(
            parse::<QueuedMp3Params>(b"{}"),
            Err(AppError::BadRequest(_))
        ));
    }
}
