mod common;

use axum::http::{Method, StatusCode};
use common::{Mp3Seed, TestApp, insert_mp3, titles};
use serde_json::{Value, json};
use sqlx::PgPool;

struct Library {
    app: TestApp,
    first: i64,
    second: i64,
    third: i64,
}

async fn library(pool: PgPool) -> Library {
    let app = TestApp::logged_in(pool).await;

    let first = insert_mp3(
        app.pool(),
        Mp3Seed {
            title: "One",
            track: 3,
            filepath: "/m/1.mp3",
            ..Mp3Seed::default()
        },
    )
    .await;
    let second = insert_mp3(
        app.pool(),
        Mp3Seed {
            title: "Two",
            track: 1,
            filepath: "/m/2.mp3",
            ..Mp3Seed::default()
        },
    )
    .await;
    let third = insert_mp3(
        app.pool(),
        Mp3Seed {
            title: "Three",
            track: 2,
            filepath: "/m/3.mp3",
            ..Mp3Seed::default()
        },
    )
    .await;

    Library {
        app,
        first,
        second,
        third,
    }
}

fn attributes(ids: &[i64]) -> Value {
    Value::Array(
        ids.iter()
            .map(|id| json!({"mp3_id": id.to_string()}))
            .collect(),
    )
}

async fn create(app: &mut TestApp, name: &str, ids: &[i64]) -> i64 {
    let (status, body) = app
        .send(
            Method::POST,
            "/api/playlists",
            Some(json!({"playlist": {"name": name, "playlist_mp3s_attributes": attributes(ids)}})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    body["playlist"]["id"].as_i64().unwrap()
}

async fn entries(app: &mut TestApp, playlist_id: i64) -> Value {
    let (status, body) = app
        .get(&format!("/api/playlists/{playlist_id}/playlist_mp3s.json"))
        .await;
    assert_eq!(status, StatusCode::OK);

    body
}

fn flags(body: &Value) -> Vec<(String, bool, bool)> {
    body["playlist_mp3s"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| {
            (
                entry["mp3"]["title"].as_str().unwrap().to_string(),
                entry["first"].as_bool().unwrap(),
                entry["last"].as_bool().unwrap(),
            )
        })
        .collect()
}

fn entry_id(body: &Value, title: &str) -> i64 {
    body["playlist_mp3s"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["mp3"]["title"] == title)
        .unwrap()["id"]
        .as_i64()
        .unwrap()
}

#[sqlx::test]
async fn index_lists_recently_played_first_then_by_name(pool: PgPool) {
    let mut library = library(pool).await;
    create(&mut library.app, "Zed", &[library.first]).await;
    create(&mut library.app, "Recently Played", &[]).await;
    create(&mut library.app, "Alpha", &[library.first, library.second]).await;

    let (_, body) = library.app.get("/api/playlists.json").await;

    let rows: Vec<(String, i64)> = body["playlists"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            (
                row["name"].as_str().unwrap().to_string(),
                row["mp3s_count"].as_i64().unwrap(),
            )
        })
        .collect();

    assert_eq!(
        rows,
        vec![
            ("Recently Played".to_string(), 0),
            ("Alpha".to_string(), 2),
            ("Zed".to_string(), 1)
        ]
    );
}

#[sqlx::test]
async fn create_orders_entries_by_track_and_reports_success(pool: PgPool) {
    let mut library = library(pool).await;

    let (status, body) = library
        .app
        .send(
            Method::POST,
            "/api/playlists",
            Some(json!({"playlist": {
                "name": "Mix",
                "playlist_mp3s_attributes": attributes(&[library.first, library.second, library.third, library.first])
            }})),
        )
        .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["message"], "Playlist created");
    assert_eq!(body["playlist"]["name"], "Mix");

    let id = body["playlist"]["id"].as_i64().unwrap();
    let listed = entries(&mut library.app, id).await;

    assert_eq!(
        titles(&listed, "playlist_mp3s"),
        vec!["Two", "Three", "One"]
    );
}

#[sqlx::test]
async fn create_rejects_a_blank_name(pool: PgPool) {
    let mut library = library(pool).await;

    let response = library
        .app
        .send(
            Method::POST,
            "/api/playlists",
            Some(json!({"playlist": {"name": ""}})),
        )
        .await;

    assert_eq!(
        response,
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            json!({"errors": {"name": ["can't be blank"]}, "message": "Failed to create playlist"})
        )
    );
}

