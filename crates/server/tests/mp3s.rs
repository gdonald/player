mod common;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use common::{Mp3Seed, TestApp, insert_mp3, titles};
use http_body_util::BodyExt;
use player_server::fixtures::{FixtureTags, write_tagged_mp3};
use player_server::tags;
use serde_json::json;
use sqlx::PgPool;

async fn library(pool: PgPool) -> TestApp {
    let app = TestApp::logged_in(pool).await;

    for seed in [
        Mp3Seed {
            artist: "Beta Band",
            album: "Second",
            title: "Yellow Song",
            track: 2,
            length: 1,
            filepath: "/m/1.mp3",
        },
        Mp3Seed {
            artist: "Alpha",
            album: "First",
            title: "Red Song",
            track: 2,
            length: 2,
            filepath: "/m/2.mp3",
        },
        Mp3Seed {
            artist: "Alpha",
            album: "First",
            title: "Blue Song",
            track: 1,
            length: 3,
            filepath: "/m/3.mp3",
        },
        Mp3Seed {
            artist: "Alpha",
            album: "Yellow Submarine",
            title: "A_b",
            track: 5,
            length: 4,
            filepath: "/m/4.mp3",
        },
        Mp3Seed {
            artist: "Alpha",
            album: "Yellow Submarine",
            title: "Axb",
            track: 6,
            length: 5,
            filepath: "/m/5.mp3",
        },
    ] {
        insert_mp3(app.pool(), seed).await;
    }

    app
}

async fn listed(app: &mut TestApp, uri: &str) -> Vec<String> {
    let (status, body) = app.get(uri).await;
    assert_eq!(status, StatusCode::OK);

    titles(&body, "mp3s")
}

#[sqlx::test]
async fn index_orders_by_artist_album_track_title(pool: PgPool) {
    let mut app = library(pool).await;

    assert_eq!(
        listed(&mut app, "/api/mp3s.json").await,
        vec!["Blue Song", "Red Song", "A_b", "Axb", "Yellow Song"]
    );
}

#[sqlx::test]
async fn index_sorts_by_the_requested_column(pool: PgPool) {
    let mut app = library(pool).await;

    assert_eq!(
        listed(&mut app, "/api/mp3s?sort=title_desc").await,
        vec!["Yellow Song", "Red Song", "Blue Song", "Axb", "A_b"]
    );
    assert_eq!(
        listed(&mut app, "/api/mp3s?sort=track_desc").await,
        vec!["Axb", "A_b", "Yellow Song", "Red Song", "Blue Song"]
    );
}

#[sqlx::test]
async fn index_rows_have_the_ios_client_fields(pool: PgPool) {
    let mut app = TestApp::logged_in(pool).await;
    let id = insert_mp3(app.pool(), Mp3Seed::default()).await;

    assert_eq!(
        app.get("/api/mp3s").await,
        (
            StatusCode::OK,
            json!({"mp3s": [{
                "id": id, "title": "Song", "track": 1, "artist_name": "Band",
                "album_name": "Record", "length": 100, "file_hash": "hash"
            }]})
        )
    );
}

#[sqlx::test]
async fn search_requires_every_word_within_one_field(pool: PgPool) {
    let mut app = library(pool).await;

    assert_eq!(
        listed(&mut app, "/api/mp3s/search?q=yellow%20song").await,
        vec!["Yellow Song"]
    );
}

#[sqlx::test]
async fn search_matches_any_field(pool: PgPool) {
    let mut app = library(pool).await;

    assert_eq!(
        listed(&mut app, "/api/mp3s/search?q=yellow").await,
        vec!["A_b", "Axb", "Yellow Song"]
    );
}

#[sqlx::test]
async fn search_keeps_quoted_phrases_together(pool: PgPool) {
    let mut app = library(pool).await;

    assert_eq!(
        listed(&mut app, "/api/mp3s/search?q=%22yellow%20sub%22").await,
        vec!["A_b", "Axb"]
    );
}

#[sqlx::test]
async fn search_treats_underscore_literally(pool: PgPool) {
    let mut app = library(pool).await;

    assert_eq!(
        listed(&mut app, "/api/mp3s/search?q=a_b").await,
        vec!["A_b"]
    );
}

