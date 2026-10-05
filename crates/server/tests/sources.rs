mod common;

use std::path::Path;

use axum::http::{Method, StatusCode};
use common::TestApp;
use player_server::fixtures::{FixtureTags, write_silent_mp3, write_tagged_mp3};
use player_server::scanner;
use player_server::source_state::SourceState;
use player_server::sources;
use serde_json::{Value, json};
use sqlx::PgPool;

struct Scanned {
    app: TestApp,
    root: tempfile::TempDir,
    source_id: i64,
}

impl Scanned {
    async fn new(pool: PgPool) -> Self {
        let app = TestApp::logged_in(pool).await;
        let root = tempfile::tempdir().unwrap();
        let source_id = common::source_id(app.pool(), &root.path().to_string_lossy()).await;

        Scanned {
            app,
            root,
            source_id,
        }
    }

    fn path(&self, relative: &str) -> std::path::PathBuf {
        self.root.path().join(relative)
    }

    fn write(&self, relative: &str, seconds: u64, tags: FixtureTags<'_>) {
        let path = self.path(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        write_tagged_mp3(&path, seconds, &tags).unwrap();
    }

    async fn scan(&mut self) -> (StatusCode, Value) {
        let response = self
            .app
            .get(&format!("/api/sources/{}/scan", self.source_id))
            .await;
        self.app.state.scanner.wait_idle().await;

        response
    }

    async fn state(&self) -> Option<SourceState> {
        sources::state(self.app.pool(), self.source_id)
            .await
            .unwrap()
    }

    async fn rows(&self) -> Vec<(String, String, String, i32, i32, Option<i32>, bool)> {
        sqlx::query_as(
            "SELECT m.title, ar.name, al.name, m.track, m.year, m.length, m.file_hash IS NOT NULL \
             FROM mp3s m JOIN artists ar ON ar.id = m.artist_id JOIN albums al ON al.id = m.album_id \
             ORDER BY m.filepath",
        )
        .fetch_all(self.app.pool())
        .await
        .unwrap()
    }
}

fn song(title: &str, track: u32) -> FixtureTags<'_> {
    FixtureTags {
        title: Some(title),
        artist: Some("Band"),
        album: Some("Record"),
        year: Some(2001),
        track: Some(track),
        ..FixtureTags::default()
    }
}

#[sqlx::test]
async fn index_lists_sources_by_path_with_mp3_counts(pool: PgPool) {
    let mut app = TestApp::logged_in(pool).await;
    common::source_id(app.pool(), "/zed").await;
    common::insert_mp3(app.pool(), common::Mp3Seed::default()).await;

    let (status, body) = app.get("/api/sources").await;

    assert_eq!(status, StatusCode::OK);

    let rows: Vec<(String, i64)> = body["sources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            (
                row["path"].as_str().unwrap().to_string(),
                row["mp3s_count"].as_i64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        rows,
        vec![("/music".to_string(), 1), ("/zed".to_string(), 0)]
    );
}

#[sqlx::test]
async fn show_returns_the_source(pool: PgPool) {
    let mut app = TestApp::logged_in(pool).await;
    let id = common::source_id(app.pool(), "/music").await;

    assert_eq!(
        app.get(&format!("/api/sources/{id}")).await,
        (
            StatusCode::OK,
            json!({"source": {"id": id, "path": "/music"}})
        )
    );
}

#[sqlx::test]
async fn show_of_a_missing_source_is_404(pool: PgPool) {
    let mut app = TestApp::logged_in(pool).await;

    assert_eq!(app.get("/api/sources/5").await.0, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn create_adds_a_source(pool: PgPool) {
    let mut app = TestApp::logged_in(pool).await;

    let (status, body) = app
        .send(
            Method::POST,
            "/api/sources",
            Some(json!({"source": {"path": "/new/music"}})),
        )
        .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["message"], "Source created");
    assert_eq!(body["source"]["path"], "/new/music");
    assert_eq!(
        sources::state(app.pool(), body["source"]["id"].as_i64().unwrap())
            .await
            .unwrap(),
        Some(SourceState::Unscanned)
    );
}

#[sqlx::test]
async fn create_rejects_blank_missing_and_taken_paths(pool: PgPool) {
    let mut app = TestApp::logged_in(pool).await;
    common::source_id(app.pool(), "/taken").await;

    let blank = app
        .send(
            Method::POST,
            "/api/sources",
            Some(json!({"source": {"path": " "}})),
        )
        .await;
    let missing = app
        .send(Method::POST, "/api/sources", Some(json!({"source": {}})))
        .await;
    let taken = app
        .send(
            Method::POST,
            "/api/sources",
            Some(json!({"source": {"path": "/taken"}})),
        )
        .await;

    assert_eq!(
        blank,
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            json!({"errors": {"path": ["can't be blank"]}, "message": "Failed to create source"})
        )
    );
    assert_eq!(missing.1["errors"]["path"], json!(["can't be blank"]));
    assert_eq!(taken.1["errors"]["path"], json!(["has already been taken"]));
}

