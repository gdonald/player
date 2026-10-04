use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

/// A request body accepted either wrapped in a key (`{"mp3": {...}}`) or bare.
pub trait Wrapped {
    const KEY: &'static str;
}

pub fn wrap<T: Serialize + Wrapped>(params: &T) -> Value {
    let mut body = serde_json::Map::new();
    body.insert(
        T::KEY.to_string(),
        serde_json::to_value(params).unwrap_or(Value::Null),
    );

    Value::Object(body)
}

/// Picks the wrapped object when the key is present, otherwise the whole body.
pub fn unwrap<T: Wrapped>(body: Value) -> Value {
    match body {
        Value::Object(mut map) if map.contains_key(T::KEY) => {
            map.remove(T::KEY).unwrap_or(Value::Null)
        }
        other => other,
    }
}

/// An id sent as a JSON number or a numeric string. Anything else is `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct FlexId(pub Option<i64>);

impl<'de> Deserialize<'de> for FlexId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;

        let id = match value {
            Value::Number(number) => number.as_i64(),
            Value::String(text) => text.trim().parse().ok(),
            _ => None,
        };

        Ok(FlexId(id))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Mp3 {
    pub id: i64,
    pub title: String,
    pub track: Option<i32>,
    pub artist_name: String,
    pub album_name: String,
    pub length: Option<i32>,
    pub file_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mp3sResponse {
    pub mp3s: Vec<Mp3>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mp3Response {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub mp3: Mp3,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Mp3Params {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub artist: Option<String>,
    #[serde(default)]
    pub album: Option<String>,
    #[serde(default)]
    pub track: Option<Value>,
}

impl Wrapped for Mp3Params {
    const KEY: &'static str = "mp3";
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CountsResponse {
    pub mp3s_count: i64,
    pub playlists_count: i64,
    pub queued_mp3s_count: i64,
    pub sources_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaylistListItem {
    pub id: i64,
    pub name: String,
    pub mp3s_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaylistsResponse {
    pub playlists: Vec<PlaylistListItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaylistSummary {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaylistResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub playlist: PlaylistSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaylistMp3Attributes {
    pub mp3_id: FlexId,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaylistParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub playlist_mp3s_attributes: Option<Vec<PlaylistMp3Attributes>>,
}

impl Wrapped for PlaylistParams {
    const KEY: &'static str = "playlist";
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaylistMp3 {
    pub id: i64,
    pub first: bool,
    pub last: bool,
    pub mp3: Mp3,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaylistMp3sResponse {
    pub playlist_mp3s: Vec<PlaylistMp3>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaylistMp3MoveParams {
    pub position: i64,
}

impl Wrapped for PlaylistMp3MoveParams {
    const KEY: &'static str = "playlist_mp3";
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueuedMp3 {
    pub id: i64,
    pub position: i32,
    pub mp3: Mp3,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueuedMp3sResponse {
    pub queued_mp3s: Vec<QueuedMp3>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueuedMp3Params {
    pub mp3_id: FlexId,
}

impl Wrapped for QueuedMp3Params {
    const KEY: &'static str = "queued_mp3";
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceListItem {
    pub id: i64,
    pub path: String,
    pub mp3s_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourcesResponse {
    pub sources: Vec<SourceListItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSummary {
    pub id: i64,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub source: SourceSummary,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceParams {
    #[serde(default)]
    pub path: Option<String>,
}

impl Wrapped for SourceParams {
    const KEY: &'static str = "source";
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionParams {
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
}

impl Wrapped for SessionParams {
    const KEY: &'static str = "session";
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageResponse {
    pub message: String,
}

pub type FieldErrors = BTreeMap<String, Vec<String>>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub errors: FieldErrors,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn sample_mp3() -> Mp3 {
        Mp3 {
            id: 7,
            title: "Song".to_string(),
            track: Some(3),
            artist_name: "Band".to_string(),
            album_name: "Record".to_string(),
            length: Some(185),
            file_hash: Some("ab12".to_string()),
        }
    }

    #[test]
    fn wrap_nests_params_under_their_key() {
        let params = SourceParams {
            path: Some("/music".to_string()),
        };

        assert_eq!(wrap(&params), json!({"source": {"path": "/music"}}));
    }

    #[test]
    fn unwrap_takes_the_object_under_the_key() {
        let body = json!({"session": {"username": "gd"}});

        assert_eq!(unwrap::<SessionParams>(body), json!({"username": "gd"}));
    }

    #[test]
    fn unwrap_keeps_a_bare_body() {
        let body = json!({"username": "gd"});

        assert_eq!(unwrap::<SessionParams>(body.clone()), body);
    }

    #[test]
    fn unwrap_keeps_a_non_object_body() {
        assert_eq!(unwrap::<SessionParams>(Value::Null), Value::Null);
    }

    #[test]
    fn flex_id_reads_a_number() {
        let params: QueuedMp3Params = serde_json::from_value(json!({"mp3_id": 12})).unwrap();

        assert_eq!(params.mp3_id, FlexId(Some(12)));
    }

    #[test]
    fn flex_id_reads_a_numeric_string() {
        let params: QueuedMp3Params = serde_json::from_value(json!({"mp3_id": " 12"})).unwrap();

        assert_eq!(params.mp3_id, FlexId(Some(12)));
    }

    #[test]
    fn flex_id_is_none_for_text() {
        let params: QueuedMp3Params = serde_json::from_value(json!({"mp3_id": "abc"})).unwrap();

        assert_eq!(params.mp3_id, FlexId(None));
    }

    #[test]
    fn flex_id_is_none_for_other_json() {
        let params: QueuedMp3Params = serde_json::from_value(json!({"mp3_id": [1]})).unwrap();

        assert_eq!(params.mp3_id, FlexId(None));
    }

    #[test]
    fn flex_id_serializes_as_a_number() {
        assert_eq!(serde_json::to_value(FlexId(Some(4))).unwrap(), json!(4));
    }

    #[test]
    fn move_params_wrap_in_the_playlist_mp3_key() {
        assert_eq!(
            wrap(&PlaylistMp3MoveParams { position: 3 }),
            json!({"playlist_mp3": {"position": 3}})
        );
    }

    #[test]
    fn mp3_list_matches_the_index_shape() {
        let response = Mp3sResponse {
            mp3s: vec![sample_mp3()],
        };

        assert_eq!(
            serde_json::to_value(&response).unwrap(),
            json!({"mp3s": [{
                "id": 7, "title": "Song", "track": 3, "artist_name": "Band",
                "album_name": "Record", "length": 185, "file_hash": "ab12"
            }]})
        );
    }

    #[test]
    fn show_response_omits_a_missing_message() {
        let response = Mp3Response {
            message: None,
            mp3: sample_mp3(),
        };

        let value = serde_json::to_value(&response).unwrap();

        assert!(value.get("message").is_none());
    }

    #[test]
    fn show_response_includes_a_present_message() {
        let response = SourceResponse {
            message: Some("Source updated".to_string()),
            source: SourceSummary {
                id: 1,
                path: "/music".to_string(),
            },
        };

        assert_eq!(
            serde_json::to_value(&response).unwrap(),
            json!({"message": "Source updated", "source": {"id": 1, "path": "/music"}})
        );
    }

    #[test]
    fn playlist_params_read_string_and_number_mp3_ids() {
        let body =
            json!({"name": "Mix", "playlist_mp3s_attributes": [{"mp3_id": "3"}, {"mp3_id": 4}]});

        let params: PlaylistParams = serde_json::from_value(body).unwrap();

        assert_eq!(
            params,
            PlaylistParams {
                name: Some("Mix".to_string()),
                playlist_mp3s_attributes: Some(vec![
                    PlaylistMp3Attributes {
                        mp3_id: FlexId(Some(3))
                    },
                    PlaylistMp3Attributes {
                        mp3_id: FlexId(Some(4))
                    },
                ]),
            }
        );
    }

    #[test]
    fn playlist_params_omit_missing_fields_when_serialized() {
        assert_eq!(
            serde_json::to_value(PlaylistParams::default()).unwrap(),
            json!({})
        );
    }

    #[test]
    fn missing_session_fields_default_to_empty() {
        let params: SessionParams = serde_json::from_value(json!({})).unwrap();

        assert_eq!(params, SessionParams::default());
    }

    #[test]
    fn error_response_matches_the_errors_shape() {
        let mut errors = FieldErrors::new();
        errors.insert("name".to_string(), vec!["can't be blank".to_string()]);

        let response = ErrorResponse {
            errors,
            message: "Failed to create playlist".to_string(),
        };

        assert_eq!(
            serde_json::to_value(&response).unwrap(),
            json!({"errors": {"name": ["can't be blank"]}, "message": "Failed to create playlist"})
        );
    }

    #[test]
    fn counts_round_trip() {
        let counts = CountsResponse {
            mp3s_count: 1,
            playlists_count: 2,
            queued_mp3s_count: 3,
            sources_count: 4,
        };

        let value = serde_json::to_value(counts).unwrap();

        assert_eq!(
            serde_json::from_value::<CountsResponse>(value).unwrap(),
            counts
        );
    }
}