#[sqlx::test]
async fn search_filters_by_artist_and_album(pool: PgPool) {
    let mut app = library(pool).await;

    assert_eq!(
        listed(&mut app, "/api/mp3s/search?q=artist:%22alpha%22").await,
        vec!["Blue Song", "Red Song", "A_b", "Axb"]
    );
    assert_eq!(
        listed(
            &mut app,
            "/api/mp3s/search?q=artist:%22Alpha%22%20album:%22First%22%20red"
        )
        .await,
        vec!["Red Song"]
    );
}

#[sqlx::test]
async fn empty_search_lists_everything_in_the_requested_order(pool: PgPool) {
    let mut app = library(pool).await;

    assert_eq!(
        listed(&mut app, "/api/mp3s/search?q=&sort=artist_desc").await,
        vec!["Yellow Song", "Red Song", "Blue Song", "A_b", "Axb"]
    );
}

#[sqlx::test]
async fn show_returns_one_mp3(pool: PgPool) {
    let mut app = TestApp::logged_in(pool).await;
    let id = insert_mp3(app.pool(), Mp3Seed::default()).await;

    let (status, body) = app.get(&format!("/api/mp3s/{id}")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["mp3"]["title"], "Song");
    assert!(body.get("message").is_none());
}

#[sqlx::test]
async fn show_of_a_missing_mp3_is_404(pool: PgPool) {
    let mut app = TestApp::logged_in(pool).await;

    assert_eq!(app.get("/api/mp3s/9").await.0, StatusCode::NOT_FOUND);
}

struct TaggedFile {
    _directory: tempfile::TempDir,
    path: String,
}

fn tagged_file() -> TaggedFile {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("song.mp3");

    write_tagged_mp3(
        &path,
        1,
        &FixtureTags {
            title: Some("Song"),
            artist: Some("Band"),
            album: Some("Record"),
            ..FixtureTags::default()
        },
    )
    .unwrap();

    TaggedFile {
        path: path.to_string_lossy().to_string(),
        _directory: directory,
    }
}

#[sqlx::test]
async fn update_saves_the_row_and_the_file_tags(pool: PgPool) {
    let mut app = TestApp::logged_in(pool).await;
    let file = tagged_file();
    let id = insert_mp3(
        app.pool(),
        Mp3Seed {
            filepath: &file.path,
            ..Mp3Seed::default()
        },
    )
    .await;

    let (status, body) = app
        .send(
            Method::PUT,
            &format!("/api/mp3s/{id}"),
            Some(
                json!({"mp3": {"title": "New", "artist": "Other", "album": "Album", "track": "7"}}),
            ),
        )
        .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["message"], "MP3 updated");
    assert_eq!(
        (
            &body["mp3"]["title"],
            &body["mp3"]["artist_name"],
            &body["mp3"]["album_name"],
            &body["mp3"]["track"]
        ),
        (&json!("New"), &json!("Other"), &json!("Album"), &json!(7))
    );

    let written = tags::read(std::path::Path::new(&file.path)).unwrap();
    assert_eq!(
        (
            written.title.as_deref(),
            written.artist.as_deref(),
            written.album.as_deref(),
            written.track
        ),
        (Some("New"), Some("Other"), Some("Album"), Some(7))
    );
}

#[sqlx::test]
async fn patch_updates_like_put(pool: PgPool) {
    let mut app = TestApp::logged_in(pool).await;
    let file = tagged_file();
    let id = insert_mp3(
        app.pool(),
        Mp3Seed {
            filepath: &file.path,
            ..Mp3Seed::default()
        },
    )
    .await;

    let (status, _) = app
        .send(
            Method::PATCH,
            &format!("/api/mp3s/{id}"),
            Some(json!({"title": "Bare", "artist": "Band", "album": "Record", "track": null})),
        )
        .await;

    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test]
