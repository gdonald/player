use sqlx::PgPool;

const IMPORTABLE_MP3S: &str = "SELECT id FROM mp3s \
    WHERE source_id IN (SELECT id FROM sources) \
    AND artist_id IN (SELECT id FROM artists) \
    AND album_id IN (SELECT id FROM albums WHERE artist_id IN (SELECT id FROM artists))";

/// Tables in foreign key order, each with the query that reads it from the
/// Rails database. Rows pointing at missing parents are left behind, and list
/// positions are renumbered from 1 because Rails allowed repeats and gaps.
fn tables() -> [(&'static str, String); 8] {
    [
        (
            "sources",
            "SELECT id, path, state, created_at, updated_at FROM sources".to_string(),
        ),
        (
            "artists",
            "SELECT id, name, created_at, updated_at FROM artists".to_string(),
        ),
        (
            "albums",
            "SELECT id, artist_id, name, created_at, updated_at FROM albums \
             WHERE artist_id IN (SELECT id FROM artists)"
                .to_string(),
        ),
        (
            "mp3s",
            format!(
                "SELECT id, source_id, artist_id, album_id, filepath, title, genre, year, track, \
                 length, comment, file_hash, created_at, updated_at FROM mp3s \
                 WHERE id IN ({IMPORTABLE_MP3S})"
            ),
        ),
        (
            "playlists",
            "SELECT id, name, created_at, updated_at FROM playlists".to_string(),
        ),
        (
            "playlist_mp3s",
            format!(
                "SELECT id, playlist_id, mp3_id, \
                 ROW_NUMBER() OVER (PARTITION BY playlist_id ORDER BY position NULLS LAST, id) \
                 AS position, created_at, updated_at FROM playlist_mp3s \
                 WHERE playlist_id IN (SELECT id FROM playlists) \
                 AND mp3_id IN ({IMPORTABLE_MP3S})"
            ),
        ),
        (
            "queued_mp3s",
            format!(
                "SELECT id, mp3_id, ROW_NUMBER() OVER (ORDER BY position, id) AS position, \
                 created_at, updated_at FROM queued_mp3s WHERE mp3_id IN ({IMPORTABLE_MP3S})"
            ),
        ),
        (
            "users",
            "SELECT id, username, p_salt, p_hash, created_at, updated_at FROM users".to_string(),
        ),
    ]
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ImportCounts(pub Vec<(&'static str, u64)>);

impl std::fmt::Display for ImportCounts {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let lines: Vec<String> = self
            .0
            .iter()
            .map(|(table, count)| format!("{table}: {count}"))
            .collect();

        write!(formatter, "{}", lines.join("\n"))
    }
}

/// Copies the Rails app's rows into an empty database, keeping ids.
pub async fn run(target: &PgPool, rails: &PgPool) -> anyhow::Result<ImportCounts> {
    for (table, _) in tables() {
        let has_rows: bool = sqlx::query_scalar(&format!("SELECT EXISTS (SELECT 1 FROM {table})"))
            .fetch_one(target)
            .await?;

        anyhow::ensure!(
            !has_rows,
            "the target database already has rows in {table}; import needs an empty database"
        );
    }

    let mut transaction = target.begin().await?;
    let mut counts = ImportCounts::default();

    for (table, query) in tables() {
        let rows: String = sqlx::query_scalar(&format!(
            "SELECT COALESCE(json_agg(rows), '[]'::json)::text FROM ({query}) rows"
        ))
        .fetch_one(rails)
        .await?;

        let inserted = sqlx::query(&format!(
            "INSERT INTO {table} SELECT * FROM json_populate_recordset(NULL::{table}, $1::json)"
        ))
        .bind(rows)
        .execute(&mut *transaction)
        .await?;

        sqlx::query(&format!(
            "SELECT setval(pg_get_serial_sequence('{table}', 'id'), \
             COALESCE((SELECT MAX(id) FROM {table}), 0) + 1, false)"
        ))
        .execute(&mut *transaction)
        .await?;

        counts.0.push((table, inserted.rows_affected()));
    }

    transaction.commit().await?;

    Ok(counts)
}

#[cfg(test)]
pub(crate) mod tests {
    use sqlx::postgres::PgPoolOptions;

    use super::*;
    use crate::{playlist_mp3s, queue, users};

    const RAILS_SCHEMA: &str = include_str!("../tests/fixtures/rails_schema.sql");

    const RAILS_ROWS: &str = "
        INSERT INTO sources (id, path, mp3s_count, created_at, updated_at, state)
            VALUES (1, '/music', 2, now(), now(), 'scanned');
        INSERT INTO artists (id, name, created_at, updated_at)
            VALUES (1, 'Band', now(), now());
        INSERT INTO albums (id, artist_id, name, created_at, updated_at)
            VALUES (1, 1, 'Record', now(), now()), (2, 99, 'Orphan', now(), now());
        INSERT INTO mp3s (id, source_id, artist_id, album_id, filepath, title, track, length,
                          file_hash, created_at, updated_at)
            VALUES (5, 1, 1, 1, '/music/a.mp3', 'A', 1, 100, 'aa', now(), now()),
                   (6, 1, 1, 1, '/music/b.mp3', 'B', 2, 200, 'bb', now(), now()),
                   (7, 1, 1, 2, '/music/c.mp3', 'C', 3, 300, 'cc', now(), now());
        INSERT INTO playlists (id, name, created_at, updated_at)
            VALUES (3, 'Mix', now(), now());
        INSERT INTO playlist_mp3s (id, playlist_id, mp3_id, position, created_at, updated_at)
            VALUES (10, 3, 6, 2, now(), now()), (11, 3, 5, 2, now(), now()),
                   (12, 3, 7, 1, now(), now()), (13, NULL, 5, 1, now(), now());
        INSERT INTO queued_mp3s (id, mp3_id, position, created_at, updated_at)
            VALUES (20, 6, 4, now(), now()), (21, 5, 9, now(), now()), (22, 7, 1, now(), now());
        INSERT INTO users (id, username, p_salt, p_hash, created_at, updated_at)
            VALUES (1, 'gd', '$2a$04$n4Uy0eSnMfvnESYL.bLwuu',
                    '$2a$04$n4Uy0eSnMfvnESYL.bLwuuj0U/ETSsoTpRT9GVk5bektyVVa5xnIi', now(), now());
    ";

    pub(crate) async fn rails_pool(pool: &PgPool) -> PgPool {
        sqlx::raw_sql("CREATE SCHEMA rails")
            .execute(pool)
            .await
            .unwrap();

        let options = pool
            .connect_options()
            .as_ref()
            .clone()
            .options([("search_path", "rails")]);
        let rails = PgPoolOptions::new().connect_with(options).await.unwrap();

        sqlx::raw_sql(RAILS_SCHEMA).execute(&rails).await.unwrap();
        sqlx::raw_sql(RAILS_ROWS).execute(&rails).await.unwrap();

        rails
    }

    #[sqlx::test]
    async fn import_copies_rows_and_leaves_orphans_behind(pool: PgPool) {
        let rails = rails_pool(&pool).await;

        let counts = run(&pool, &rails).await.unwrap();

        assert_eq!(
            counts.to_string(),
            "sources: 1\nartists: 1\nalbums: 1\nmp3s: 2\nplaylists: 1\n\
             playlist_mp3s: 2\nqueued_mp3s: 2\nusers: 1"
        );
    }

    #[sqlx::test]
    async fn import_renumbers_list_positions(pool: PgPool) {
        let rails = rails_pool(&pool).await;
        run(&pool, &rails).await.unwrap();

        let entries: Vec<(i64, bool, bool)> = playlist_mp3s::list(&pool, 3)
            .await
            .unwrap()
            .into_iter()
            .map(|entry| (entry.id, entry.first, entry.last))
            .collect();
        let queued: Vec<(i64, i32)> = queue::list(&pool)
            .await
            .unwrap()
            .into_iter()
            .map(|entry| (entry.id, entry.position))
            .collect();

        assert_eq!(entries, vec![(10, true, false), (11, false, true)]);
        assert_eq!(queued, vec![(20, 1), (21, 2)]);
    }

    #[sqlx::test]
    async fn imported_user_logs_in_with_the_rails_hash(pool: PgPool) {
        let rails = rails_pool(&pool).await;
        run(&pool, &rails).await.unwrap();

        assert_eq!(
            users::authenticate(&pool, "gd", "correctbatteryhorsestapler")
                .await
                .unwrap(),
            Some(1)
        );
    }

    #[sqlx::test]
    async fn sequences_continue_after_the_imported_ids(pool: PgPool) {
        let rails = rails_pool(&pool).await;
        run(&pool, &rails).await.unwrap();

        let id: i64 =
            sqlx::query_scalar("INSERT INTO playlists (name) VALUES ('New') RETURNING id")
                .fetch_one(&pool)
                .await
                .unwrap();

        assert_eq!(id, 4);
    }

    #[sqlx::test]
    async fn import_refuses_a_database_with_rows(pool: PgPool) {
        let rails = rails_pool(&pool).await;
        sqlx::query("INSERT INTO playlists (name) VALUES ('Existing')")
            .execute(&pool)
            .await
            .unwrap();

        let error = run(&pool, &rails).await.unwrap_err();

        assert_eq!(
            error.to_string(),
            "the target database already has rows in playlists; import needs an empty database"
        );
    }
}
