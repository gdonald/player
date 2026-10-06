#![allow(dead_code)]

use std::path::Path;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use axum::response::Response;
use http_body_util::BodyExt;
use player_server::app::{self, App, AppState};
use player_server::users::{self, NewUser};
use player_server::{db, library};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

pub const USERNAME: &str = "gd";
pub const PASSWORD: &str = "open-sesame";

pub struct TestApp {
    pub state: AppState,
    app: App,
    cookie: Option<String>,
}

impl TestApp {
    pub async fn new(pool: PgPool) -> Self {
        Self::with_web_root(pool, None).await
    }

    pub async fn with_web_root(pool: PgPool, web_root: Option<&Path>) -> Self {
        db::migrate(&pool).await.unwrap();

        users::create(
            &pool,
            NewUser {
                username: USERNAME,
                password: PASSWORD,
                password_confirmation: PASSWORD,
            },
            4,
        )
        .await
        .unwrap();

        let state = AppState::new(pool);

        TestApp {
            app: app::build(state.clone(), web_root),
            state,
            cookie: None,
        }
    }

    pub async fn logged_in(pool: PgPool) -> Self {
        let mut test_app = Self::new(pool).await;
        let (status, _) = test_app
            .send(
                Method::POST,
                "/api/sessions",
                Some(serde_json::json!({"session": {"username": USERNAME, "password": PASSWORD}})),
            )
            .await;
        assert_eq!(status, StatusCode::OK);

        test_app
    }

    pub fn pool(&self) -> &PgPool {
        &self.state.pool
    }

    pub fn cookie(&self) -> Option<&str> {
        self.cookie.as_deref()
    }

    pub fn set_cookie(&mut self, cookie: Option<String>) {
        self.cookie = cookie;
    }

    pub async fn raw(&mut self, mut request: Request<Body>) -> Response {
        if let Some(cookie) = &self.cookie {
            request
                .headers_mut()
                .insert(header::COOKIE, cookie.parse().unwrap());
        }

        let response = self.app.clone().oneshot(request).await.unwrap();

        if let Some(set_cookie) = response.headers().get(header::SET_COOKIE) {
            let pair = set_cookie.to_str().unwrap().split(';').next().unwrap();
            self.cookie = Some(pair.to_string());
        }

        response
    }

    pub async fn send_bytes(
        &mut self,
        method: Method,
        uri: &str,
        body: Vec<u8>,
    ) -> (StatusCode, Value) {
        let request = Request::builder()
            .method(method)
            .uri(uri)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .unwrap();

        let response = self.raw(request).await;
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();

        let value = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes)
                .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).to_string()))
        };

        (status, value)
    }

    pub async fn send(
        &mut self,
        method: Method,
        uri: &str,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let bytes = body
            .map(|value| value.to_string().into_bytes())
            .unwrap_or_default();

        self.send_bytes(method, uri, bytes).await
    }

    pub async fn get(&mut self, uri: &str) -> (StatusCode, Value) {
        self.send(Method::GET, uri, None).await
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Mp3Seed<'a> {
    pub artist: &'a str,
    pub album: &'a str,
    pub title: &'a str,
    pub track: i32,
    pub length: i32,
    pub filepath: &'a str,
}

impl Default for Mp3Seed<'_> {
    fn default() -> Self {
        Mp3Seed {
            artist: "Band",
            album: "Record",
            title: "Song",
            track: 1,
            length: 100,
            filepath: "/music/song.mp3",
        }
    }
}

pub async fn source_id(pool: &PgPool, path: &str) -> i64 {
    sqlx::query_scalar(
        "INSERT INTO sources (path) VALUES ($1) \
         ON CONFLICT (path) DO UPDATE SET path = EXCLUDED.path RETURNING id",
    )
    .bind(path)
    .fetch_one(pool)
    .await
    .unwrap()
}

pub async fn insert_mp3(pool: &PgPool, seed: Mp3Seed<'_>) -> i64 {
    let source = source_id(pool, "/music").await;
    let artist = library::find_or_create_artist(pool, seed.artist)
        .await
        .unwrap();
    let album = library::find_or_create_album(pool, artist, seed.album)
        .await
        .unwrap();

    sqlx::query_scalar(
        "INSERT INTO mp3s (source_id, artist_id, album_id, filepath, title, track, length, file_hash) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, 'hash') RETURNING id",
    )
    .bind(source)
    .bind(artist)
    .bind(album)
    .bind(seed.filepath)
    .bind(seed.title)
    .bind(seed.track)
    .bind(seed.length)
    .fetch_one(pool)
    .await
    .unwrap()
}

pub fn titles(body: &Value, list_key: &str) -> Vec<String> {
    body[list_key]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| {
            item.get("mp3")
                .unwrap_or(item)
                .get("title")
                .unwrap()
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect()
}
