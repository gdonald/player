use sqlx::PgExecutor;

pub async fn find_or_create_artist<'e>(
    executor: impl PgExecutor<'e>,
    name: &str,
) -> sqlx::Result<i64> {
    sqlx::query_scalar(
        "INSERT INTO artists (name) VALUES ($1) \
         ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name RETURNING id",
    )
    .bind(name)
    .fetch_one(executor)
    .await
}

pub async fn find_or_create_album<'e>(
    executor: impl PgExecutor<'e>,
    artist_id: i64,
    name: &str,
) -> sqlx::Result<i64> {
    sqlx::query_scalar(
        "INSERT INTO albums (artist_id, name) VALUES ($1, $2) \
         ON CONFLICT (artist_id, name) DO UPDATE SET name = EXCLUDED.name RETURNING id",
    )
    .bind(artist_id)
    .bind(name)
    .fetch_one(executor)
    .await
}

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use super::*;

    #[sqlx::test]
    async fn artist_is_created_once(pool: PgPool) {
        let first = find_or_create_artist(&pool, "Band").await.unwrap();
        let second = find_or_create_artist(&pool, "Band").await.unwrap();

        assert_eq!(first, second);
    }

    #[sqlx::test]
    async fn album_is_scoped_to_its_artist(pool: PgPool) {
        let band = find_or_create_artist(&pool, "Band").await.unwrap();
        let other = find_or_create_artist(&pool, "Other").await.unwrap();

        let first = find_or_create_album(&pool, band, "Record").await.unwrap();
        let again = find_or_create_album(&pool, band, "Record").await.unwrap();
        let elsewhere = find_or_create_album(&pool, other, "Record").await.unwrap();

        assert_eq!(first, again);
        assert_ne!(first, elsewhere);
    }
}
