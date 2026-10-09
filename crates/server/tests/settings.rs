mod common;

use axum::http::{Method, StatusCode};
use common::TestApp;
use player_server::users::{self, NewUser};
use serde_json::{Value, json};
use sqlx::PgPool;

const TREBLE_BOOST: &str = "on;0;0,0,0,0,0,1,2,4,5,6";

async fn put_equalizer(app: &mut TestApp, value: &str) -> (StatusCode, Value) {
    app.send(
        Method::PUT,
        "/api/settings/equalizer",
        Some(json!({"setting": {"value": value}})),
    )
    .await
}

#[sqlx::test]
async fn index_is_empty_before_anything_is_saved(pool: PgPool) {
    let mut app = TestApp::logged_in(pool).await;

    assert_eq!(
        app.get("/api/settings").await,
        (StatusCode::OK, json!({"settings": {}}))
    );
}

#[sqlx::test]
async fn update_stores_the_value_and_returns_every_setting(pool: PgPool) {
    let mut app = TestApp::logged_in(pool).await;

    assert_eq!(
        put_equalizer(&mut app, TREBLE_BOOST).await,
        (
            StatusCode::OK,
            json!({"settings": {"equalizer": TREBLE_BOOST}})
        )
    );
}

#[sqlx::test]
async fn update_replaces_the_earlier_value(pool: PgPool) {
    let mut app = TestApp::logged_in(pool).await;
    put_equalizer(&mut app, TREBLE_BOOST).await;

    put_equalizer(&mut app, "off;-3;1,1,1,1,1,1,1,1,1,1").await;

    assert_eq!(
        app.get("/api/settings").await.1,
        json!({"settings": {"equalizer": "off;-3;1,1,1,1,1,1,1,1,1,1"}})
    );
}

#[sqlx::test]
async fn update_keeps_a_value_in_canonical_form(pool: PgPool) {
    let mut app = TestApp::logged_in(pool).await;

    let (_, body) = app
        .send(
            Method::PUT,
            "/api/settings/equalizer_presets",
            Some(json!({"setting": {"value": "-3;5,4,2,-1,-2,-1,1,3,4,5;Late Night\nbroken"}})),
        )
        .await;

    assert_eq!(
        body,
        json!({"settings": {"equalizer_presets": "-3;5,4,2,-1,-2,-1,1,3,4,5;Late Night"}})
    );
}

#[sqlx::test]
async fn update_of_an_unknown_setting_is_404(pool: PgPool) {
    let mut app = TestApp::logged_in(pool).await;

    let (status, _) = app
        .send(
            Method::PUT,
            "/api/settings/mode",
            Some(json!({"setting": {"value": "1"}})),
        )
        .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn settings_belong_to_the_signed_in_user(pool: PgPool) {
    let mut app = TestApp::logged_in(pool).await;
    put_equalizer(&mut app, TREBLE_BOOST).await;
    users::create(
        app.pool(),
        NewUser {
            username: "other",
            password: "other-password",
            password_confirmation: "other-password",
        },
        4,
    )
    .await
    .unwrap();

    app.set_cookie(None);
    app.send(
        Method::POST,
        "/api/sessions",
        Some(json!({"session": {"username": "other", "password": "other-password"}})),
    )
    .await;

    assert_eq!(app.get("/api/settings").await.1, json!({"settings": {}}));
}

#[sqlx::test]
async fn settings_need_a_signed_in_user(pool: PgPool) {
    let mut app = TestApp::new(pool).await;

    assert_eq!(app.get("/api/settings").await.0, StatusCode::UNAUTHORIZED);
}
