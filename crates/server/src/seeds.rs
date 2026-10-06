use player_core::names::{RECENTLY_PLAYED, UNKNOWN};
use sqlx::PgPool;

use crate::library;
use crate::users::{self, NewUser};

/// The login `player seed` creates, from `PLAYER_SEED_USERNAME` and
/// `PLAYER_SEED_PASSWORD`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedLogin {
    pub username: String,
    pub password: String,
}

/// The Unknown artist and album, the Recently Played playlist, and the seed
/// login when one is given. Running it again adds nothing.
pub async fn run(pool: &PgPool, cost: u32, login: Option<&SeedLogin>) -> anyhow::Result<()> {
    let artist_id = library::find_or_create_artist(pool, UNKNOWN).await?;
    library::find_or_create_album(pool, artist_id, UNKNOWN).await?;

    sqlx::query("INSERT INTO playlists (name) VALUES ($1) ON CONFLICT (name) DO NOTHING")
        .bind(RECENTLY_PLAYED)
        .execute(pool)
        .await?;

    let Some(login) = login else {
        return Ok(());
    };

    let has_user: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM users WHERE username = $1)")
            .bind(login.username.trim().to_lowercase())
            .fetch_one(pool)
            .await?;

    if !has_user {
        users::create(
            pool,
            NewUser {
                username: &login.username,
                password: &login.password,
                password_confirmation: &login.password,
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

    fn login() -> SeedLogin {
        SeedLogin {
            username: "listener".to_string(),
            password: "open-sesame".to_string(),
        }
    }

    async fn counts(pool: &PgPool) -> (i64, i64, i64, i64) {
        sqlx::query_as(
            "SELECT (SELECT COUNT(*) FROM artists), (SELECT COUNT(*) FROM albums), \
             (SELECT COUNT(*) FROM playlists), (SELECT COUNT(*) FROM users)",
        )
        .fetch_one(pool)
        .await
        .unwrap()
    }

    #[sqlx::test]
    async fn seeding_twice_creates_each_row_once(pool: PgPool) {
        run(&pool, 4, Some(&login())).await.unwrap();
        run(&pool, 4, Some(&login())).await.unwrap();

        assert_eq!(counts(&pool).await, (1, 1, 1, 1));
    }

    #[sqlx::test]
    async fn the_seeded_user_logs_in_with_the_seed_password(pool: PgPool) {
        run(&pool, 4, Some(&login())).await.unwrap();

        assert!(
            users::authenticate(&pool, "listener", "open-sesame")
                .await
                .unwrap()
                .is_some()
        );
    }

    #[sqlx::test]
    async fn seeding_without_a_login_creates_no_user(pool: PgPool) {
        run(&pool, 4, None).await.unwrap();

        assert_eq!(counts(&pool).await, (1, 1, 1, 0));
    }
}
