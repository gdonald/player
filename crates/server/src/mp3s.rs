use std::path::PathBuf;

use player_core::search::SearchQuery;
use player_core::sort::{Sort, SortDirection, SortField};
use player_types::{Mp3, Mp3Params};
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder};

use crate::db::{escape_like, is_unique_violation};
use crate::error::{AppError, AppResult, Validation};
use crate::{library, tags};

pub const SELECT: &str = "SELECT m.id, COALESCE(m.title, '') AS title, m.track, \
    ar.name AS artist_name, al.name AS album_name, m.length, m.file_hash \
    FROM mp3s m \
    JOIN artists ar ON ar.id = m.artist_id \
    JOIN albums al ON al.id = m.album_id";

#[derive(Debug, sqlx::FromRow)]
pub struct Mp3Row {
    pub id: i64,
    pub title: String,
    pub track: Option<i32>,
    pub artist_name: String,
    pub album_name: String,
    pub length: Option<i32>,
    pub file_hash: Option<String>,
}

impl From<Mp3Row> for Mp3 {
    fn from(row: Mp3Row) -> Self {
        Mp3 {
            id: row.id,
            title: row.title,
            track: row.track,
            artist_name: row.artist_name,
            album_name: row.album_name,
            length: row.length,
            file_hash: row.file_hash,
        }
    }
}

const UPDATE_FAILED: &str = "Failed to update mp3";

fn order_by(sort: Sort) -> String {
    match sort {
        Sort::Default => "ar.name, al.name, m.track, m.title, m.id".to_string(),
        Sort::By(field, direction) => {
            let column = match field {
                SortField::Artist => "ar.name",
                SortField::Album => "al.name",
                SortField::Track => "m.track",
                SortField::Title => "m.title",
            };
            let direction = match direction {
                SortDirection::Ascending => "ASC",
                SortDirection::Descending => "DESC",
            };

            format!("{column} {direction}, m.id")
        }
    }
}

pub async fn list(pool: &PgPool, sort: Sort) -> sqlx::Result<Vec<Mp3>> {
    search(pool, &SearchQuery::default(), sort).await
}

