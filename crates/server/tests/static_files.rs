mod common;

use axum::http::StatusCode;
use common::TestApp;
use serde_json::{Value, json};
use sqlx::PgPool;

async fn app_with_web_root(pool: PgPool) -> (TestApp, tempfile::TempDir) {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("index.html"), "<div id=\"root\"></div>").unwrap();
    std::fs::write(root.path().join("app.css"), "body {}").unwrap();

    let app = TestApp::with_web_root(pool, Some(root.path())).await;

    (app, root)
}

#[sqlx::test]
async fn root_serves_the_client_page(pool: PgPool) {
    let (mut app, _root) = app_with_web_root(pool).await;

    assert_eq!(
        app.get("/").await,
        (
            StatusCode::OK,
            Value::String("<div id=\"root\"></div>".to_string())
        )
    );
}

#[sqlx::test]
async fn built_files_are_served(pool: PgPool) {
    let (mut app, _root) = app_with_web_root(pool).await;

    assert_eq!(
        app.get("/app.css").await,
        (StatusCode::OK, Value::String("body {}".to_string()))
    );
}

#[sqlx::test]
async fn other_paths_fall_back_to_the_client_page(pool: PgPool) {
    let (mut app, _root) = app_with_web_root(pool).await;

    assert_eq!(
        app.get("/playlists/3").await.1,
        Value::String("<div id=\"root\"></div>".to_string())
    );
}

#[sqlx::test]
async fn unknown_api_paths_stay_json_404s(pool: PgPool) {
    let (mut app, _root) = app_with_web_root(pool).await;

    assert_eq!(
        app.get("/api/missing").await,
        (StatusCode::NOT_FOUND, json!({"message": "Not found"}))
    );
}
