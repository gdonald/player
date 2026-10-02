use sqlx::PgPool;

use crate::error::{AppResult, Validation};

const CREATE_FAILED: &str = "Failed to create user";
const MAX_USERNAME: usize = 32;
const MAX_PASSWORD: usize = 16;
const SALT_PREFIX_LENGTH: usize = 29;

#[derive(Debug, Clone, Copy)]
pub struct NewUser<'a> {
    pub username: &'a str,
    pub password: &'a str,
    pub password_confirmation: &'a str,
}

pub async fn create(pool: &PgPool, user: NewUser<'_>, cost: u32) -> AppResult<i64> {
    let username = user.username.trim().to_lowercase();

    let mut validation = Validation::default();
    validation.require("username", Some(&username));
    validation.require("password", Some(user.password));
    validation.require("password_confirmation", Some(user.password_confirmation));

    if username.chars().count() > MAX_USERNAME {
        validation.add("username", "is too long (maximum is 32 characters)");
    }

    if user.password.chars().count() > MAX_PASSWORD {
        validation.add("password", "is too long (maximum is 16 characters)");
    }

    if user.password != user.password_confirmation {
        validation.add("password_confirmation", "doesn't match Password");
    }

    let taken: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM users WHERE username = $1)")
        .bind(&username)
        .fetch_one(pool)
        .await?;

    if taken {
        validation.add("username", "has already been taken");
    }

    validation.into_result(CREATE_FAILED)?;

    let password = user.password.to_string();
    let hash = tokio::task::spawn_blocking(move || bcrypt::hash(password, cost))
        .await
        .map_err(anyhow::Error::from)?
        .map_err(anyhow::Error::from)?;

    let id = sqlx::query_scalar(
        "INSERT INTO users (username, p_salt, p_hash) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(&username)
    .bind(&hash[..SALT_PREFIX_LENGTH])
    .bind(&hash)
    .fetch_one(pool)
    .await?;

    Ok(id)
}

/// The user id when the password matches the stored bcrypt hash.
pub async fn authenticate(pool: &PgPool, username: &str, password: &str) -> AppResult<Option<i64>> {
    let row: Option<(i64, Option<String>)> =
        sqlx::query_as("SELECT id, p_hash FROM users WHERE username = $1")
            .bind(username.trim().to_lowercase())
            .fetch_optional(pool)
            .await?;

    let Some((id, Some(hash))) = row else {
        return Ok(None);
    };

    let password = password.to_string();
    let matches =
        tokio::task::spawn_blocking(move || bcrypt::verify(password, &hash).unwrap_or(false))
            .await
            .map_err(anyhow::Error::from)?;

    Ok(matches.then_some(id))
}

pub async fn exists(pool: &PgPool, id: i64) -> sqlx::Result<bool> {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM users WHERE id = $1)")
        .bind(id)
        .fetch_one(pool)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;

    const TEST_COST: u32 = 4;

    fn new_user<'a>(username: &'a str, password: &'a str) -> NewUser<'a> {
        NewUser {
            username,
            password,
            password_confirmation: password,
        }
    }

    #[sqlx::test]
    async fn created_user_authenticates_case_insensitively(pool: PgPool) {
        let id = create(&pool, new_user("GD", "changeme"), TEST_COST)
            .await
            .unwrap();

        assert_eq!(
            authenticate(&pool, "Gd", "changeme").await.unwrap(),
            Some(id)
        );
    }

    #[sqlx::test]
    async fn stored_salt_is_the_hash_prefix(pool: PgPool) {
        create(&pool, new_user("gd", "changeme"), TEST_COST)
            .await
            .unwrap();

        let (salt, hash): (String, String) =
            sqlx::query_as("SELECT p_salt, p_hash FROM users WHERE username = 'gd'")
                .fetch_one(&pool)
                .await
                .unwrap();

        assert!(hash.starts_with(&salt));
    }

    #[sqlx::test]
    async fn wrong_password_does_not_authenticate(pool: PgPool) {
        create(&pool, new_user("gd", "changeme"), TEST_COST)
            .await
            .unwrap();

        assert_eq!(authenticate(&pool, "gd", "wrong").await.unwrap(), None);
    }

    #[sqlx::test]
    async fn unknown_user_does_not_authenticate(pool: PgPool) {
        assert_eq!(authenticate(&pool, "nobody", "x").await.unwrap(), None);
    }

    #[sqlx::test]
    async fn user_without_a_hash_does_not_authenticate(pool: PgPool) {
        sqlx::query("INSERT INTO users (username) VALUES ('nohash')")
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(authenticate(&pool, "nohash", "").await.unwrap(), None);
    }

    #[sqlx::test]
    async fn malformed_stored_hash_does_not_authenticate(pool: PgPool) {
        sqlx::query("INSERT INTO users (username, p_hash) VALUES ('bad', 'not-a-hash')")
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(authenticate(&pool, "bad", "x").await.unwrap(), None);
    }

    #[sqlx::test]
    async fn two_a_hash_written_by_the_bcrypt_gem_format_authenticates(pool: PgPool) {
        sqlx::query("INSERT INTO users (username, p_salt, p_hash) VALUES ('gd', $1, $2)")
            .bind("$2a$04$n4Uy0eSnMfvnESYL.bLwuu")
            .bind("$2a$04$n4Uy0eSnMfvnESYL.bLwuuj0U/ETSsoTpRT9GVk5bektyVVa5xnIi")
            .execute(&pool)
            .await
            .unwrap();

        assert!(
            authenticate(&pool, "gd", "correctbatteryhorsestapler")
                .await
                .unwrap()
                .is_some()
        );
    }

    #[sqlx::test]
    async fn create_reports_every_validation_error(pool: PgPool) {
        create(&pool, new_user("taken", "pw"), TEST_COST)
            .await
            .unwrap();

        let error = create(
            &pool,
            NewUser {
                username: "Taken",
                password: "a-password-over-16",
                password_confirmation: "other",
            },
            TEST_COST,
        )
        .await
        .unwrap_err();

        assert_eq!(
            error.to_string(),
            "Failed to create user: password is too long (maximum is 16 characters), \
             password_confirmation doesn't match Password, username has already been taken"
        );
    }

    #[sqlx::test]
    async fn create_rejects_blank_fields_and_long_usernames(pool: PgPool) {
        let long_name = "x".repeat(33);

        let error = create(
            &pool,
            NewUser {
                username: &long_name,
                password: "",
                password_confirmation: "",
            },
            TEST_COST,
        )
        .await
        .unwrap_err();

        assert_eq!(
            error.to_string(),
            "Failed to create user: password can't be blank, \
             password_confirmation can't be blank, \
             username is too long (maximum is 32 characters)"
        );
    }

    #[sqlx::test]
    async fn invalid_cost_is_an_internal_error(pool: PgPool) {
        assert!(matches!(
            create(&pool, new_user("gd", "pw"), 99).await,
            Err(AppError::Internal(_))
        ));
    }

    #[sqlx::test]
    async fn exists_reports_whether_the_id_is_a_user(pool: PgPool) {
        let id = create(&pool, new_user("gd", "pw"), TEST_COST)
            .await
            .unwrap();

        assert!(exists(&pool, id).await.unwrap());
        assert!(!exists(&pool, id + 1).await.unwrap());
    }
}
