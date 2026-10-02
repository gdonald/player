use player_types::{SourceListItem, SourceParams, SourceSummary};
use sqlx::PgPool;

use crate::db::is_unique_violation;
use crate::error::{AppError, AppResult, Validation};
use crate::source_state::{SourceEvent, SourceState};

const CREATE_FAILED: &str = "Failed to create source";
const UPDATE_FAILED: &str = "Failed to update source";

pub async fn list(pool: &PgPool) -> sqlx::Result<Vec<SourceListItem>> {
    let rows: Vec<(i64, String, i64)> = sqlx::query_as(
        "SELECT s.id, s.path, COUNT(m.id) FROM sources s \
         LEFT JOIN mp3s m ON m.source_id = s.id GROUP BY s.id ORDER BY s.path",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(id, path, mp3s_count)| SourceListItem {
            id,
            path,
            mp3s_count,
        })
        .collect())
}

pub async fn find(pool: &PgPool, id: i64) -> sqlx::Result<Option<SourceSummary>> {
    let row: Option<(i64, String)> = sqlx::query_as("SELECT id, path FROM sources WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;

    Ok(row.map(|(id, path)| SourceSummary { id, path }))
}

fn validate_path(path: Option<&str>, message: &str) -> AppResult<String> {
    let mut validation = Validation::default();
    validation.require("path", path);
    validation.into_result(message)?;

    Ok(path.unwrap_or_default().to_string())
}

fn path_taken(error: sqlx::Error, message: &str) -> AppError {
    if is_unique_violation(&error) {
        AppError::field("path", "has already been taken", message)
    } else {
        error.into()
    }
}

pub async fn create(pool: &PgPool, path: &str) -> AppResult<SourceSummary> {
    let path = validate_path(Some(path), CREATE_FAILED)?;

    let id: i64 = sqlx::query_scalar("INSERT INTO sources (path) VALUES ($1) RETURNING id")
        .bind(&path)
        .fetch_one(pool)
        .await
        .map_err(|error| path_taken(error, CREATE_FAILED))?;

    Ok(SourceSummary { id, path })
}

pub async fn update(pool: &PgPool, id: i64, params: &SourceParams) -> AppResult<SourceSummary> {
    if find(pool, id).await?.is_none() {
        return Err(AppError::NotFound);
    }

    let path = validate_path(params.path.as_deref(), UPDATE_FAILED)?;

    sqlx::query("UPDATE sources SET path = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
        .bind(&path)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|error| path_taken(error, UPDATE_FAILED))?;

    Ok(SourceSummary { id, path })
}

pub async fn state(pool: &PgPool, id: i64) -> sqlx::Result<Option<SourceState>> {
    let state: Option<String> = sqlx::query_scalar("SELECT state FROM sources WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;

    Ok(state.as_deref().and_then(SourceState::parse))
}

/// Applies the event when the source is in a state it may leave, returning the
/// source path. `None` means the transition did not apply.
pub async fn transition(
    pool: &PgPool,
    id: i64,
    event: SourceEvent,
) -> sqlx::Result<Option<String>> {
    sqlx::query_scalar(
        "UPDATE sources SET state = $1, updated_at = CURRENT_TIMESTAMP \
         WHERE id = $2 AND state = ANY($3) RETURNING path",
    )
    .bind(event.target().as_str())
    .bind(id)
    .bind(event.from_states())
    .fetch_optional(pool)
    .await
}

/// A scan cut off by a restart leaves its source in `scanning`.
pub async fn reset_interrupted_scans(pool: &PgPool) -> sqlx::Result<u64> {
    let result = sqlx::query(
        "UPDATE sources SET state = $1, updated_at = CURRENT_TIMESTAMP WHERE state = $2",
    )
    .bind(SourceState::Errored.as_str())
    .bind(SourceState::Scanning.as_str())
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_unique_database_errors_pass_through() {
        assert!(matches!(
            path_taken(sqlx::Error::RowNotFound, CREATE_FAILED),
            AppError::Internal(_)
        ));
    }

    #[sqlx::test]
    async fn unique_violation_becomes_a_path_error(pool: PgPool) {
        create(&pool, "/music").await.unwrap();

        let error = sqlx::query("INSERT INTO sources (path) VALUES ('/music')")
            .execute(&pool)
            .await
            .unwrap_err();

        assert_eq!(
            path_taken(error, CREATE_FAILED).to_string(),
            "Failed to create source: path has already been taken"
        );
    }

    #[sqlx::test]
    async fn create_rejects_a_duplicate_path(pool: PgPool) {
        create(&pool, "/music").await.unwrap();

        assert_eq!(
            create(&pool, "/music").await.unwrap_err().to_string(),
            "Failed to create source: path has already been taken"
        );
    }

    #[sqlx::test]
    async fn create_rejects_a_blank_path(pool: PgPool) {
        assert_eq!(
            create(&pool, " ").await.unwrap_err().to_string(),
            "Failed to create source: path can't be blank"
        );
    }

    #[sqlx::test]
    async fn transition_applies_only_from_allowed_states(pool: PgPool) {
        let source = create(&pool, "/music").await.unwrap();

        assert_eq!(
            transition(&pool, source.id, SourceEvent::Done)
                .await
                .unwrap(),
            None
        );
        assert_eq!(
            transition(&pool, source.id, SourceEvent::StartScan)
                .await
                .unwrap(),
            Some("/music".to_string())
        );
        assert_eq!(
            state(&pool, source.id).await.unwrap(),
            Some(SourceState::Scanning)
        );
    }

    #[sqlx::test]
    async fn reset_moves_scanning_sources_to_errored(pool: PgPool) {
        let scanning = create(&pool, "/a").await.unwrap();
        let untouched = create(&pool, "/b").await.unwrap();
        transition(&pool, scanning.id, SourceEvent::StartScan)
            .await
            .unwrap();

        assert_eq!(reset_interrupted_scans(&pool).await.unwrap(), 1);
        assert_eq!(
            state(&pool, scanning.id).await.unwrap(),
            Some(SourceState::Errored)
        );
        assert_eq!(
            state(&pool, untouched.id).await.unwrap(),
            Some(SourceState::Unscanned)
        );
    }

    #[sqlx::test]
    async fn state_of_a_missing_source_is_none(pool: PgPool) {
        assert_eq!(state(&pool, 99).await.unwrap(), None);
    }
}