#[sqlx::test]
async fn create_requires_a_session(pool: PgPool) {
    let mut app = TestApp::new(pool).await;

    let (status, _) = app
        .send(
            Method::POST,
            "/api/sources",
            Some(json!({"source": {"path": "/x"}})),
        )
        .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn update_changes_the_path(pool: PgPool) {
    let mut app = TestApp::logged_in(pool).await;
    let id = common::source_id(app.pool(), "/music").await;

    let response = app
        .send(
            Method::PUT,
            &format!("/api/sources/{id}"),
            Some(json!({"source": {"path": "/new"}})),
        )
        .await;

    assert_eq!(
        response,
        (
            StatusCode::OK,
            json!({"message": "Source updated", "source": {"id": id, "path": "/new"}})
        )
    );
}

#[sqlx::test]
async fn update_rejects_blank_and_taken_paths(pool: PgPool) {
    let mut app = TestApp::logged_in(pool).await;
    common::source_id(app.pool(), "/taken").await;
    let id = common::source_id(app.pool(), "/music").await;

    let blank = app
        .send(
            Method::PUT,
            &format!("/api/sources/{id}"),
            Some(json!({"source": {"path": ""}})),
        )
        .await;
    let taken = app
        .send(
            Method::PATCH,
            &format!("/api/sources/{id}"),
            Some(json!({"source": {"path": "/taken"}})),
        )
        .await;

    assert_eq!(
        blank,
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            json!({"errors": {"path": ["can't be blank"]}, "message": "Failed to update source"})
        )
    );
    assert_eq!(taken.1["errors"]["path"], json!(["has already been taken"]));
}

