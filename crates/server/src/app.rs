use std::path::Path;

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

use crate::routes;
use crate::scanner::Scanner;

pub const SESSION_COOKIE: &str = "_player_session";

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
}
