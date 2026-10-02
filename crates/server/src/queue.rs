use player_types::{Mp3, QueuedMp3};
use sqlx::{PgConnection, PgPool};

use crate::error::{AppError, AppResult};
use crate::mp3s::{self, Mp3Row};

#[derive(Debug, sqlx::FromRow)]
struct QueuedRow {
    queued_id: i64,
    position: i32,
    #[sqlx(flatten)]
    mp3: Mp3Row,
}

/// Serializes position assignment between concurrent appends.
pub async fn lock(connection: &mut PgConnection) -> sqlx::Result<()> {
    sqlx::query("LOCK TABLE queued_mp3s IN SHARE ROW EXCLUSIVE MODE")
        .execute(connection)
        .await?;

    Ok(())
}

pub async fn list(pool: &PgPool) -> sqlx::Result<Vec<QueuedMp3>> {
    let select = mp3s::SELECT.replacen("SELECT ", "SELECT q.id AS queued_id, q.position, ", 1);
    let rows: Vec<QueuedRow> = sqlx::query_as(&format!(
        "{select} JOIN queued_mp3s q ON q.mp3_id = m.id ORDER BY q.position, q.id"
    ))
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| QueuedMp3 {
            id: row.queued_id,
            position: row.position,
            mp3: Mp3::from(row.mp3),
        })
        .collect())
}

pub async fn append(pool: &PgPool, mp3_id: Option<i64>) -> AppResult<()> {
    let mut transaction = pool.begin().await?;
    lock(&mut transaction).await?;

    let inserted = sqlx::query(
        "INSERT INTO queued_mp3s (mp3_id, position) \
         SELECT m.id, (SELECT COALESCE(MAX(position), 0) + 1 FROM queued_mp3s) \
         FROM mp3s m WHERE m.id = $1",
    )
    .bind(mp3_id)
    .execute(&mut *transaction)
    .await?;

    if inserted.rows_affected() == 0 {
        return Err(AppError::InvalidMessages(vec![
            "Mp3 must exist".to_string(),
        ]));
    }

    transaction.commit().await?;

    Ok(())
}

/// Removes the entry when present and closes the gap it leaves.
pub async fn remove(pool: &PgPool, id: i64) -> sqlx::Result<()> {
    let mut transaction = pool.begin().await?;
    lock(&mut transaction).await?;

    let position: Option<i32> =
        sqlx::query_scalar("DELETE FROM queued_mp3s WHERE id = $1 RETURNING position")
            .bind(id)
            .fetch_optional(&mut *transaction)
            .await?;

    if let Some(position) = position {
        sqlx::query("UPDATE queued_mp3s SET position = position - 1 WHERE position > $1")
            .bind(position)
            .execute(&mut *transaction)
            .await?;
    }

    transaction.commit().await
}
