use axum::extract::{Path, Query, Request, State};
use axum::http::StatusCode;
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use player_core::{search, sort};
use player_types::{
    AlbumsResponse, ArtistsResponse, CountsResponse, MessageResponse, Mp3Params, Mp3Response,
    Mp3sResponse, PlaylistMp3MoveParams, PlaylistMp3sResponse, PlaylistParams, PlaylistResponse,
    PlaylistsResponse, QueuedMp3Params, QueuedMp3sResponse, SessionParams, SourceParams,
    SourceResponse, SourcesResponse,
};
use serde::Deserialize;
use serde_json::{Value, json};
use tower::ServiceExt;
use tower_http::services::ServeFile;
use tower_sessions::Session;

use crate::app::AppState;
use crate::error::{AppError, AppResult};
use crate::params::Params;
use crate::{library, mp3s, playlist_mp3s, playlists, queue, sources, users};

pub const USER_ID: &str = "user_id";

pub fn api(state: AppState) -> Router<AppState> {
    let protected = Router::new()
        .route("/sessions/active", get(session_active))
        .route("/counts", get(counts))
        .route("/mp3s", get(mp3s_index))
        .route("/mp3s/search", get(mp3s_search))
        .route("/albums", get(albums_index))
        .route("/artists", get(artists_index))
        .route(
            "/mp3s/{id}",
            get(mp3s_show).put(mp3s_update).patch(mp3s_update),
        )
        .route("/mp3s/{id}/play", get(mp3s_play))
        .route("/mp3s/{id}/played", post(mp3s_played))
        .route("/playlists", get(playlists_index).post(playlists_create))
        .route(
            "/playlists/{id}",
            get(playlists_show)
                .put(playlists_update)
                .patch(playlists_update)
                .delete(playlists_destroy),
        )
        .route("/playlists/{id}/enqueue", post(playlists_enqueue))
        .route(
            "/playlists/{playlist_id}/playlist_mp3s",
            get(playlist_mp3s_index),
        )
        .route(
            "/playlists/{playlist_id}/playlist_mp3s/{id}",
            axum::routing::delete(playlist_mp3s_destroy),
        )
        .route(
            "/playlists/{playlist_id}/playlist_mp3s/{id}/move_to",
            post(playlist_mp3s_move_to),
        )
        .route(
            "/queued_mp3s",
            get(queued_mp3s_index)
                .post(queued_mp3s_create)
                .delete(queued_mp3s_clear),
        )
        .route(
            "/queued_mp3s/{id}",
            axum::routing::delete(queued_mp3s_destroy),
        )
        .route("/sources", get(sources_index).post(sources_create))
        .route(
            "/sources/{id}",
            get(sources_show).put(sources_update).patch(sources_update),
        )
        .route("/sources/{id}/scan", get(sources_scan))
        .route_layer(middleware::from_fn_with_state(state, require_user));

    Router::new()
        .route("/sessions", post(session_create))
        .route("/sessions/destroy", get(session_destroy))
        .merge(protected)
        .fallback(not_found)
}

async fn not_found() -> AppError {
    AppError::NotFound
}

async fn require_user(
    State(state): State<AppState>,
    session: Session,
    request: Request,
    next: Next,
) -> AppResult<Response> {
    let user_id: Option<i64> = session.get(USER_ID).await.map_err(anyhow::Error::from)?;

    let Some(user_id) = user_id else {
        return Err(AppError::Unauthorized);
    };

    if !users::exists(&state.pool, user_id).await? {
        return Err(AppError::Unauthorized);
    }

    Ok(next.run(request).await)
}

async fn session_create(
    State(state): State<AppState>,
    session: Session,
    Params(params): Params<SessionParams>,
) -> AppResult<Json<Value>> {
    let Some(user_id) =
        users::authenticate(&state.pool, &params.username, &params.password).await?
    else {
        return Err(AppError::Unauthorized);
    };

    session.cycle_id().await.map_err(anyhow::Error::from)?;
    session
        .insert(USER_ID, user_id)
        .await
        .map_err(anyhow::Error::from)?;

    Ok(Json(json!({})))
}

