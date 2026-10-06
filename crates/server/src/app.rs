use std::future::Future;
use std::path::{Path, PathBuf};

use axum::Router;
use axum::extract::Request;
use axum::http::Uri;
use sqlx::PgPool;
use tower::ServiceBuilder;
use tower::util::MapRequest;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use tower_sessions::cookie::SameSite;
use tower_sessions::{Expiry, SessionManagerLayer};
use tower_sessions_sqlx_store::PostgresStore;
use tracing_subscriber::EnvFilter;

use crate::scanner::Scanner;
use crate::seeds::SeedLogin;
use crate::{db, routes, sources};

pub const SESSION_COOKIE: &str = "_player_session";

pub const DEFAULT_DATABASE_URL: &str = "postgres://localhost/player_development";
pub const DEFAULT_PORT: u16 = 3000;
pub const DEFAULT_WEB_ROOT: &str = "crates/client/dist";
pub const DEFAULT_LOG_FILTER: &str = "info,lofty=error";

/// What the server reads from its environment, with the defaults filled in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub database_url: String,
    pub port: u16,
    pub web_root: PathBuf,
    pub log_filter: String,
    pub seed_login: Option<SeedLogin>,
}

impl Settings {
    /// `variable` looks up an environment variable by name.
    pub fn from_env(variable: impl Fn(&str) -> Option<String>) -> Self {
        Settings {
            database_url: variable("DATABASE_URL")
                .unwrap_or_else(|| DEFAULT_DATABASE_URL.to_string()),
            port: variable("PORT")
                .and_then(|port| port.parse().ok())
                .unwrap_or(DEFAULT_PORT),
            web_root: variable("WEB_ROOT")
                .map_or_else(|| PathBuf::from(DEFAULT_WEB_ROOT), PathBuf::from),
            log_filter: variable("RUST_LOG").unwrap_or_else(|| DEFAULT_LOG_FILTER.to_string()),
            seed_login: variable("PLAYER_SEED_USERNAME")
                .filter(|username| !username.is_empty())
                .zip(variable("PLAYER_SEED_PASSWORD").filter(|password| !password.is_empty()))
                .map(|(username, password)| SeedLogin { username, password }),
        }
    }
}

/// The log filter for `RUST_LOG`, or the default filter when it does not parse.
pub fn log_filter(directives: &str) -> EnvFilter {
    EnvFilter::try_new(directives).unwrap_or_else(|_| EnvFilter::new(DEFAULT_LOG_FILTER))
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub scanner: Scanner,
}

impl AppState {
    pub fn new(pool: PgPool) -> Self {
        AppState {
            scanner: Scanner::new(pool.clone()),
            pool,
        }
    }
}

pub type App = MapRequest<Router, fn(Request) -> Request>;

/// The iOS client asks for `/api/mp3s.json` and the like, which route to the
/// same handlers as the bare paths.
pub fn strip_json_suffix(mut request: Request) -> Request {
    let path = request.uri().path();

    if let Some(stripped) = path.strip_suffix(".json")
        && path.starts_with("/api/")
    {
        let path_and_query = match request.uri().query() {
            Some(query) => format!("{stripped}?{query}"),
            None => stripped.to_string(),
        };

        let mut parts = request.uri().clone().into_parts();
        parts.path_and_query = Some(
            path_and_query
                .parse()
                .expect("a valid path stays valid without its suffix"),
        );
        *request.uri_mut() =
            Uri::from_parts(parts).expect("a valid URI stays valid without its suffix");
    }

    request
}

/// The API under `/api`, and the built client from `web_root` for every other
/// path, with `index.html` for paths that are not files.
pub fn build(state: AppState, web_root: Option<&Path>) -> App {
    let sessions = SessionManagerLayer::new(PostgresStore::new(state.pool.clone()))
        .with_name(SESSION_COOKIE)
        .with_secure(false)
        .with_http_only(true)
        .with_same_site(SameSite::Lax)
        .with_expiry(Expiry::OnSessionEnd);

    let mut router = Router::new().nest("/api", routes::api(state.clone()));

    if let Some(root) = web_root {
        router = router.fallback_service(
            ServeDir::new(root).not_found_service(ServeFile::new(root.join("index.html"))),
        );
    }

    let router = router
        .layer(sessions)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    ServiceBuilder::new()
        .map_request(strip_json_suffix as fn(Request) -> Request)
        .service(router)
}

/// Brings the database up to date and marks scans a restart interrupted as
/// errored, so they can run again.
pub async fn prepare(pool: &PgPool) -> anyhow::Result<u64> {
    db::migrate(pool).await?;

    let reset = sources::reset_interrupted_scans(pool).await?;
    if reset > 0 {
        tracing::warn!("{reset} interrupted scan(s) marked errored");
    }

    Ok(reset)
}