pub async fn search(pool: &PgPool, query: &SearchQuery, sort: Sort) -> sqlx::Result<Vec<Mp3>> {
    let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(SELECT);
    builder.push(" WHERE TRUE");

    if !query.parts.is_empty() {
        builder.push(" AND (");

        for (field_index, field) in ["ar.name", "al.name", "m.title"].iter().enumerate() {
            if field_index > 0 {
                builder.push(" OR ");
            }
            builder.push("(");

            for (part_index, part) in query.parts.iter().enumerate() {
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

    if let Some(artist) = &query.artist {
        builder.push(" AND ar.name ILIKE ");
        builder.push_bind(escape_like(artist));
    }

    if let Some(album) = &query.album {
        builder.push(" AND al.name ILIKE ");
        builder.push_bind(escape_like(album));
    }

    builder.push(" ORDER BY ");
    builder.push(order_by(sort));

    let rows: Vec<Mp3Row> = builder.build_query_as().fetch_all(pool).await?;

    Ok(rows.into_iter().map(Mp3::from).collect())
}

pub async fn find(pool: &PgPool, id: i64) -> sqlx::Result<Option<Mp3>> {
    sqlx::query_as(&format!("{SELECT} WHERE m.id = $1"))
        .bind(id)
        .fetch_optional(pool)
        .await
        .map(|row: Option<Mp3Row>| row.map(Mp3::from))
}

pub async fn filepath(pool: &PgPool, id: i64) -> sqlx::Result<Option<String>> {
    sqlx::query_scalar("SELECT filepath FROM mp3s WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// Track is empty or an integer above -1.
pub fn parse_track(value: Option<&Value>) -> Result<Option<i32>, &'static str> {
    let number = match value {
        None | Some(Value::Null) => return Ok(None),
        Some(Value::String(text)) if text.trim().is_empty() => return Ok(None),
        Some(Value::String(text)) => {
            if let Ok(integer) = text.trim().parse::<i64>() {
                integer
            } else if text.trim().parse::<f64>().is_ok() {
                return Err("must be an integer");
            } else {
                return Err("is not a number");
            }
        }
        Some(Value::Number(number)) => match number.as_i64() {
            Some(integer) => integer,
            None => return Err("must be an integer"),
        },
        Some(_) => return Err("is not a number"),
    };

    if number < 0 {
        return Err("must be greater than -1");
    }

    i32::try_from(number).map(Some).map_err(|_| "is too large")
}

fn title_taken(error: sqlx::Error) -> AppError {
    if is_unique_violation(&error) {
        AppError::field("title", "has already been taken", UPDATE_FAILED)
    } else {
        error.into()
    }
}

fn trimmed(value: Option<&String>) -> String {
    value
        .map(|text| text.trim().to_string())
        .unwrap_or_default()
}

pub async fn update(pool: &PgPool, id: i64, params: &Mp3Params) -> AppResult<Mp3> {
    let path = filepath(pool, id).await?.ok_or(AppError::NotFound)?;

    let mut validation = Validation::default();
    validation.require("title", params.title.as_deref());
    validation.require("artist_name", params.artist.as_deref());
    validation.require("album_name", params.album.as_deref());

    let track = parse_track(params.track.as_ref()).unwrap_or_else(|message| {
        validation.add("track", message);
        None
    });

    validation.into_result(UPDATE_FAILED)?;

    let title = trimmed(params.title.as_ref());
    let artist = trimmed(params.artist.as_ref());
    let album = trimmed(params.album.as_ref());

    let mut transaction = pool.begin().await?;

    let artist_id = library::find_or_create_artist(&mut *transaction, &artist).await?;
    let album_id = library::find_or_create_album(&mut *transaction, artist_id, &album).await?;

    sqlx::query(
        "UPDATE mp3s SET artist_id = $1, album_id = $2, title = $3, track = $4, \
         updated_at = CURRENT_TIMESTAMP WHERE id = $5",
    )
    .bind(artist_id)
    .bind(album_id)
    .bind(&title)
    .bind(track)
    .bind(id)
    .execute(&mut *transaction)
    .await
    .map_err(title_taken)?;

    let file = PathBuf::from(path);
    let written = tokio::task::spawn_blocking(move || {
        let edits = tags::Edits {
            title: &title,
            artist: &artist,
            album: &album,
            track: track.and_then(|track| u32::try_from(track).ok()),
        };

        tags::write_edits(&file, &edits)
    })
    .await
    .map_err(anyhow::Error::from)?;

    if let Err(error) = written {
        return Err(AppError::field(
            "file",
            &format!("{error:#}"),
            UPDATE_FAILED,
        ));
    }

    transaction.commit().await?;

    find(pool, id).await?.ok_or(AppError::NotFound)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[sqlx::test]
    async fn unique_violation_becomes_a_title_error(pool: sqlx::PgPool) {
        sqlx::query("INSERT INTO artists (name) VALUES ('Band')")
            .execute(&pool)
            .await
            .unwrap();

        let error = sqlx::query("INSERT INTO artists (name) VALUES ('Band')")
            .execute(&pool)
            .await
            .unwrap_err();

        assert_eq!(
            title_taken(error).to_string(),
            "Failed to update mp3: title has already been taken"
        );
    }

    #[test]
    fn non_unique_database_errors_pass_through() {
        assert!(matches!(
            title_taken(sqlx::Error::RowNotFound),
            AppError::Internal(_)
        ));
    }

    #[test]
    fn missing_or_empty_track_is_none() {
        assert_eq!(parse_track(None), Ok(None));
        assert_eq!(parse_track(Some(&Value::Null)), Ok(None));
        assert_eq!(parse_track(Some(&json!(" "))), Ok(None));
    }

    #[test]
    fn integer_track_is_accepted_as_a_number_or_a_string() {
        assert_eq!(parse_track(Some(&json!(3))), Ok(Some(3)));
        assert_eq!(parse_track(Some(&json!("4"))), Ok(Some(4)));
        assert_eq!(parse_track(Some(&json!(0))), Ok(Some(0)));
    }

    #[test]
    fn negative_track_is_rejected() {
        assert_eq!(
            parse_track(Some(&json!(-1))),
            Err("must be greater than -1")
        );
    }

    #[test]
    fn fractional_track_is_rejected() {
        assert_eq!(parse_track(Some(&json!(1.5))), Err("must be an integer"));
        assert_eq!(parse_track(Some(&json!("1.5"))), Err("must be an integer"));
    }

    #[test]
    fn non_numeric_track_is_rejected() {
        assert_eq!(parse_track(Some(&json!("three"))), Err("is not a number"));
        assert_eq!(parse_track(Some(&json!(true))), Err("is not a number"));
    }

    #[test]
    fn track_beyond_the_column_range_is_rejected() {
        assert_eq!(
            parse_track(Some(&json!(5_000_000_000_i64))),
            Err("is too large")
        );
    }

    #[test]
    fn default_order_is_artist_album_track_title() {
        assert_eq!(
            order_by(Sort::Default),
            "ar.name, al.name, m.track, m.title, m.id"
        );
    }

    #[test]
    fn each_sort_field_maps_to_its_column() {
        assert_eq!(
            order_by(Sort::By(SortField::Artist, SortDirection::Descending)),
            "ar.name DESC, m.id"
        );
        assert_eq!(
            order_by(Sort::By(SortField::Album, SortDirection::Ascending)),
            "al.name ASC, m.id"
        );
        assert_eq!(
            order_by(Sort::By(SortField::Track, SortDirection::Ascending)),
            "m.track ASC, m.id"
        );
        assert_eq!(
            order_by(Sort::By(SortField::Title, SortDirection::Descending)),
            "m.title DESC, m.id"
        );
    }
}