#[sqlx::test]
async fn update_of_a_missing_source_is_404(pool: PgPool) {
    let mut app = TestApp::logged_in(pool).await;

    let (status, _) = app
        .send(
            Method::PUT,
            "/api/sources/5",
            Some(json!({"source": {"path": "/x"}})),
        )
        .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn scan_of_a_missing_source_is_404(pool: PgPool) {
    let mut app = TestApp::logged_in(pool).await;

    assert_eq!(
        app.get("/api/sources/5/scan").await.0,
        StatusCode::NOT_FOUND
    );
}

#[sqlx::test]
async fn scan_adds_tagged_files_and_marks_the_source_scanned(pool: PgPool) {
    let mut scanned = Scanned::new(pool).await;
    scanned.write("Band/Record/01.mp3", 3, song("First", 1));
    scanned.write("Band/Record/02.MP3", 4, song("Second", 2));
    std::fs::write(scanned.path("Band/cover.jpg"), b"").unwrap();

    assert_eq!(scanned.scan().await, (StatusCode::OK, Value::Null));
    assert_eq!(scanned.state().await, Some(SourceState::Scanned));
    assert_eq!(
        scanned.rows().await,
        vec![
            (
                "First".to_string(),
                "Band".to_string(),
                "Record".to_string(),
                1,
                2001,
                Some(3),
                true
            ),
            (
                "Second".to_string(),
                "Band".to_string(),
                "Record".to_string(),
                2,
                2001,
                Some(4),
                true
            ),
        ]
    );
}

#[sqlx::test]
async fn untagged_files_take_names_from_the_filename(pool: PgPool) {
    let mut scanned = Scanned::new(pool).await;
    write_silent_mp3(&scanned.path("Some Band - Some Song.mp3"), 1).unwrap();
    write_silent_mp3(&scanned.path("Lonely.mp3"), 2).unwrap();

    scanned.scan().await;

    assert_eq!(
        scanned.rows().await,
        vec![
            (
                "Lonely".to_string(),
                "Unknown".to_string(),
                "Unknown".to_string(),
                0,
                0,
                Some(2),
                true
            ),
            (
                "Some Song".to_string(),
                "Some Band".to_string(),
                "Unknown".to_string(),
                0,
                0,
                Some(1),
                true
            ),
        ]
    );
}

#[sqlx::test]
async fn rescan_updates_changed_tags_and_keeps_the_row(pool: PgPool) {
    let mut scanned = Scanned::new(pool).await;
    scanned.write("a.mp3", 1, song("Before", 1));
    scanned.scan().await;
    let (id_before,): (i64,) = sqlx::query_as("SELECT id FROM mp3s")
        .fetch_one(scanned.app.pool())
        .await
        .unwrap();

    scanned.write("a.mp3", 1, song("After", 5));
    scanned.scan().await;

    let (id_after, title, track): (i64, String, i32) =
        sqlx::query_as("SELECT id, title, track FROM mp3s")
            .fetch_one(scanned.app.pool())
            .await
            .unwrap();
    assert_eq!((id_after, title.as_str(), track), (id_before, "After", 5));
}

#[sqlx::test]
async fn rescan_fills_a_missing_file_hash(pool: PgPool) {
    let mut scanned = Scanned::new(pool).await;
    scanned.write("a.mp3", 1, song("Song", 1));
    scanned.scan().await;
    sqlx::query("UPDATE mp3s SET file_hash = NULL")
        .execute(scanned.app.pool())
        .await
        .unwrap();

    scanned.scan().await;

    let hash: Option<String> = sqlx::query_scalar("SELECT file_hash FROM mp3s")
        .fetch_one(scanned.app.pool())
        .await
        .unwrap();
    assert_eq!(
        hash,
        Some(scanner::file_hash(&scanned.path("a.mp3")).unwrap())
    );
}

#[sqlx::test]
async fn rescan_removes_rows_for_deleted_files(pool: PgPool) {
    let mut scanned = Scanned::new(pool).await;
    scanned.write("a.mp3", 1, song("Kept", 1));
    scanned.write("b.mp3", 2, song("Gone", 2));
    scanned.scan().await;

    std::fs::remove_file(scanned.path("b.mp3")).unwrap();
    scanned.scan().await;

    let titles: Vec<String> = sqlx::query_scalar("SELECT title FROM mp3s")
        .fetch_all(scanned.app.pool())
        .await
        .unwrap();
    assert_eq!(titles, vec!["Kept"]);
}

#[sqlx::test]
async fn duplicate_songs_are_skipped_and_the_scan_finishes(pool: PgPool) {
    let mut scanned = Scanned::new(pool).await;
    scanned.write("a.mp3", 2, song("Same", 1));
    scanned.write("copy/a.mp3", 2, song("Same", 1));

    scanned.scan().await;

    assert_eq!(scanned.rows().await.len(), 1);
    assert_eq!(scanned.state().await, Some(SourceState::Scanned));
}

#[sqlx::test]
async fn unreadable_audio_is_skipped_and_the_scan_finishes(pool: PgPool) {
    let mut scanned = Scanned::new(pool).await;
    std::fs::write(scanned.path("broken.mp3"), b"not audio").unwrap();
    scanned.write("good.mp3", 1, song("Good", 1));

    scanned.scan().await;

    assert_eq!(scanned.rows().await.len(), 1);
    assert_eq!(scanned.state().await, Some(SourceState::Scanned));
}

#[sqlx::test]
async fn missing_source_directory_ends_in_errored(pool: PgPool) {
    let mut app = TestApp::logged_in(pool).await;
    let id = common::source_id(app.pool(), "/no/such/directory").await;

    app.get(&format!("/api/sources/{id}/scan")).await;
    app.state.scanner.wait_idle().await;

    assert_eq!(
        sources::state(app.pool(), id).await.unwrap(),
        Some(SourceState::Errored)
    );
}

#[sqlx::test]
async fn errored_source_can_be_scanned_again(pool: PgPool) {
    let mut scanned = Scanned::new(pool).await;
    sqlx::query("UPDATE sources SET state = 'errored'")
        .execute(scanned.app.pool())
        .await
        .unwrap();

    scanned.scan().await;

    assert_eq!(scanned.state().await, Some(SourceState::Scanned));
}

#[sqlx::test]
async fn scan_request_while_scanning_is_ignored(pool: PgPool) {
    let mut scanned = Scanned::new(pool).await;
    scanned.write("a.mp3", 1, song("Song", 1));
    sqlx::query("UPDATE sources SET state = 'scanning'")
        .execute(scanned.app.pool())
        .await
        .unwrap();

    scanned.scan().await;

    assert_eq!(scanned.rows().await, Vec::new());
    assert_eq!(scanned.state().await, Some(SourceState::Scanning));
}

#[sqlx::test]
async fn scan_with_a_closed_pool_logs_and_returns(pool: PgPool) {
    pool.close().await;

    scanner::run(&pool, 1).await;

    assert!(pool.is_closed());
}

#[sqlx::test]
async fn hidden_files_and_directories_are_not_scanned(pool: PgPool) {
    let mut scanned = Scanned::new(pool).await;
    scanned.write(".hidden/a.mp3", 1, song("Hidden", 1));
    scanned.write("._b.mp3", 1, song("Dot", 2));

    scanned.scan().await;

    assert_eq!(scanned.rows().await, Vec::new());
}

#[test]
fn file_hash_matches_the_written_bytes() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("x.mp3");
    write_silent_mp3(&path, 1).unwrap();

    assert_eq!(scanner::file_hash(Path::new(&path)).unwrap().len(), 64);
}