/// Serves the app on the listener until `shutdown` finishes, then lets the
/// requests in flight complete.
pub async fn serve(
    listener: tokio::net::TcpListener,
    pool: PgPool,
    web_root: &Path,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> anyhow::Result<()> {
    tracing::info!("listening on {}", listener.local_addr()?);

    let app = build(AppState::new(pool), Some(web_root));
    axum::serve(
        listener,
        axum::ServiceExt::<Request>::into_make_service(app),
    )
    .with_graceful_shutdown(shutdown)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use axum::body::Body;

    use super::*;

    fn rewritten(uri: &str) -> String {
        let request = Request::builder().uri(uri).body(Body::empty()).unwrap();

        strip_json_suffix(request).uri().to_string()
    }

    #[test]
    fn api_json_suffix_is_removed() {
        assert_eq!(rewritten("/api/mp3s.json"), "/api/mp3s");
    }

    #[test]
    fn query_string_is_kept() {
        assert_eq!(
            rewritten("/api/mp3s.json?sort=title_asc"),
            "/api/mp3s?sort=title_asc"
        );
    }

    #[test]
    fn non_api_paths_are_left_alone() {
        assert_eq!(rewritten("/data.json"), "/data.json");
    }

    #[test]
    fn paths_without_the_suffix_are_left_alone() {
        assert_eq!(rewritten("/api/mp3s"), "/api/mp3s");
    }

    #[test]
    fn absolute_uris_keep_their_authority() {
        assert_eq!(
            rewritten("http://localhost:3000/api/counts.json"),
            "http://localhost:3000/api/counts"
        );
    }

    fn settings(pairs: &[(&str, &str)]) -> Settings {
        Settings::from_env(|name| {
            pairs
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| (*value).to_string())
        })
    }

    #[test]
    fn settings_default_when_the_environment_is_empty() {
        assert_eq!(
            settings(&[]),
            Settings {
                database_url: DEFAULT_DATABASE_URL.to_string(),
                port: DEFAULT_PORT,
                web_root: PathBuf::from(DEFAULT_WEB_ROOT),
                log_filter: DEFAULT_LOG_FILTER.to_string(),
                seed_login: None,
            }
        );
    }

    #[test]
    fn settings_come_from_the_environment() {
        assert_eq!(
            settings(&[
                ("DATABASE_URL", "postgres://db/music"),
                ("PORT", "8080"),
                ("WEB_ROOT", "/srv/player"),
                ("RUST_LOG", "debug"),
                ("PLAYER_SEED_USERNAME", "listener"),
                ("PLAYER_SEED_PASSWORD", "open-sesame"),
            ]),
            Settings {
                database_url: "postgres://db/music".to_string(),
                port: 8080,
                web_root: PathBuf::from("/srv/player"),
                log_filter: "debug".to_string(),
                seed_login: Some(SeedLogin {
                    username: "listener".to_string(),
                    password: "open-sesame".to_string(),
                }),
            }
        );
    }

    #[test]
    fn a_seed_login_needs_both_a_username_and_a_password() {
        assert_eq!(
            settings(&[("PLAYER_SEED_USERNAME", "listener")]).seed_login,
            None
        );
        assert_eq!(
            settings(&[
                ("PLAYER_SEED_USERNAME", "listener"),
                ("PLAYER_SEED_PASSWORD", "")
            ])
            .seed_login,
            None
        );
        assert_eq!(
            settings(&[
                ("PLAYER_SEED_USERNAME", ""),
                ("PLAYER_SEED_PASSWORD", "open-sesame")
            ])
            .seed_login,
            None
        );
    }

    #[test]
    fn an_unreadable_port_uses_the_default() {
        assert_eq!(settings(&[("PORT", "eighty")]).port, DEFAULT_PORT);
    }

    #[test]
    fn a_readable_log_filter_is_used() {
        assert_eq!(log_filter("warn").to_string(), "warn");
    }

    #[test]
    fn an_unreadable_log_filter_uses_the_default() {
        assert_eq!(
            log_filter("lofty=[").to_string(),
            EnvFilter::new(DEFAULT_LOG_FILTER).to_string()
        );
    }

    #[sqlx::test]
    async fn prepare_marks_interrupted_scans_errored(pool: PgPool) {
        sqlx::query("INSERT INTO sources (path, state) VALUES ('/music', 'scanning')")
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(prepare(&pool).await.unwrap(), 1);
        assert_eq!(
            sqlx::query_scalar::<_, String>("SELECT state FROM sources")
                .fetch_one(&pool)
                .await
                .unwrap(),
            "errored"
        );
    }

    #[sqlx::test]
    async fn prepare_with_no_interrupted_scans_resets_nothing(pool: PgPool) {
        assert_eq!(prepare(&pool).await.unwrap(), 0);
    }

    #[sqlx::test]
    async fn serve_answers_requests_until_shutdown(pool: PgPool) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        let address = listener.local_addr().unwrap();
        let web_root = tempfile::tempdir().unwrap();
        let (stop, stopped) = tokio::sync::oneshot::channel::<()>();

        let server = tokio::spawn({
            let root = web_root.path().to_path_buf();
            async move {
                serve(listener, pool, &root, async {
                    stopped.await.ok();
                })
                .await
            }
        });

        let mut stream = tokio::net::TcpStream::connect(address).await.unwrap();
        stream
            .write_all(
                b"GET /api/sessions/active HTTP/1.1\r\nHost: player\r\nConnection: close\r\n\r\n",
            )
            .await
            .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).await.unwrap();

        stop.send(()).unwrap();
        server.await.unwrap().unwrap();

        assert!(response.starts_with("HTTP/1.1 401"), "{response}");
    }
}
