use player_core::search::SearchQuery;
use player_core::sort::{ListField, SortDirection};
use player_types::{AlbumListItem, ArtistListItem};
use sqlx::{PgExecutor, PgPool, Postgres, QueryBuilder};

use crate::db::{escape_like, push_search_parts};

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

#[derive(Debug, sqlx::FromRow)]
struct AlbumRow {
    id: i64,
    name: String,
    artist_name: String,
    mp3s_count: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct ArtistRow {
    id: i64,
    name: String,
    albums_count: i64,
    mp3s_count: i64,
}

/// The direction keyword for `ORDER BY`.
fn direction(direction: SortDirection) -> &'static str {
    match direction {
        SortDirection::Ascending => "ASC",
        SortDirection::Descending => "DESC",
    }
}

/// The album order: the chosen column, then artist and album name.
fn album_order(sort: Option<(ListField, SortDirection)>) -> String {
    const DEFAULT: &str = "ar.name, al.name, al.id";

    match sort {
        Some((ListField::Album, chosen)) => format!("al.name {}, {DEFAULT}", direction(chosen)),
        Some((ListField::Artist, chosen)) => format!("ar.name {}, {DEFAULT}", direction(chosen)),
        Some((ListField::Songs, chosen)) => format!("mp3s_count {}, {DEFAULT}", direction(chosen)),
        Some((ListField::Albums, _)) | None => DEFAULT.to_string(),
    }
}

/// The artist order: the chosen column, then artist name.
fn artist_order(sort: Option<(ListField, SortDirection)>) -> String {
    const DEFAULT: &str = "ar.name, ar.id";

    match sort {
        Some((ListField::Artist, chosen)) => format!("ar.name {}, ar.id", direction(chosen)),
        Some((ListField::Albums, chosen)) => {
            format!("albums_count {}, {DEFAULT}", direction(chosen))
        }
        Some((ListField::Songs, chosen)) => format!("mp3s_count {}, {DEFAULT}", direction(chosen)),
        Some((ListField::Album, _)) | None => DEFAULT.to_string(),
    }
}

/// Albums with songs, by artist then album name unless `sort` picks a column. Search parts match the album
/// or the artist name, and the `artist:` and `album:` filters match exactly.
pub async fn albums(
    pool: &PgPool,
    query: &SearchQuery,
    sort: Option<(ListField, SortDirection)>,
) -> sqlx::Result<Vec<AlbumListItem>> {
    let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT al.id, al.name, ar.name AS artist_name, COUNT(m.id) AS mp3s_count \
         FROM albums al \
         JOIN artists ar ON ar.id = al.artist_id \
         JOIN mp3s m ON m.album_id = al.id \
         WHERE TRUE",
    );

    push_search_parts(&mut builder, &["ar.name", "al.name"], &query.parts);

    if let Some(artist) = &query.artist {
        builder.push(" AND ar.name ILIKE ");
        builder.push_bind(escape_like(artist));
    }

    if let Some(album) = &query.album {
        builder.push(" AND al.name ILIKE ");
        builder.push_bind(escape_like(album));
    }

    builder.push(" GROUP BY al.id, ar.name ORDER BY ");
    builder.push(album_order(sort));

    let rows: Vec<AlbumRow> = builder.build_query_as().fetch_all(pool).await?;

    Ok(rows
        .into_iter()
        .map(|row| AlbumListItem {
            id: row.id,
            name: row.name,
            artist_name: row.artist_name,
            mp3s_count: row.mp3s_count,
        })
        .collect())
}

/// Artists with songs, by name unless `sort` picks a column. Search parts match the artist name, the
/// `artist:` filter matches it exactly, and the `album:` filter keeps artists
/// with an album of that name.
pub async fn artists(
    pool: &PgPool,
    query: &SearchQuery,
    sort: Option<(ListField, SortDirection)>,
) -> sqlx::Result<Vec<ArtistListItem>> {
    let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT ar.id, ar.name, COUNT(DISTINCT m.album_id) AS albums_count, \
         COUNT(m.id) AS mp3s_count \
         FROM artists ar \
         JOIN mp3s m ON m.artist_id = ar.id \
         WHERE TRUE",
    );

    push_search_parts(&mut builder, &["ar.name"], &query.parts);

    if let Some(artist) = &query.artist {
        builder.push(" AND ar.name ILIKE ");
        builder.push_bind(escape_like(artist));
    }

    if let Some(album) = &query.album {
        builder.push(" AND ar.id IN (SELECT artist_id FROM albums WHERE name ILIKE ");
        builder.push_bind(escape_like(album));
        builder.push(")");
    }

    builder.push(" GROUP BY ar.id ORDER BY ");
    builder.push(artist_order(sort));

    let rows: Vec<ArtistRow> = builder.build_query_as().fetch_all(pool).await?;

    Ok(rows
        .into_iter()
        .map(|row| ArtistListItem {
            id: row.id,
            name: row.name,
            albums_count: row.albums_count,
            mp3s_count: row.mp3s_count,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use super::*;

    #[test]
    fn albums_default_to_artist_then_album() {
        assert_eq!(album_order(None), "ar.name, al.name, al.id");
        assert_eq!(
            album_order(Some((ListField::Albums, SortDirection::Descending))),
            "ar.name, al.name, al.id"
        );
    }

    #[test]
    fn albums_sort_by_the_chosen_column_first() {
        assert_eq!(
            album_order(Some((ListField::Album, SortDirection::Descending))),
            "al.name DESC, ar.name, al.name, al.id"
        );
        assert_eq!(
            album_order(Some((ListField::Artist, SortDirection::Ascending))),
            "ar.name ASC, ar.name, al.name, al.id"
        );
        assert_eq!(
            album_order(Some((ListField::Songs, SortDirection::Descending))),
            "mp3s_count DESC, ar.name, al.name, al.id"
        );
    }

    #[test]
    fn artists_default_to_their_name() {
        assert_eq!(artist_order(None), "ar.name, ar.id");
        assert_eq!(
            artist_order(Some((ListField::Album, SortDirection::Ascending))),
            "ar.name, ar.id"
        );
    }

    #[test]
    fn artists_sort_by_the_chosen_column_first() {
        assert_eq!(
            artist_order(Some((ListField::Artist, SortDirection::Descending))),
            "ar.name DESC, ar.id"
        );
        assert_eq!(
            artist_order(Some((ListField::Albums, SortDirection::Descending))),
            "albums_count DESC, ar.name, ar.id"
        );
        assert_eq!(
            artist_order(Some((ListField::Songs, SortDirection::Ascending))),
            "mp3s_count ASC, ar.name, ar.id"
        );
    }

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
