use player_types::{Mp3, PlaylistMp3};
use sqlx::PgPool;

use crate::error::{AppError, AppResult};
use crate::mp3s::{self, Mp3Row};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Move {
    Higher,
    Lower,
}

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

/// Swaps the entry with its neighbor. At either end it does nothing.
pub async fn move_entry(
    pool: &PgPool,
    playlist_id: i64,
    entry_id: i64,
    direction: Move,
) -> AppResult<()> {
    let position = entry_position(pool, playlist_id, entry_id).await?;

    let neighbor_query = match direction {
        Move::Higher => {
            "SELECT id, position FROM playlist_mp3s WHERE playlist_id = $1 AND position < $2 \
             ORDER BY position DESC LIMIT 1"
        }
        Move::Lower => {
            "SELECT id, position FROM playlist_mp3s WHERE playlist_id = $1 AND position > $2 \
             ORDER BY position ASC LIMIT 1"
        }
    };

    let neighbor: Option<(i64, i32)> = sqlx::query_as(neighbor_query)
        .bind(playlist_id)
        .bind(position)
        .fetch_optional(pool)
        .await?;

    let Some((neighbor_id, neighbor_position)) = neighbor else {
        return Ok(());
    };

    let mut transaction = pool.begin().await?;

    for (id, new_position) in [(entry_id, neighbor_position), (neighbor_id, position)] {
        sqlx::query(
            "UPDATE playlist_mp3s SET position = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2",
        )
        .bind(new_position)
        .bind(id)
        .execute(&mut *transaction)
        .await?;
    }

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