async fn update_reports_each_invalid_field_and_leaves_the_file(pool: PgPool) {
    let mut app = TestApp::logged_in(pool).await;
    let file = tagged_file();
    let id = insert_mp3(
        app.pool(),
        Mp3Seed {
            filepath: &file.path,
            ..Mp3Seed::default()
        },
    )
    .await;

    let response = app
        .send(
            Method::PUT,
            &format!("/api/mp3s/{id}"),
            Some(json!({"mp3": {"title": " ", "artist": "", "track": "x"}})),
        )
        .await;

    assert_eq!(
        response,
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            json!({
                "errors": {
                    "album_name": ["can't be blank"],
                    "artist_name": ["can't be blank"],
                    "title": ["can't be blank"],
                    "track": ["is not a number"]
                },
                "message": "Failed to update mp3"
            })
        )
    );
    assert_eq!(
        tags::read(std::path::Path::new(&file.path))
            .unwrap()
            .title
            .as_deref(),
        Some("Song")
    );
}

#[sqlx::test]
async fn update_rejects_a_duplicate_of_another_mp3(pool: PgPool) {
    let mut app = TestApp::logged_in(pool).await;
    let file = tagged_file();
    insert_mp3(app.pool(), Mp3Seed::default()).await;
    let id = insert_mp3(
        app.pool(),
        Mp3Seed {
            title: "Other",
            filepath: &file.path,
            ..Mp3Seed::default()
        },
    )
    .await;

    let (status, body) = app
        .send(
            Method::PUT,
            &format!("/api/mp3s/{id}"),
            Some(json!({"mp3": {"title": "Song", "artist": "Band", "album": "Record"}})),
        )
        .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["errors"]["title"], json!(["has already been taken"]));
}

#[sqlx::test]
async fn update_of_an_unwritable_file_rolls_back_the_row(pool: PgPool) {
    let mut app = TestApp::logged_in(pool).await;
    let id = insert_mp3(
        app.pool(),
        Mp3Seed {
            filepath: "/no/such/file.mp3",
            ..Mp3Seed::default()
        },
    )
    .await;

    let (status, body) = app
        .send(
            Method::PUT,
            &format!("/api/mp3s/{id}"),
            Some(json!({"mp3": {"title": "New", "artist": "Band", "album": "Record"}})),
        )
        .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(body["errors"]["file"].is_array());
    assert_eq!(
        app.get(&format!("/api/mp3s/{id}")).await.1["mp3"]["title"],
        "Song"
    );
}

#[sqlx::test]
async fn update_of_a_missing_mp3_is_404(pool: PgPool) {
    let mut app = TestApp::logged_in(pool).await;

    let (status, _) = app
        .send(
            Method::PUT,
            "/api/mp3s/9",
            Some(json!({"mp3": {"title": "x"}})),
        )
        .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn play_streams_the_file_as_audio(pool: PgPool) {
    let mut app = TestApp::logged_in(pool).await;
    let file = tagged_file();
    let id = insert_mp3(
        app.pool(),
        Mp3Seed {
            filepath: &file.path,
            ..Mp3Seed::default()
        },
    )
    .await;

    let request = Request::builder()
        .uri(format!("/api/mp3s/{id}/play"))
        .body(Body::empty())
        .unwrap();
    let response = app.raw(request).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CONTENT_TYPE], "audio/mpeg");

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(
        bytes.len() as u64,
        std::fs::metadata(&file.path).unwrap().len()
    );
}

#[sqlx::test]
async fn play_answers_range_requests(pool: PgPool) {
    let mut app = TestApp::logged_in(pool).await;
    let file = tagged_file();
    let id = insert_mp3(
        app.pool(),
        Mp3Seed {
            filepath: &file.path,
            ..Mp3Seed::default()
        },
    )
    .await;

    let request = Request::builder()
        .uri(format!("/api/mp3s/{id}/play"))
        .header(header::RANGE, "bytes=0-99")
        .body(Body::empty())
        .unwrap();
    let response = app.raw(request).await;

    assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(bytes.len(), 100);
}

#[sqlx::test]
async fn play_of_a_missing_mp3_is_404(pool: PgPool) {
    let mut app = TestApp::logged_in(pool).await;

    assert_eq!(app.get("/api/mp3s/9/play").await.0, StatusCode::NOT_FOUND);
}