#[sqlx::test]
async fn create_without_a_name_is_rejected(pool: PgPool) {
    let mut library = library(pool).await;

    let (status, body) = library
        .app
        .send(
            Method::POST,
            "/api/playlists",
            Some(json!({"playlist": {}})),
        )
        .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["errors"]["name"], json!(["can't be blank"]));
}

#[sqlx::test]
async fn create_rejects_a_taken_name(pool: PgPool) {
    let mut library = library(pool).await;
    create(&mut library.app, "Mix", &[]).await;

    let (status, body) = library
        .app
        .send(
            Method::POST,
            "/api/playlists",
            Some(json!({"playlist": {"name": "Mix"}})),
        )
        .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["errors"]["name"], json!(["has already been taken"]));
}

#[sqlx::test]
async fn create_rejects_unknown_mp3s(pool: PgPool) {
    let mut library = library(pool).await;

    let (status, body) = library
        .app
        .send(
            Method::POST,
            "/api/playlists",
            Some(
                json!({"playlist": {"name": "Mix", "playlist_mp3s_attributes": [{"mp3_id": 999}]}}),
            ),
        )
        .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["errors"]["playlist_mp3s.mp3"], json!(["must exist"]));
}

#[sqlx::test]
async fn show_returns_the_playlist(pool: PgPool) {
    let mut library = library(pool).await;
    let id = create(&mut library.app, "Mix", &[]).await;

    assert_eq!(
        library.app.get(&format!("/api/playlists/{id}")).await,
        (
            StatusCode::OK,
            json!({"playlist": {"id": id, "name": "Mix"}})
        )
    );
}

#[sqlx::test]
async fn show_of_a_missing_playlist_is_404(pool: PgPool) {
    let mut library = library(pool).await;

    assert_eq!(
        library.app.get("/api/playlists/77").await.0,
        StatusCode::NOT_FOUND
    );
}

#[sqlx::test]
async fn update_renames_the_playlist(pool: PgPool) {
    let mut library = library(pool).await;
    let id = create(&mut library.app, "Mix", &[]).await;

    let response = library
        .app
        .send(
            Method::PUT,
            &format!("/api/playlists/{id}"),
            Some(json!({"playlist": {"name": "Renamed"}})),
        )
        .await;

    assert_eq!(
        response,
        (
            StatusCode::OK,
            json!({"message": "Playlist updated", "playlist": {"id": id, "name": "Renamed"}})
        )
    );
}

#[sqlx::test]
async fn update_appends_new_mp3s_to_the_bottom_and_skips_existing_ones(pool: PgPool) {
    let mut library = library(pool).await;
    let id = create(&mut library.app, "Mix", &[library.first]).await;

    let (status, body) = library
        .app
        .send(
            Method::PATCH,
            &format!("/api/playlists/{id}"),
            Some(json!({"playlist": {"playlist_mp3s_attributes": attributes(&[library.third, library.first, library.second])}})),
        )
        .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["playlist"]["name"], "Mix");
    assert_eq!(
        titles(&entries(&mut library.app, id).await, "playlist_mp3s"),
        vec!["One", "Three", "Two"]
    );
}

#[sqlx::test]
async fn update_rejects_a_name_taken_by_another_playlist(pool: PgPool) {
    let mut library = library(pool).await;
    create(&mut library.app, "Taken", &[]).await;
    let id = create(&mut library.app, "Mix", &[]).await;

    let (status, body) = library
        .app
        .send(
            Method::PUT,
            &format!("/api/playlists/{id}"),
            Some(json!({"playlist": {"name": "Taken"}})),
        )
        .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["message"], "Failed to update playlist");
}

#[sqlx::test]
async fn update_keeps_its_own_name(pool: PgPool) {
    let mut library = library(pool).await;
    let id = create(&mut library.app, "Mix", &[]).await;

    let (status, _) = library
        .app
        .send(
            Method::PUT,
            &format!("/api/playlists/{id}"),
            Some(json!({"playlist": {"name": "Mix"}})),
        )
        .await;

    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test]
