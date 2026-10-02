mod common;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use common::{PASSWORD, TestApp, USERNAME};
use serde_json::json;
use sqlx::PgPool;

#[sqlx::test]
async fn protected_routes_answer_401_with_an_empty_object(pool: PgPool) {
    let mut app = TestApp::new(pool).await;

    assert_eq!(
        app.get("/api/counts").await,
        (StatusCode::UNAUTHORIZED, json!({}))
    );
}

#[sqlx::test]
async fn wrapped_login_starts_a_session(pool: PgPool) {
    let mut app = TestApp::new(pool).await;

    let login = app
        .send(
            Method::POST,
            "/api/sessions",
            Some(json!({"session": {"username": USERNAME, "password": PASSWORD}})),
        )
        .await;

    assert_eq!(login, (StatusCode::OK, json!({})));
    assert_eq!(app.get("/api/sessions/active").await.0, StatusCode::OK);
}

#[sqlx::test]
async fn unwrapped_login_from_the_ios_client_starts_a_session(pool: PgPool) {
    let mut app = TestApp::new(pool).await;

    let (status, _) = app
        .send(
            Method::POST,
            "/api/sessions",
            Some(json!({"username": USERNAME, "password": PASSWORD})),
        )
        .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(app.get("/api/sessions/active.json").await.0, StatusCode::OK);
}

#[sqlx::test]
async fn username_is_matched_case_insensitively(pool: PgPool) {
    let mut app = TestApp::new(pool).await;

    let (status, _) = app
        .send(
            Method::POST,
            "/api/sessions",
            Some(json!({"username": "GD", "password": PASSWORD})),
        )
        .await;

    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test]
async fn wrong_password_is_rejected_without_a_session(pool: PgPool) {
    let mut app = TestApp::new(pool).await;

    let login = app
        .send(
            Method::POST,
            "/api/sessions",
            Some(json!({"username": USERNAME, "password": "wrong"})),
        )
        .await;

    assert_eq!(login, (StatusCode::UNAUTHORIZED, json!({})));
    assert_eq!(
        app.get("/api/sessions/active").await.0,
        StatusCode::UNAUTHORIZED
    );
}

#[sqlx::test]
async fn unknown_user_is_rejected(pool: PgPool) {
    let mut app = TestApp::new(pool).await;

    let (status, _) = app
        .send(
            Method::POST,
            "/api/sessions",
            Some(json!({"username": "nobody", "password": PASSWORD})),
        )
        .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn malformed_login_body_is_a_bad_request(pool: PgPool) {
    let mut app = TestApp::new(pool).await;

    let (status, _) = app
        .send_bytes(Method::POST, "/api/sessions", b"{".to_vec())
        .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn oversized_body_is_a_bad_request(pool: PgPool) {
    let mut app = TestApp::new(pool).await;

    let (status, _) = app
        .send_bytes(Method::POST, "/api/sessions", vec![b' '; 3 * 1024 * 1024])
        .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn session_cookie_is_named_for_the_ios_client(pool: PgPool) {
    let mut app = TestApp::new(pool).await;

    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/sessions")
        .body(Body::from(
            json!({"username": USERNAME, "password": PASSWORD}).to_string(),
        ))
        .unwrap();
    let response = app.raw(request).await;

    let cookie = response.headers()[header::SET_COOKIE].to_str().unwrap();

    assert!(cookie.starts_with("_player_session="));
    assert!(cookie.contains("HttpOnly"));
    assert!(cookie.contains("SameSite=Lax"));
}

#[sqlx::test]
async fn logout_ends_the_session(pool: PgPool) {
    let mut app = TestApp::logged_in(pool).await;

    assert_eq!(
        app.get("/api/sessions/destroy").await,
        (StatusCode::OK, json!({}))
    );
    assert_eq!(
        app.get("/api/sessions/active").await.0,
        StatusCode::UNAUTHORIZED
    );
}

#[sqlx::test]
async fn logout_without_a_session_succeeds(pool: PgPool) {
    let mut app = TestApp::new(pool).await;

    assert_eq!(app.get("/api/sessions/destroy").await.0, StatusCode::OK);
}

#[sqlx::test]
async fn session_of_a_deleted_user_is_rejected(pool: PgPool) {
    let mut app = TestApp::logged_in(pool).await;

    sqlx::query("DELETE FROM users")
        .execute(app.pool())
        .await
        .unwrap();

    assert_eq!(
        app.get("/api/sessions/active").await.0,
        StatusCode::UNAUTHORIZED
    );
}

#[sqlx::test]
async fn forged_cookie_is_rejected(pool: PgPool) {
    let mut app = TestApp::new(pool).await;
    app.set_cookie(Some("_player_session=forged".to_string()));

    assert_eq!(
        app.get("/api/sessions/active").await.0,
        StatusCode::UNAUTHORIZED
    );
}

#[sqlx::test]
async fn unknown_api_path_is_a_json_404(pool: PgPool) {
    let mut app = TestApp::logged_in(pool).await;

    assert_eq!(
        app.get("/api/nothing/here").await,
        (StatusCode::NOT_FOUND, json!({"message": "Not found"}))
    );
}

#[sqlx::test]
async fn counts_include_every_table_the_clients_show(pool: PgPool) {
    let mut app = TestApp::logged_in(pool).await;
    common::insert_mp3(app.pool(), common::Mp3Seed::default()).await;

    assert_eq!(
        app.get("/api/counts.json").await,
        (
            StatusCode::OK,
            json!({"mp3s_count": 1, "playlists_count": 0, "queued_mp3s_count": 0, "sources_count": 1})
        )
    );
}
