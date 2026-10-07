mod common;

use axum::http::StatusCode;
use common::{Mp3Seed, TestApp, insert_mp3};
use serde_json::{Value, json};
use sqlx::PgPool;

/// Two artists, three albums, four songs, and an album with no songs.
async fn library(pool: PgPool) -> TestApp {
    let app = TestApp::logged_in(pool).await;

    for (artist, album, title, filepath) in [
        (
            "Iron Maiden",
            "Piece Of Mind",
            "Where Eagles Dare",
            "/music/1.mp3",
        ),
        (
            "Iron Maiden",
            "Piece Of Mind",
            "Revelations",
            "/music/2.mp3",
        ),
        ("Iron Maiden", "Powerslave", "Aces High", "/music/3.mp3"),
        ("Alice In Chains", "Dirt", "Rooster", "/music/4.mp3"),
    ] {
        insert_mp3(
            app.pool(),
            Mp3Seed {
                artist,
                album,
                title,
                filepath,
                ..Mp3Seed::default()
            },
        )
        .await;
    }

    sqlx::query("INSERT INTO artists (name) VALUES ('Unknown')")
        .execute(app.pool())
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO albums (artist_id, name) SELECT id, 'Unknown' FROM artists WHERE name = 'Unknown'",
    )
    .execute(app.pool())
    .await
    .unwrap();

    app
}

fn albums(body: &Value) -> Vec<(String, String, i64)> {
    body["albums"]
        .as_array()
        .unwrap()
        .iter()
        .map(|album| {
            (
                album["name"].as_str().unwrap().to_string(),
                album["artist_name"].as_str().unwrap().to_string(),
                album["mp3s_count"].as_i64().unwrap(),
            )
        })
        .collect()
}

fn artists(body: &Value) -> Vec<(String, i64, i64)> {
    body["artists"]
        .as_array()
        .unwrap()
        .iter()
        .map(|artist| {
            (
                artist["name"].as_str().unwrap().to_string(),
                artist["albums_count"].as_i64().unwrap(),
                artist["mp3s_count"].as_i64().unwrap(),
            )
        })
        .collect()
}

fn album(name: &str, artist: &str, songs: i64) -> (String, String, i64) {
    (name.to_string(), artist.to_string(), songs)
}

fn artist(name: &str, albums: i64, songs: i64) -> (String, i64, i64) {
    (name.to_string(), albums, songs)
}

#[sqlx::test]
async fn albums_list_each_album_with_songs_by_artist_then_name(pool: PgPool) {
    let mut app = library(pool).await;

    let (status, body) = app.get("/api/albums").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        albums(&body),
        vec![
            album("Dirt", "Alice In Chains", 1),
            album("Piece Of Mind", "Iron Maiden", 2),
            album("Powerslave", "Iron Maiden", 1),
        ]
    );
}

#[sqlx::test]
async fn album_rows_carry_their_id(pool: PgPool) {
    let mut app = library(pool).await;

    let (_, body) = app.get("/api/albums?q=dirt").await;

    assert!(body["albums"][0]["id"].is_i64(), "{body}");
}

#[sqlx::test]
async fn album_search_matches_the_album_or_the_artist_name(pool: PgPool) {
    let mut app = library(pool).await;

    let (_, by_album) = app.get("/api/albums?q=power").await;
    let (_, by_artist) = app.get("/api/albums?q=alice").await;

    assert_eq!(
        albums(&by_album),
        vec![album("Powerslave", "Iron Maiden", 1)]
    );
    assert_eq!(
        albums(&by_artist),
        vec![album("Dirt", "Alice In Chains", 1)]
    );
}

#[sqlx::test]
async fn album_search_applies_the_artist_and_album_filters(pool: PgPool) {
    let mut app = library(pool).await;

    let (_, by_artist) = app.get("/api/albums?q=artist%3A%22Iron%20Maiden%22").await;
    let (_, by_album) = app.get("/api/albums?q=album%3A%22Dirt%22").await;

    assert_eq!(
        albums(&by_artist),
        vec![
            album("Piece Of Mind", "Iron Maiden", 2),
            album("Powerslave", "Iron Maiden", 1),
        ]
    );
    assert_eq!(albums(&by_album), vec![album("Dirt", "Alice In Chains", 1)]);
}

#[sqlx::test]
async fn artists_list_each_artist_with_songs_by_name(pool: PgPool) {
    let mut app = library(pool).await;

    let (status, body) = app.get("/api/artists").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        artists(&body),
        vec![artist("Alice In Chains", 1, 1), artist("Iron Maiden", 2, 3)]
    );
    assert!(body["artists"][0]["id"].is_i64(), "{body}");
}

#[sqlx::test]
async fn artist_search_matches_the_artist_name(pool: PgPool) {
    let mut app = library(pool).await;

    let (_, body) = app.get("/api/artists?q=maiden").await;

    assert_eq!(artists(&body), vec![artist("Iron Maiden", 2, 3)]);
}

#[sqlx::test]
async fn artist_search_applies_the_artist_and_album_filters(pool: PgPool) {
    let mut app = library(pool).await;

    let (_, by_artist) = app
        .get("/api/artists?q=artist%3A%22Alice%20In%20Chains%22")
        .await;
    let (_, by_album) = app.get("/api/artists?q=album%3A%22Powerslave%22").await;

    assert_eq!(artists(&by_artist), vec![artist("Alice In Chains", 1, 1)]);
    assert_eq!(artists(&by_album), vec![artist("Iron Maiden", 2, 3)]);
}

#[sqlx::test]
async fn album_and_artist_lists_need_a_session(pool: PgPool) {
    let mut app = TestApp::new(pool).await;

    assert_eq!(
        app.get("/api/albums").await,
        (StatusCode::UNAUTHORIZED, json!({}))
    );
    assert_eq!(
        app.get("/api/artists").await,
        (StatusCode::UNAUTHORIZED, json!({}))
    );
}

#[sqlx::test]
async fn albums_sort_by_a_chosen_column_in_either_direction(pool: PgPool) {
    let mut app = library(pool).await;

    let (_, by_songs) = app.get("/api/albums?sort=songs_desc").await;
    let (_, by_name) = app.get("/api/albums?sort=album_desc").await;

    assert_eq!(
        albums(&by_songs),
        vec![
            album("Piece Of Mind", "Iron Maiden", 2),
            album("Dirt", "Alice In Chains", 1),
            album("Powerslave", "Iron Maiden", 1),
        ]
    );
    assert_eq!(
        albums(&by_name),
        vec![
            album("Powerslave", "Iron Maiden", 1),
            album("Piece Of Mind", "Iron Maiden", 2),
            album("Dirt", "Alice In Chains", 1),
        ]
    );
}

#[sqlx::test]
async fn artists_sort_by_a_chosen_column_with_a_search(pool: PgPool) {
    let mut app = library(pool).await;

    let (_, by_albums) = app.get("/api/artists?sort=albums_desc").await;
    let (_, searched) = app.get("/api/artists?q=i&sort=artist_desc").await;

    assert_eq!(
        artists(&by_albums),
        vec![artist("Iron Maiden", 2, 3), artist("Alice In Chains", 1, 1)]
    );
    assert_eq!(
        artists(&searched),
        vec![artist("Iron Maiden", 2, 3), artist("Alice In Chains", 1, 1)]
    );
}