async fn update_of_a_missing_playlist_is_404(pool: PgPool) {
    let mut library = library(pool).await;

    let (status, _) = library
        .app
        .send(
            Method::PUT,
            "/api/playlists/77",
            Some(json!({"playlist": {"name": "x"}})),
        )
        .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn destroy_deletes_the_playlist_and_its_entries(pool: PgPool) {
    let mut library = library(pool).await;
    let id = create(&mut library.app, "Mix", &[library.first]).await;

    let response = library
        .app
        .send(Method::DELETE, &format!("/api/playlists/{id}"), None)
        .await;

    assert_eq!(
        response,
        (StatusCode::OK, json!({"message": "Playlist deleted"}))
    );

    let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM playlist_mp3s")
        .fetch_one(library.app.pool())
        .await
        .unwrap();
    assert_eq!(remaining, 0);
}

#[sqlx::test]
async fn destroy_of_a_missing_playlist_is_404(pool: PgPool) {
    let mut library = library(pool).await;

    assert_eq!(
        library
            .app
            .send(Method::DELETE, "/api/playlists/77", None)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
}

#[sqlx::test]
async fn enqueue_appends_entries_in_playlist_order_after_the_queue(pool: PgPool) {
    let mut library = library(pool).await;
    let id = create(&mut library.app, "Mix", &[library.first, library.second]).await;
    library
        .app
        .send(
            Method::POST,
            "/api/queued_mp3s",
            Some(json!({"queued_mp3": {"mp3_id": library.third}})),
        )
        .await;

    let (status, body) = library
        .app
        .send(
            Method::POST,
            &format!("/api/playlists/{id}/enqueue"),
            Some(json!({})),
        )
        .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(titles(&body, "queued_mp3s"), vec!["Three", "Two", "One"]);

    let positions: Vec<i64> = body["queued_mp3s"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["position"].as_i64().unwrap())
        .collect();
    assert_eq!(positions, vec![1, 2, 3]);
}

#[sqlx::test]
async fn enqueue_of_a_missing_playlist_returns_the_unchanged_queue(pool: PgPool) {
    let mut library = library(pool).await;

    assert_eq!(
        library
            .app
            .send(Method::POST, "/api/playlists/77/enqueue", None)
            .await,
        (StatusCode::OK, json!({"queued_mp3s": []}))
    );
}

#[sqlx::test]
async fn entries_mark_the_first_and_last_rows(pool: PgPool) {
    let mut library = library(pool).await;
    let id = create(
        &mut library.app,
        "Mix",
        &[library.first, library.second, library.third],
    )
    .await;

    let listed = entries(&mut library.app, id).await;

    assert_eq!(
        flags(&listed),
        vec![
            ("Two".to_string(), true, false),
            ("Three".to_string(), false, false),
            ("One".to_string(), false, true)
        ]
    );
}

#[sqlx::test]
async fn entries_of_a_missing_playlist_are_404(pool: PgPool) {
    let mut library = library(pool).await;

    assert_eq!(
        library.app.get("/api/playlists/77/playlist_mp3s").await.0,
        StatusCode::NOT_FOUND
    );
}

#[sqlx::test]
async fn move_to_places_the_entry_at_the_position_and_updates_the_end_flags(pool: PgPool) {
    let mut library = library(pool).await;
    let id = create(
        &mut library.app,
        "Mix",
        &[library.first, library.second, library.third],
    )
    .await;
    let listed = entries(&mut library.app, id).await;
    let one = entry_id(&listed, "One");

    let (_, body) = library
        .app
        .send(
            Method::POST,
            &format!("/api/playlists/{id}/playlist_mp3s/{one}/move_to"),
            Some(json!({"playlist_mp3": {"position": 1}})),
        )
        .await;

    assert_eq!(
        flags(&body),
        vec![
            ("One".to_string(), true, false),
            ("Three".to_string(), false, false),
            ("Two".to_string(), false, true)
        ]
    );
}

#[sqlx::test]
async fn move_to_renumbers_positions_from_one(pool: PgPool) {
    let mut library = library(pool).await;
    let id = create(
        &mut library.app,
        "Mix",
        &[library.first, library.second, library.third],
    )
    .await;
    let listed = entries(&mut library.app, id).await;
    let three = entry_id(&listed, "Three");

    library
        .app
        .send(
            Method::POST,
            &format!("/api/playlists/{id}/playlist_mp3s/{three}/move_to"),
            Some(json!({"position": 3})),
        )
        .await;

    let positions: Vec<i32> = sqlx::query_scalar(
        "SELECT position FROM playlist_mp3s WHERE playlist_id = $1 ORDER BY position",
    )
    .bind(id)
    .fetch_all(library.app.pool())
    .await
    .unwrap();

    assert_eq!(positions, vec![1, 2, 3]);
}

#[sqlx::test]
async fn move_to_for_an_entry_of_another_playlist_is_404(pool: PgPool) {
    let mut library = library(pool).await;
    let mix = create(&mut library.app, "Mix", &[library.first]).await;
    let other = create(&mut library.app, "Other", &[]).await;
    let listed = entries(&mut library.app, mix).await;
    let one = entry_id(&listed, "One");

    let (status, _) = library
        .app
        .send(
            Method::POST,
            &format!("/api/playlists/{other}/playlist_mp3s/{one}/move_to"),
            Some(json!({"position": 1})),
        )
        .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn destroying_an_entry_closes_the_gap(pool: PgPool) {
    let mut library = library(pool).await;
    let id = create(
        &mut library.app,
        "Mix",
        &[library.first, library.second, library.third],
    )
    .await;
    let listed = entries(&mut library.app, id).await;
    let two = entry_id(&listed, "Two");

    let (status, body) = library
        .app
        .send(
            Method::DELETE,
            &format!("/api/playlists/{id}/playlist_mp3s/{two}"),
            None,
        )
        .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        flags(&body),
        vec![
            ("Three".to_string(), true, false),
            ("One".to_string(), false, true)
        ]
    );

    let positions: Vec<i32> = sqlx::query_scalar(
        "SELECT position FROM playlist_mp3s WHERE playlist_id = $1 ORDER BY position",
    )
    .bind(id)
    .fetch_all(library.app.pool())
    .await
    .unwrap();
    assert_eq!(positions, vec![1, 2]);
}

async fn record(app: &mut TestApp, mp3_id: i64) -> StatusCode {
    app.send(Method::POST, &format!("/api/mp3s/{mp3_id}/played"), None)
        .await
        .0
}

async fn recently_played(library: &mut Library) -> Vec<String> {
    let id: i64 = sqlx::query_scalar("SELECT id FROM playlists WHERE name = 'Recently Played'")
        .fetch_one(library.app.pool())
        .await
        .unwrap();

    titles(&entries(&mut library.app, id).await, "playlist_mp3s")
}

#[sqlx::test]
async fn played_songs_go_to_the_top_of_recently_played(pool: PgPool) {
    let mut library = library(pool).await;

    assert_eq!(
        record(&mut library.app, library.first).await,
        StatusCode::OK
    );
    record(&mut library.app, library.second).await;
    record(&mut library.app, library.third).await;

    assert_eq!(
        recently_played(&mut library).await,
        vec!["Three", "Two", "One"]
    );
}

#[sqlx::test]
async fn a_song_played_again_moves_back_to_the_top(pool: PgPool) {
    let mut library = library(pool).await;
    record(&mut library.app, library.first).await;
    record(&mut library.app, library.second).await;
    record(&mut library.app, library.third).await;

    record(&mut library.app, library.second).await;

    assert_eq!(
        recently_played(&mut library).await,
        vec!["Two", "Three", "One"]
    );

    let positions: Vec<i32> = sqlx::query_scalar(
        "SELECT pm.position FROM playlist_mp3s pm JOIN playlists p ON p.id = pm.playlist_id \
         WHERE p.name = 'Recently Played' ORDER BY pm.position",
    )
    .fetch_all(library.app.pool())
    .await
    .unwrap();
    assert_eq!(positions, vec![1, 2, 3]);
}

#[sqlx::test]
async fn recently_played_is_created_when_missing(pool: PgPool) {
    let mut library = library(pool).await;

    record(&mut library.app, library.first).await;

    assert_eq!(recently_played(&mut library).await, vec!["One"]);
}

#[sqlx::test]
async fn recording_a_missing_mp3_is_404(pool: PgPool) {
    let mut library = library(pool).await;

    assert_eq!(record(&mut library.app, 999).await, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn recording_requires_a_session(pool: PgPool) {
    let mut app = TestApp::new(pool).await;

    let (status, _) = app.send(Method::POST, "/api/mp3s/1/played", None).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
