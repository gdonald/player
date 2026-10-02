mod common;

use axum::http::{Method, StatusCode};
use common::{Mp3Seed, TestApp, insert_mp3, titles};
use serde_json::{Value, json};
use sqlx::PgPool;

async fn with_mp3s(pool: PgPool) -> (TestApp, Vec<i64>) {
    let app = TestApp::logged_in(pool).await;
    let mut ids = Vec::new();

    for (title, filepath) in [
        ("One", "/m/1.mp3"),
        ("Two", "/m/2.mp3"),
        ("Three", "/m/3.mp3"),
    ] {
        ids.push(
            insert_mp3(
                app.pool(),
                Mp3Seed {
                    title,
                    filepath,
                    ..Mp3Seed::default()
                },
            )
            .await,
        );
    }

    (app, ids)
}

async fn enqueue(app: &mut TestApp, mp3_id: Value) -> (StatusCode, Value) {
    app.send(
        Method::POST,
        "/api/queued_mp3s",
        Some(json!({"queued_mp3": {"mp3_id": mp3_id}})),
    )
    .await
}

fn positions(body: &Value) -> Vec<i64> {
    body["queued_mp3s"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["position"].as_i64().unwrap())
        .collect()
}

#[sqlx::test]
async fn create_appends_to_the_end_of_the_queue(pool: PgPool) {
    let (mut app, ids) = with_mp3s(pool).await;

    enqueue(&mut app, json!(ids[1])).await;
    let (status, body) = enqueue(&mut app, json!(ids[0].to_string())).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(titles(&body, "queued_mp3s"), vec!["Two", "One"]);
    assert_eq!(positions(&body), vec![1, 2]);
}

#[sqlx::test]
async fn queued_entries_carry_the_mp3_fields(pool: PgPool) {
    let (mut app, ids) = with_mp3s(pool).await;

    let (_, body) = enqueue(&mut app, json!(ids[0])).await;

    assert_eq!(
        body["queued_mp3s"][0]["mp3"],
        json!({
            "id": ids[0], "title": "One", "track": 1, "artist_name": "Band",
            "album_name": "Record", "length": 100, "file_hash": "hash"
        })
    );
}

#[sqlx::test]
async fn create_rejects_an_unknown_mp3(pool: PgPool) {
    let (mut app, _) = with_mp3s(pool).await;

    assert_eq!(
        enqueue(&mut app, json!(999)).await,
        (StatusCode::UNPROCESSABLE_ENTITY, json!(["Mp3 must exist"]))
    );
}

#[sqlx::test]
async fn create_rejects_a_non_numeric_id(pool: PgPool) {
    let (mut app, _) = with_mp3s(pool).await;

    assert_eq!(
        enqueue(&mut app, json!("abc")).await.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[sqlx::test]
async fn destroy_closes_the_gap(pool: PgPool) {
    let (mut app, ids) = with_mp3s(pool).await;
    for id in &ids {
        enqueue(&mut app, json!(id)).await;
    }
    let (_, body) = app.get("/api/queued_mp3s.json").await;
    let first = body["queued_mp3s"][0]["id"].as_i64().unwrap();

    let (status, body) = app
        .send(
            Method::DELETE,
            &format!("/api/queued_mp3s/{first}.json"),
            Some(json!({})),
        )
        .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(titles(&body, "queued_mp3s"), vec!["Two", "Three"]);
    assert_eq!(positions(&body), vec![1, 2]);
}

#[sqlx::test]
async fn destroy_of_a_missing_entry_returns_the_queue(pool: PgPool) {
    let (mut app, ids) = with_mp3s(pool).await;
    enqueue(&mut app, json!(ids[0])).await;

    let (status, body) = app.send(Method::DELETE, "/api/queued_mp3s/0", None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(titles(&body, "queued_mp3s"), vec!["One"]);
}

#[sqlx::test]
async fn appending_after_a_removal_takes_the_next_position(pool: PgPool) {
    let (mut app, ids) = with_mp3s(pool).await;
    enqueue(&mut app, json!(ids[0])).await;
    let (_, body) = enqueue(&mut app, json!(ids[1])).await;
    let first = body["queued_mp3s"][0]["id"].as_i64().unwrap();
    app.send(Method::DELETE, &format!("/api/queued_mp3s/{first}"), None)
        .await;

    let (_, body) = enqueue(&mut app, json!(ids[2])).await;

    assert_eq!(positions(&body), vec![1, 2]);
}

#[sqlx::test]
async fn deleting_an_mp3_removes_it_from_the_queue_and_playlists(pool: PgPool) {
    let (mut app, ids) = with_mp3s(pool).await;
    enqueue(&mut app, json!(ids[0])).await;
    app.send(
        Method::POST,
        "/api/playlists",
        Some(
            json!({"playlist": {"name": "Mix", "playlist_mp3s_attributes": [{"mp3_id": ids[0]}]}}),
        ),
    )
    .await;

    sqlx::query("DELETE FROM mp3s WHERE id = $1")
        .bind(ids[0])
        .execute(app.pool())
        .await
        .unwrap();

    assert_eq!(
        app.get("/api/queued_mp3s").await.1,
        json!({"queued_mp3s": []})
    );
    assert_eq!(
        app.get("/api/playlists").await.1["playlists"][0]["mp3s_count"],
        0
    );
}
