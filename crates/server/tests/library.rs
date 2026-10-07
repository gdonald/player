mod common;

use axum::http::{Method, StatusCode};
use common::{Mp3Seed, TestApp, insert_mp3, titles};
use serde_json::{Value, json};
use sqlx::PgPool;

/// Two artists, three albums, four songs, and an album with no songs.
async fn library(pool: PgPool) -> TestApp {
    let app = TestApp::logged_in(pool).await;

    for (artist, album, title, track, filepath) in [
        (
            "Iron Maiden",
            "Piece Of Mind",
            "Revelations",
            3,
            "/music/2.mp3",
        ),
        (
            "Iron Maiden",
            "Piece Of Mind",
            "Where Eagles Dare",
            1,
            "/music/1.mp3",
        ),
        ("Iron Maiden", "Powerslave", "Aces High", 1, "/music/3.mp3"),
        ("Alice In Chains", "Dirt", "Rooster", 7, "/music/4.mp3"),
    ] {
        insert_mp3(
            app.pool(),
            Mp3Seed {
                artist,
                album,
                title,
                track,
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

/// The id of the first row in an album or artist list response.
fn first_id(body: &Value, list_key: &str) -> i64 {
    body[list_key][0]["id"].as_i64().unwrap()
}

async fn entry_titles(app: &mut TestApp, playlist_id: i64) -> Vec<String> {
    let (_, body) = app
        .get(&format!("/api/playlists/{playlist_id}/playlist_mp3s"))
        .await;

    titles(&body, "playlist_mp3s")
}

#[sqlx::test]
async fn an_album_playlist_is_named_for_the_artist_and_album_in_track_order(pool: PgPool) {
    let mut app = library(pool).await;
    let (_, list) = app
        .get("/api/albums?q=album%3A%22Piece%20Of%20Mind%22")
        .await;
    let album_id = first_id(&list, "albums");

    let (status, body) = app
        .send(
            Method::POST,
            &format!("/api/albums/{album_id}/playlist"),
            None,
        )
        .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["message"], "Playlist created");
    assert_eq!(body["playlist"]["name"], "Iron Maiden - Piece Of Mind");
    let playlist_id = body["playlist"]["id"].as_i64().unwrap();
    assert_eq!(
        entry_titles(&mut app, playlist_id).await,
        vec!["Where Eagles Dare", "Revelations"]
    );
}

#[sqlx::test]
async fn an_artist_playlist_is_named_for_the_artist_in_album_then_track_order(pool: PgPool) {
    let mut app = library(pool).await;
    let (_, list) = app.get("/api/artists?q=maiden").await;
    let artist_id = first_id(&list, "artists");

    let (status, body) = app
        .send(
            Method::POST,
            &format!("/api/artists/{artist_id}/playlist"),
            None,
        )
        .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["playlist"]["name"], "Iron Maiden");
    let playlist_id = body["playlist"]["id"].as_i64().unwrap();
    assert_eq!(
        entry_titles(&mut app, playlist_id).await,
        vec!["Where Eagles Dare", "Revelations", "Aces High"]
    );
}

#[sqlx::test]
async fn a_taken_playlist_name_gets_a_random_suffix(pool: PgPool) {
    let mut app = library(pool).await;
    let (_, list) = app.get("/api/artists?q=alice").await;
    let artist_id = first_id(&list, "artists");
    let path = format!("/api/artists/{artist_id}/playlist");
    app.send(Method::POST, &path, None).await;

    let (status, body) = app.send(Method::POST, &path, None).await;

    assert_eq!(status, StatusCode::OK);
    let name = body["playlist"]["name"].as_str().unwrap();
    let suffix = name.strip_prefix("Alice In Chains ").unwrap_or_default();
    assert!(
        suffix.len() == 4
            && suffix
                .chars()
                .all(|character| character.is_ascii_hexdigit()),
        "{name}"
    );
    let playlist_id = body["playlist"]["id"].as_i64().unwrap();
    assert_eq!(entry_titles(&mut app, playlist_id).await, vec!["Rooster"]);
}

#[sqlx::test]
async fn a_playlist_of_an_unknown_album_or_artist_is_404(pool: PgPool) {
    let mut app = library(pool).await;

    let (album, _) = app
        .send(Method::POST, "/api/albums/9999/playlist", None)
        .await;
    let (artist, _) = app
        .send(Method::POST, "/api/artists/9999/playlist", None)
        .await;

    assert_eq!(
        (album, artist),
        (StatusCode::NOT_FOUND, StatusCode::NOT_FOUND)
    );
}
