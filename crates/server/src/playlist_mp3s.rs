use player_core::reorder;
use player_types::{Mp3, PlaylistMp3};
use sqlx::PgPool;

use crate::error::{AppError, AppResult};
use crate::mp3s::{self, Mp3Row};

#[derive(Debug, sqlx::FromRow)]
struct EntryRow {
    entry_id: i64,
    #[sqlx(flatten)]
    mp3: Mp3Row,
}

pub async fn list(pool: &PgPool, playlist_id: i64) -> AppResult<Vec<PlaylistMp3>> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM playlists WHERE id = $1)")
        .bind(playlist_id)
        .fetch_one(pool)
        .await?;

    if !exists {
        return Err(AppError::NotFound);
    }

    let select = mp3s::SELECT.replacen("SELECT ", "SELECT pm.id AS entry_id, ", 1);
    let rows: Vec<EntryRow> = sqlx::query_as(&format!(
        "{select} JOIN playlist_mp3s pm ON pm.mp3_id = m.id \
         WHERE pm.playlist_id = $1 ORDER BY pm.position, pm.id"
    ))
    .bind(playlist_id)
    .fetch_all(pool)
    .await?;

    let last_index = rows.len().saturating_sub(1);

    Ok(rows
        .into_iter()
        .enumerate()
        .map(|(index, row)| PlaylistMp3 {
            id: row.entry_id,
            first: index == 0,
            last: index == last_index,
            mp3: Mp3::from(row.mp3),
        })
        .collect())
}

async fn entry_position(pool: &PgPool, playlist_id: i64, entry_id: i64) -> AppResult<i32> {
    sqlx::query_scalar("SELECT position FROM playlist_mp3s WHERE id = $1 AND playlist_id = $2")
        .bind(entry_id)
        .bind(playlist_id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)
}

/// Moves the entry to the 1-based position in the playlist, shifting the
/// entries between, and renumbers the playlist from 1.
pub async fn move_to(
    pool: &PgPool,
    playlist_id: i64,
    entry_id: i64,
    position: i64,
) -> AppResult<()> {
    entry_position(pool, playlist_id, entry_id).await?;

    let mut transaction = pool.begin().await?;

    let ids: Vec<i64> = sqlx::query_scalar(
        "SELECT id FROM playlist_mp3s WHERE playlist_id = $1 ORDER BY position, id FOR UPDATE",
    )
    .bind(playlist_id)
    .fetch_all(&mut *transaction)
    .await?;

    let reordered = reorder::move_to(&ids, entry_id, position);

    sqlx::query(
        "UPDATE playlist_mp3s pm SET position = ordered.position::int, \
         updated_at = CURRENT_TIMESTAMP \
         FROM unnest($1::bigint[]) WITH ORDINALITY AS ordered(id, position) \
         WHERE pm.id = ordered.id",
    )
    .bind(&reordered)
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await?;

    Ok(())
}

pub async fn remove(pool: &PgPool, playlist_id: i64, entry_id: i64) -> AppResult<()> {
    let position = entry_position(pool, playlist_id, entry_id).await?;

    let mut transaction = pool.begin().await?;

    sqlx::query("DELETE FROM playlist_mp3s WHERE id = $1")
        .bind(entry_id)
        .execute(&mut *transaction)
        .await?;

    sqlx::query(
        "UPDATE playlist_mp3s SET position = position - 1 WHERE playlist_id = $1 AND position > $2",
    )
    .bind(playlist_id)
    .bind(position)
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await?;

    Ok(())
}