async fn session_destroy(session: Session) -> AppResult<Json<Value>> {
    session
        .remove::<i64>(USER_ID)
        .await
        .map_err(anyhow::Error::from)?;

    Ok(Json(json!({})))
}

async fn session_active() -> Json<Value> {
    Json(json!({}))
}

async fn counts(State(state): State<AppState>) -> AppResult<Json<CountsResponse>> {
    let (mp3s_count, playlists_count, queued_mp3s_count, sources_count) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM mp3s), (SELECT COUNT(*) FROM playlists), \
         (SELECT COUNT(*) FROM queued_mp3s), (SELECT COUNT(*) FROM sources)",
    )
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(CountsResponse {
        mp3s_count,
        playlists_count,
        queued_mp3s_count,
        sources_count,
    }))
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    q: Option<String>,
    sort: Option<String>,
}

async fn mp3s_index(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> AppResult<Json<Mp3sResponse>> {
    let mp3s = mp3s::list(&state.pool, sort::parse(query.sort.as_deref())).await?;

    Ok(Json(Mp3sResponse { mp3s }))
}

async fn mp3s_search(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> AppResult<Json<Mp3sResponse>> {
    let search = search::parse(query.q.as_deref().unwrap_or_default());
    let mp3s = mp3s::search(&state.pool, &search, sort::parse(query.sort.as_deref())).await?;

    Ok(Json(Mp3sResponse { mp3s }))
}

async fn albums_index(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> AppResult<Json<AlbumsResponse>> {
    let search = search::parse(query.q.as_deref().unwrap_or_default());
    let albums = library::albums(
        &state.pool,
        &search,
        sort::parse_list(query.sort.as_deref()),
    )
    .await?;

    Ok(Json(AlbumsResponse { albums }))
}

async fn artists_index(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> AppResult<Json<ArtistsResponse>> {
    let search = search::parse(query.q.as_deref().unwrap_or_default());
    let artists = library::artists(
        &state.pool,
        &search,
        sort::parse_list(query.sort.as_deref()),
    )
    .await?;

    Ok(Json(ArtistsResponse { artists }))
}

async fn mp3s_show(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<Mp3Response>> {
    let mp3 = mp3s::find(&state.pool, id)
        .await?
        .ok_or(AppError::NotFound)?;

    Ok(Json(Mp3Response { message: None, mp3 }))
}

async fn mp3s_update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Params(params): Params<Mp3Params>,
) -> AppResult<Json<Mp3Response>> {
    let mp3 = mp3s::update(&state.pool, id, &params).await?;

    Ok(Json(Mp3Response {
        message: Some("MP3 updated".to_string()),
        mp3,
    }))
}

async fn mp3s_play(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    request: Request,
) -> AppResult<Response> {
    let filepath = mp3s::filepath(&state.pool, id)
        .await?
        .ok_or(AppError::NotFound)?;

    let Ok(response) = ServeFile::new(filepath).oneshot(request).await;

    Ok(response.into_response())
}

async fn mp3s_played(State(state): State<AppState>, Path(id): Path<i64>) -> AppResult<Json<Value>> {
    playlists::record_played(&state.pool, id).await?;

    Ok(Json(json!({})))
}

async fn playlists_index(State(state): State<AppState>) -> AppResult<Json<PlaylistsResponse>> {
    let playlists = playlists::list(&state.pool).await?;

    Ok(Json(PlaylistsResponse { playlists }))
}

async fn playlists_show(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<PlaylistResponse>> {
    let playlist = playlists::find(&state.pool, id)
        .await?
        .ok_or(AppError::NotFound)?;

    Ok(Json(PlaylistResponse {
        message: None,
        playlist,
    }))
}

async fn playlists_create(
    State(state): State<AppState>,
    Params(params): Params<PlaylistParams>,
) -> AppResult<Json<PlaylistResponse>> {
    let playlist = playlists::create(&state.pool, &params).await?;

    Ok(Json(PlaylistResponse {
        message: Some("Playlist created".to_string()),
        playlist,
    }))
}

async fn playlists_update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Params(params): Params<PlaylistParams>,
) -> AppResult<Json<PlaylistResponse>> {
    let playlist = playlists::update(&state.pool, id, &params).await?;

    Ok(Json(PlaylistResponse {
        message: Some("Playlist updated".to_string()),
        playlist,
    }))
}

async fn playlists_destroy(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<MessageResponse>> {
    playlists::delete(&state.pool, id).await?;

    Ok(Json(MessageResponse {
        message: "Playlist deleted".to_string(),
    }))
}

async fn queued_list(state: &AppState) -> AppResult<Json<QueuedMp3sResponse>> {
    let queued_mp3s = queue::list(&state.pool).await?;

    Ok(Json(QueuedMp3sResponse { queued_mp3s }))
}

async fn playlists_enqueue(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<QueuedMp3sResponse>> {
    playlists::enqueue(&state.pool, id).await?;

    queued_list(&state).await
}

async fn entries_list(state: &AppState, playlist_id: i64) -> AppResult<Json<PlaylistMp3sResponse>> {
    let playlist_mp3s = playlist_mp3s::list(&state.pool, playlist_id).await?;

    Ok(Json(PlaylistMp3sResponse { playlist_mp3s }))
}

async fn playlist_mp3s_index(
    State(state): State<AppState>,
    Path(playlist_id): Path<i64>,
) -> AppResult<Json<PlaylistMp3sResponse>> {
    entries_list(&state, playlist_id).await
}

async fn playlist_mp3s_move_to(
    State(state): State<AppState>,
    Path((playlist_id, id)): Path<(i64, i64)>,
    Params(params): Params<PlaylistMp3MoveParams>,
) -> AppResult<Json<PlaylistMp3sResponse>> {
    playlist_mp3s::move_to(&state.pool, playlist_id, id, params.position).await?;

    entries_list(&state, playlist_id).await
}

async fn playlist_mp3s_destroy(
    State(state): State<AppState>,
    Path((playlist_id, id)): Path<(i64, i64)>,
) -> AppResult<Json<PlaylistMp3sResponse>> {
    playlist_mp3s::remove(&state.pool, playlist_id, id).await?;

    entries_list(&state, playlist_id).await
}

async fn queued_mp3s_index(State(state): State<AppState>) -> AppResult<Json<QueuedMp3sResponse>> {
    queued_list(&state).await
}

async fn queued_mp3s_clear(State(state): State<AppState>) -> AppResult<Json<QueuedMp3sResponse>> {
    queue::clear(&state.pool).await?;

    queued_list(&state).await
}

async fn queued_mp3s_create(
    State(state): State<AppState>,
    Params(params): Params<QueuedMp3Params>,
) -> AppResult<Json<QueuedMp3sResponse>> {
    queue::append(&state.pool, params.mp3_id.0).await?;

    queued_list(&state).await
}

async fn queued_mp3s_destroy(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<QueuedMp3sResponse>> {
    queue::remove(&state.pool, id).await?;

    queued_list(&state).await
}

async fn sources_index(State(state): State<AppState>) -> AppResult<Json<SourcesResponse>> {
    let sources = sources::list(&state.pool).await?;

    Ok(Json(SourcesResponse { sources }))
}

async fn sources_show(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<SourceResponse>> {
    let source = sources::find(&state.pool, id)
        .await?
        .ok_or(AppError::NotFound)?;

    Ok(Json(SourceResponse {
        message: None,
        source,
    }))
}

async fn sources_create(
    State(state): State<AppState>,
    Params(params): Params<SourceParams>,
) -> AppResult<Json<SourceResponse>> {
    let source = sources::create(&state.pool, params.path.as_deref().unwrap_or_default()).await?;

    Ok(Json(SourceResponse {
        message: Some("Source created".to_string()),
        source,
    }))
}

async fn sources_update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Params(params): Params<SourceParams>,
) -> AppResult<Json<SourceResponse>> {
    let source = sources::update(&state.pool, id, &params).await?;

    Ok(Json(SourceResponse {
        message: Some("Source updated".to_string()),
        source,
    }))
}

async fn sources_scan(State(state): State<AppState>, Path(id): Path<i64>) -> AppResult<StatusCode> {
    sources::find(&state.pool, id)
        .await?
        .ok_or(AppError::NotFound)?;

    state.scanner.spawn(id);

    Ok(StatusCode::OK)
}
