use std::collections::HashMap;

use player_core::names::RECENTLY_PLAYED;
use player_types::{PlaylistListItem, PlaylistParams, PlaylistSummary};
use sqlx::{PgConnection, PgPool};

use crate::db::is_unique_violation;
use crate::error::{AppError, AppResult, Validation};
use crate::queue;

const CREATE_FAILED: &str = "Failed to create playlist";
const UPDATE_FAILED: &str = "Failed to update playlist";

pub async fn list(pool: &PgPool) -> sqlx::Result<Vec<PlaylistListItem>> {
    let rows: Vec<(i64, String, i64)> = sqlx::query_as(
        "SELECT p.id, p.name, COUNT(pm.id) FROM playlists p \
         LEFT JOIN playlist_mp3s pm ON pm.playlist_id = p.id \
         GROUP BY p.id ORDER BY (p.name <> $1), p.name",
    )
    .bind(RECENTLY_PLAYED)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(id, name, mp3s_count)| PlaylistListItem {
            id,
            name,
            mp3s_count,
        })
        .collect())
}

pub async fn find(pool: &PgPool, id: i64) -> sqlx::Result<Option<PlaylistSummary>> {
    let row: Option<(i64, String)> = sqlx::query_as("SELECT id, name FROM playlists WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;

    Ok(row.map(|(id, name)| PlaylistSummary { id, name }))
}

/// The mp3 ids to add, in request order, without repeats. `None` marks an id
/// that was missing or unparseable.
fn requested_ids(params: &PlaylistParams) -> Vec<Option<i64>> {
    let mut ids: Vec<Option<i64>> = Vec::new();

    for attributes in params.playlist_mp3s_attributes.iter().flatten() {
        if !ids.contains(&attributes.mp3_id.0) {
            ids.push(attributes.mp3_id.0);
        }
    }

    ids
}

async fn validate(
    connection: &mut PgConnection,
    params: &PlaylistParams,
    name_required: bool,
    message: &str,
) -> AppResult<Vec<i64>> {
    let mut validation = Validation::default();

    if name_required || params.name.is_some() {
        validation.require("name", params.name.as_deref());
    }

    let requested = requested_ids(params);
    let present: Vec<i64> = requested.iter().flatten().copied().collect();

    let found: Vec<i64> = sqlx::query_scalar("SELECT id FROM mp3s WHERE id = ANY($1)")
        .bind(&present)
        .fetch_all(&mut *connection)
        .await?;

    if requested
        .iter()
        .any(|id| id.is_none_or(|id| !found.contains(&id)))
    {
        validation.add("playlist_mp3s.mp3", "must exist");
    }

    validation.into_result(message)?;

    Ok(present)
}

/// Orders new entries by track number, falling back to request position, as
/// `Playlist#reorder_using_tracks` did, then numbers them from 1.
async fn add_ordered_by_track(
    connection: &mut PgConnection,
    playlist_id: i64,
    mp3_ids: &[i64],
) -> sqlx::Result<()> {
    let tracks: HashMap<i64, Option<i32>> =
        sqlx::query_as::<_, (i64, Option<i32>)>("SELECT id, track FROM mp3s WHERE id = ANY($1)")
            .bind(mp3_ids)
            .fetch_all(&mut *connection)
            .await?
            .into_iter()
            .collect();

    let mut ordered: Vec<(i64, usize, i64)> = mp3_ids
        .iter()
        .enumerate()
        .map(|(index, mp3_id)| {
            let fallback = i64::try_from(index + 1).unwrap_or(i64::MAX);
            let key = tracks
                .get(mp3_id)
                .copied()
                .flatten()
                .map_or(fallback, i64::from);

            (key, index, *mp3_id)
        })
        .collect();
    ordered.sort_unstable();

    for (position, (_, _, mp3_id)) in (1_i32..).zip(ordered) {
        sqlx::query(
            "INSERT INTO playlist_mp3s (playlist_id, mp3_id, position) VALUES ($1, $2, $3)",
        )
        .bind(playlist_id)
        .bind(mp3_id)
        .bind(position)
        .execute(&mut *connection)
        .await?;
    }

    Ok(())
}

async fn append(
    connection: &mut PgConnection,
    playlist_id: i64,
    mp3_ids: &[i64],
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO playlist_mp3s (playlist_id, mp3_id, position) \
         SELECT $1, requested.mp3_id, \
                (SELECT COALESCE(MAX(position), 0) FROM playlist_mp3s WHERE playlist_id = $1) \
                + ROW_NUMBER() OVER (ORDER BY requested.ordinal) \
         FROM UNNEST($2::BIGINT[]) WITH ORDINALITY AS requested (mp3_id, ordinal) \
         WHERE NOT EXISTS ( \
             SELECT 1 FROM playlist_mp3s WHERE playlist_id = $1 AND mp3_id = requested.mp3_id \
         )",
    )
    .bind(playlist_id)
    .bind(mp3_ids)
    .execute(connection)
    .await?;

    Ok(())
}

fn name_taken(error: sqlx::Error, message: &str) -> AppError {
    if is_unique_violation(&error) {
        AppError::field("name", "has already been taken", message)
    } else {
        error.into()
    }
}

