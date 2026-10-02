use player_core::names::{RECENTLY_PLAYED, UNKNOWN};
use sqlx::PgPool;

use crate::library;
use crate::users::{self, NewUser};

pub const DEFAULT_USERNAME: &str = "gd";
pub const DEFAULT_PASSWORD: &str = "changeme";

/// The rows `db/seeds.rb` created. Running it again adds nothing.
pub async fn run(pool: &PgPool, cost: u32) -> anyhow::Result<()> {
    let artist_id = library::find_or_create_artist(pool, UNKNOWN).await?;
    library::find_or_create_album(pool, artist_id, UNKNOWN).await?;

    sqlx::query("INSERT INTO playlists (name) VALUES ($1) ON CONFLICT (name) DO NOTHING")
        .bind(RECENTLY_PLAYED)
        .execute(pool)
        .await?;

    let has_user: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM users WHERE username = $1)")
            .bind(DEFAULT_USERNAME)
            .fetch_one(pool)
            .await?;

    if !has_user {
        users::create(
            pool,
            NewUser {
                username: DEFAULT_USERNAME,
                password: DEFAULT_PASSWORD,
                password_confirmation: DEFAULT_PASSWORD,
            },
            cost,
        )
        .await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test]
    async fn seeding_twice_creates_each_row_once(pool: PgPool) {
        run(&pool, 4).await.unwrap();
        run(&pool, 4).await.unwrap();

        let counts: (i64, i64, i64, i64) = sqlx::query_as(
            "SELECT (SELECT COUNT(*) FROM artists), (SELECT COUNT(*) FROM albums), \
             (SELECT COUNT(*) FROM playlists), (SELECT COUNT(*) FROM users)",
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(counts, (1, 1, 1, 1));
    }

    #[sqlx::test]
    async fn seeded_user_logs_in_with_the_default_password(pool: PgPool) {
        run(&pool, 4).await.unwrap();

        assert!(
            users::authenticate(&pool, DEFAULT_USERNAME, DEFAULT_PASSWORD)
                .await
                .unwrap()
                .is_some()
        );
    }
}
