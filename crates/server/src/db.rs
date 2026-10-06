use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Postgres, QueryBuilder};
use tower_sessions_sqlx_store::PostgresStore;

pub async fn connect(url: &str) -> sqlx::Result<PgPool> {
    PgPoolOptions::new().max_connections(10).connect(url).await
}

pub async fn migrate(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::migrate!("./migrations").run(pool).await?;
    PostgresStore::new(pool.clone()).migrate().await?;

    Ok(())
}

pub fn is_unique_violation(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .is_some_and(sqlx::error::DatabaseError::is_unique_violation)
}

/// Adds ` AND (...)` matching rows where some field contains every search
/// part, case-insensitively. Adds nothing for a search with no parts.
pub fn push_search_parts(
    builder: &mut QueryBuilder<'_, Postgres>,
    fields: &[&str],
    parts: &[String],
) {
    if parts.is_empty() {
        return;
    }

    builder.push(" AND (");

    for (field_index, field) in fields.iter().enumerate() {
        if field_index > 0 {
            builder.push(" OR ");
        }
        builder.push("(");

        for (part_index, part) in parts.iter().enumerate() {
            if part_index > 0 {
                builder.push(" AND ");
            }
            builder.push(format!("{field} ILIKE "));
            builder.push_bind(format!("%{}%", escape_like(part)));
        }

        builder.push(")");
    }

    builder.push(")");
}

/// Escapes `%`, `_`, and `\` so a value matches literally under `ILIKE`.
pub fn escape_like(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());

    for character in value.chars() {
        if matches!(character, '%' | '_' | '\\') {
            escaped.push('\\');
        }
        escaped.push(character);
    }

    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_like_escapes_wildcards_and_backslashes() {
        assert_eq!(escape_like(r"50%_a\b"), r"50\%\_a\\b");
    }

    #[test]
    fn non_database_errors_are_not_unique_violations() {
        assert!(!is_unique_violation(&sqlx::Error::RowNotFound));
    }

    #[sqlx::test]
    async fn duplicate_names_are_unique_violations(pool: PgPool) {
        sqlx::query("INSERT INTO artists (name) VALUES ('Band')")
            .execute(&pool)
            .await
            .unwrap();

        let error = sqlx::query("INSERT INTO artists (name) VALUES ('Band')")
            .execute(&pool)
            .await
            .unwrap_err();

        assert!(is_unique_violation(&error));
    }

    #[sqlx::test]
    async fn every_unique_index_rejects_a_duplicate(pool: PgPool) {
        sqlx::raw_sql(
            "INSERT INTO sources (path) VALUES ('/music');
             INSERT INTO artists (name) VALUES ('Band');
             INSERT INTO albums (artist_id, name) VALUES (1, 'Record');
             INSERT INTO mp3s (source_id, artist_id, album_id, filepath, title, length)
                 VALUES (1, 1, 1, '/music/a.mp3', 'Song', 100),
                        (1, 1, 1, '/music/c.mp3', 'Other', 200);
             INSERT INTO playlists (name) VALUES ('Mix');
             INSERT INTO playlist_mp3s (playlist_id, mp3_id, position) VALUES (1, 1, 1);
             INSERT INTO queued_mp3s (mp3_id, position) VALUES (1, 1);
             INSERT INTO users (username) VALUES ('gd');",
        )
        .execute(&pool)
        .await
        .unwrap();

        let duplicates = [
            "INSERT INTO sources (path) VALUES ('/music')",
            "INSERT INTO artists (name) VALUES ('Band')",
            "INSERT INTO albums (artist_id, name) VALUES (1, 'Record')",
            "INSERT INTO mp3s (source_id, artist_id, album_id, filepath, title, length) \
             VALUES (1, 1, 1, '/music/b.mp3', 'Song', 100)",
            "INSERT INTO playlists (name) VALUES ('Mix')",
            "INSERT INTO playlist_mp3s (playlist_id, mp3_id, position) VALUES (1, 1, 2)",
            "INSERT INTO playlist_mp3s (playlist_id, mp3_id, position) VALUES (1, 2, 1)",
            "INSERT INTO queued_mp3s (mp3_id, position) VALUES (1, 1)",
            "INSERT INTO users (username) VALUES ('gd')",
        ];

        for duplicate in duplicates {
            let error = sqlx::query(duplicate).execute(&pool).await.unwrap_err();

            assert!(is_unique_violation(&error), "{duplicate}: {error}");
        }
    }

    #[sqlx::test]
    async fn connect_opens_a_pool_at_the_given_url(pool: PgPool) {
        let options = pool.connect_options();
        let url = format!(
            "postgres://{}@{}:{}/{}",
            options.get_username(),
            options.get_host(),
            options.get_port(),
            options.get_database().unwrap()
        );

        let connected = connect(&url).await.unwrap();

        let one: i32 = sqlx::query_scalar("SELECT 1")
            .fetch_one(&connected)
            .await
            .unwrap();
        assert_eq!(one, 1);
    }

    #[sqlx::test]
    async fn migrate_is_repeatable(pool: PgPool) {
        migrate(&pool).await.unwrap();
        migrate(&pool).await.unwrap();
    }
}
