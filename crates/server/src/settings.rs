use std::collections::BTreeMap;

use player_core::settings;
use sqlx::PgPool;

use crate::error::{AppError, AppResult};

pub async fn list(pool: &PgPool, user_id: i64) -> sqlx::Result<BTreeMap<String, String>> {
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT name, value FROM user_settings WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(pool)
            .await?;

    Ok(rows.into_iter().collect())
}

/// Stores the setting for the user, replacing any earlier value. An unknown
/// name is not found.
pub async fn put(pool: &PgPool, user_id: i64, name: &str, value: &str) -> AppResult<()> {
    let value = settings::normalize(name, value).ok_or(AppError::NotFound)?;

    sqlx::query(
        "INSERT INTO user_settings (user_id, name, value) VALUES ($1, $2, $3) \
         ON CONFLICT (user_id, name) \
         DO UPDATE SET value = EXCLUDED.value, updated_at = CURRENT_TIMESTAMP",
    )
    .bind(user_id)
    .bind(name)
    .bind(&value)
    .execute(pool)
    .await?;

    Ok(())
}