pub async fn create(pool: &PgPool, params: &PlaylistParams) -> AppResult<PlaylistSummary> {
    let mut transaction = pool.begin().await?;

    let mp3_ids = validate(&mut transaction, params, true, CREATE_FAILED).await?;
    let name = params.name.clone().unwrap_or_default();

    let id: i64 = sqlx::query_scalar("INSERT INTO playlists (name) VALUES ($1) RETURNING id")
        .bind(&name)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|error| name_taken(error, CREATE_FAILED))?;

    add_ordered_by_track(&mut transaction, id, &mp3_ids).await?;

    transaction.commit().await?;

    Ok(PlaylistSummary { id, name })
}

pub async fn update(pool: &PgPool, id: i64, params: &PlaylistParams) -> AppResult<PlaylistSummary> {
    let mut transaction = pool.begin().await?;

    let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM playlists WHERE id = $1)")
        .bind(id)
        .fetch_one(&mut *transaction)
        .await?;

    if !exists {
        return Err(AppError::NotFound);
    }

    let mp3_ids = validate(&mut transaction, params, false, UPDATE_FAILED).await?;

    if let Some(name) = &params.name {
        sqlx::query("UPDATE playlists SET name = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
            .bind(name)
            .bind(id)
            .execute(&mut *transaction)
            .await
            .map_err(|error| name_taken(error, UPDATE_FAILED))?;
    }

    append(&mut transaction, id, &mp3_ids).await?;

    transaction.commit().await?;

    find(pool, id).await?.ok_or(AppError::NotFound)
}

pub async fn delete(pool: &PgPool, id: i64) -> AppResult<()> {
    let deleted = sqlx::query("DELETE FROM playlists WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    if deleted.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(())
}

/// Puts the mp3 at the top of Recently Played, creating that playlist when it
/// is missing. A song already in it moves back to the top.
pub async fn record_played(pool: &PgPool, mp3_id: i64) -> AppResult<()> {
    let mut transaction = pool.begin().await?;

    let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM mp3s WHERE id = $1)")
        .bind(mp3_id)
        .fetch_one(&mut *transaction)
        .await?;

    if !exists {
        return Err(AppError::NotFound);
    }

    let playlist_id: i64 = sqlx::query_scalar(
        "INSERT INTO playlists (name) VALUES ($1) \
         ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name RETURNING id",
    )
    .bind(RECENTLY_PLAYED)
    .fetch_one(&mut *transaction)
    .await?;

    let removed: Option<i32> = sqlx::query_scalar(
        "DELETE FROM playlist_mp3s WHERE playlist_id = $1 AND mp3_id = $2 RETURNING position",
    )
    .bind(playlist_id)
    .bind(mp3_id)
    .fetch_optional(&mut *transaction)
    .await?;

    sqlx::query(
        "UPDATE playlist_mp3s SET position = position + 1 \
         WHERE playlist_id = $1 AND position < COALESCE($2, 2147483647)",
    )
    .bind(playlist_id)
    .bind(removed)
    .execute(&mut *transaction)
    .await?;

    sqlx::query("INSERT INTO playlist_mp3s (playlist_id, mp3_id, position) VALUES ($1, $2, 1)")
        .bind(playlist_id)
        .bind(mp3_id)
        .execute(&mut *transaction)
        .await?;

    transaction.commit().await?;

    Ok(())
}

/// Appends the playlist's mp3s to the queue in playlist order. A missing
/// playlist adds nothing.
pub async fn enqueue(pool: &PgPool, id: i64) -> sqlx::Result<()> {
    let mut transaction = pool.begin().await?;
    queue::lock(&mut transaction).await?;

    sqlx::query(
        "INSERT INTO queued_mp3s (mp3_id, position) \
         SELECT pm.mp3_id, \
                (SELECT COALESCE(MAX(position), 0) FROM queued_mp3s) \
                + ROW_NUMBER() OVER (ORDER BY pm.position) \
         FROM playlist_mp3s pm WHERE pm.playlist_id = $1",
    )
    .bind(id)
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await
}

#[cfg(test)]
mod tests {
    use player_types::{FlexId, PlaylistMp3Attributes};

    use super::*;

    #[test]
    fn requested_ids_drop_repeats_and_keep_order() {
        let params = PlaylistParams {
            name: None,
            playlist_mp3s_attributes: Some(vec![
                PlaylistMp3Attributes {
                    mp3_id: FlexId(Some(3)),
                },
                PlaylistMp3Attributes {
                    mp3_id: FlexId(None),
                },
                PlaylistMp3Attributes {
                    mp3_id: FlexId(Some(3)),
                },
                PlaylistMp3Attributes {
                    mp3_id: FlexId(Some(1)),
                },
            ]),
        };

        assert_eq!(requested_ids(&params), vec![Some(3), None, Some(1)]);
    }

    #[test]
    fn requested_ids_are_empty_without_attributes() {
        assert_eq!(
            requested_ids(&PlaylistParams::default()),
            Vec::<Option<i64>>::new()
        );
    }

    #[sqlx::test]
    async fn unique_violation_becomes_a_name_error(pool: PgPool) {
        sqlx::query("INSERT INTO playlists (name) VALUES ('Mix')")
            .execute(&pool)
            .await
            .unwrap();

        let error = sqlx::query("INSERT INTO playlists (name) VALUES ('Mix')")
            .execute(&pool)
            .await
            .unwrap_err();

        assert_eq!(
            name_taken(error, CREATE_FAILED).to_string(),
            "Failed to create playlist: name has already been taken"
        );
    }

    #[test]
    fn non_unique_database_errors_pass_through() {
        assert!(matches!(
            name_taken(sqlx::Error::RowNotFound, CREATE_FAILED),
            AppError::Internal(_)
        ));
    }
}
