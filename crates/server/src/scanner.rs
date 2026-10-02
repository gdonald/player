use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::Context;
use player_core::filename::{self, ScanMode};
use player_core::names;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tokio::task::JoinHandle;

use crate::db::is_unique_violation;
use crate::source_state::SourceEvent;
use crate::tags::{self, TagData};
use crate::{library, sources};

#[derive(Debug, Clone)]
pub struct Scanner {
    pool: PgPool,
    tasks: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl Scanner {
    pub fn new(pool: PgPool) -> Self {
        Scanner {
            pool,
            tasks: Arc::default(),
        }
    }

    pub fn spawn(&self, source_id: i64) {
        let pool = self.pool.clone();
        let handle = tokio::spawn(async move { run(&pool, source_id).await });

        self.tasks
            .lock()
            .expect("scanner task list lock")
            .push(handle);
    }

    /// Waits for every scan started so far, including ones started meanwhile.
    pub async fn wait_idle(&self) {
        loop {
            let handles: Vec<JoinHandle<()>> =
                std::mem::take(&mut *self.tasks.lock().expect("scanner task list lock"));

            if handles.is_empty() {
                return;
            }

            for handle in handles {
                handle.await.ok();
            }
        }
    }
}

pub async fn run(pool: &PgPool, source_id: i64) {
    if let Err(error) = run_scan(pool, source_id).await {
        tracing::error!("scan of source {source_id} failed: {error:#}");
    }
}

async fn run_scan(pool: &PgPool, source_id: i64) -> anyhow::Result<()> {
    let Some(path) = sources::transition(pool, source_id, SourceEvent::StartScan).await? else {
        tracing::warn!("source {source_id} is missing or already scanning");
        return Ok(());
    };

    let event = match scan(pool, source_id, Path::new(&path)).await {
        Ok(()) => SourceEvent::Done,
        Err(error) => {
            tracing::error!("scan of {path} failed: {error:#}");
            SourceEvent::Error
        }
    };

    sources::transition(pool, source_id, event).await?;

    Ok(())
}

async fn scan(pool: &PgPool, source_id: i64, root: &Path) -> anyhow::Result<()> {
    prune(pool).await?;

    let root = root.to_path_buf();
    let files = tokio::task::spawn_blocking(move || find_mp3s(&root)).await??;

    for file in files {
        scan_file(pool, source_id, &file).await?;
    }

    Ok(())
}

/// Removes rows for files that can no longer be read, across every source.
async fn prune(pool: &PgPool) -> anyhow::Result<()> {
    let known: Vec<(i64, String)> = sqlx::query_as("SELECT id, filepath FROM mp3s")
        .fetch_all(pool)
        .await?;

    let missing: Vec<i64> = tokio::task::spawn_blocking(move || {
        known
            .into_iter()
            .filter(|(_, filepath)| {
                let readable = std::fs::File::open(filepath).is_ok();
                if !readable {
                    tracing::warn!("cannot find known file {filepath}");
                }
                !readable
            })
            .map(|(id, _)| id)
            .collect()
    })
    .await?;

    sqlx::query("DELETE FROM mp3s WHERE id = ANY($1)")
        .bind(&missing)
        .execute(pool)
        .await?;

    Ok(())
}

fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    entry.depth() > 0 && entry.file_name().to_string_lossy().starts_with('.')
}

/// Every non-hidden `*.mp3` under the root, any case, sorted by path.
pub fn find_mp3s(root: &Path) -> anyhow::Result<Vec<PathBuf>> {
    std::fs::read_dir(root).with_context(|| format!("reading {}", root.display()))?;

    let files = walkdir::WalkDir::new(root)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|entry| !is_hidden(entry))
        .filter_map(|entry| match entry {
            Ok(entry) => Some(entry),
            Err(error) => {
                tracing::warn!("skipping unreadable entry: {error}");
                None
            }
        })
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .to_lowercase()
                .ends_with(".mp3")
        })
        .map(walkdir::DirEntry::into_path)
        .collect();

    Ok(files)
}

pub fn file_hash(path: &Path) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 4096];

    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(hex::encode(hasher.finalize()))
}

fn tag_number(value: Option<u32>) -> i32 {
    value
        .and_then(|number| i32::try_from(number).ok())
        .unwrap_or(0)
}

struct Resolved {
    title: String,
    artist_id: i64,
    album_id: i64,
}

async fn resolve(
    pool: &PgPool,
    filepath: &str,
    data: &TagData,
    mode: ScanMode,
) -> sqlx::Result<Resolved> {
    let names = filename::resolve(
        filepath,
        data.title.as_deref(),
        data.artist.as_deref(),
        mode,
    );

    let artist = names::normalize(names.artist.as_deref());
    let album = names::normalize(data.album.as_deref());

    let artist_id = library::find_or_create_artist(pool, &artist).await?;
    let album_id = library::find_or_create_album(pool, artist_id, &album).await?;

    Ok(Resolved {
        title: names.title.unwrap_or_default(),
        artist_id,
        album_id,
    })
}

async fn scan_file(pool: &PgPool, source_id: i64, file: &Path) -> anyhow::Result<()> {
    let filepath = file.to_string_lossy().to_string();

    let path = file.to_path_buf();
    let data = match tokio::task::spawn_blocking(move || tags::read(&path)).await? {
        Ok(data) => data,
        Err(error) => {
            tracing::warn!("skipping {filepath}: {error:#}");
            return Ok(());
        }
    };

    let existing: Option<(i64, bool)> =
        sqlx::query_as("SELECT id, file_hash IS NULL FROM mp3s WHERE filepath = $1 LIMIT 1")
            .bind(&filepath)
            .fetch_optional(pool)
            .await?;

    let mode = if existing.is_some() {
        ScanMode::Update
    } else {
        ScanMode::Create
    };
    let resolved = resolve(pool, &filepath, &data, mode).await?;

    let needs_hash = existing.is_none_or(|(_, hash_missing)| hash_missing);
    let hash = if needs_hash {
        let path = file.to_path_buf();
        Some(tokio::task::spawn_blocking(move || file_hash(&path)).await??)
    } else {
        None
    };

    let written = if let Some((id, _)) = existing {
        sqlx::query(
            "UPDATE mp3s SET artist_id = $1, album_id = $2, title = $3, length = $4, genre = $5, \
             year = $6, track = $7, comment = $8, file_hash = COALESCE(file_hash, $9), \
             updated_at = CURRENT_TIMESTAMP WHERE id = $10",
        )
        .bind(resolved.artist_id)
        .bind(resolved.album_id)
        .bind(&resolved.title)
        .bind(data.length_seconds)
        .bind(&data.genre)
        .bind(tag_number(data.year))
        .bind(tag_number(data.track))
        .bind(&data.comment)
        .bind(hash)
        .bind(id)
        .execute(pool)
        .await
    } else {
        sqlx::query(
            "INSERT INTO mp3s (source_id, artist_id, album_id, filepath, title, length, genre, \
             year, track, comment, file_hash) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(source_id)
        .bind(resolved.artist_id)
        .bind(resolved.album_id)
        .bind(&filepath)
        .bind(&resolved.title)
        .bind(data.length_seconds)
        .bind(&data.genre)
        .bind(tag_number(data.year))
        .bind(tag_number(data.track))
        .bind(&data.comment)
        .bind(hash)
        .execute(pool)
        .await
    };

    skip_duplicate(written.map(|_| ()), &filepath)?;

    Ok(())
}

/// A file with the same artist, album, title, and length as another is skipped.
fn skip_duplicate(written: sqlx::Result<()>, filepath: &str) -> sqlx::Result<()> {
    match written {
        Err(error) if is_unique_violation(&error) => {
            tracing::warn!(
                "skipping {filepath}: same artist, album, title, and length as another file"
            );
            Ok(())
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test]
    async fn unique_violations_are_skipped(pool: PgPool) {
        sqlx::query("INSERT INTO artists (name) VALUES ('Band')")
            .execute(&pool)
            .await
            .unwrap();

        let error = sqlx::query("INSERT INTO artists (name) VALUES ('Band')")
            .execute(&pool)
            .await
            .unwrap_err();

        assert!(skip_duplicate(Err(error), "/a.mp3").is_ok());
    }

    #[test]
    fn other_database_errors_are_not_skipped() {
        assert!(skip_duplicate(Err(sqlx::Error::RowNotFound), "/a.mp3").is_err());
    }

    #[test]
    fn tag_numbers_default_to_zero() {
        assert_eq!(tag_number(None), 0);
        assert_eq!(tag_number(Some(7)), 7);
        assert_eq!(tag_number(Some(u32::MAX)), 0);
    }

    #[test]
    fn file_hash_is_the_sha256_of_the_contents() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("abc.txt");
        std::fs::write(&path, b"abc").unwrap();

        assert_eq!(
            file_hash(&path).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn file_hash_of_a_missing_file_is_an_error() {
        let directory = tempfile::tempdir().unwrap();

        assert!(file_hash(&directory.path().join("gone")).is_err());
    }

    #[test]
    fn find_mp3s_matches_any_case_skips_hidden_entries_and_sorts() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        std::fs::create_dir_all(root.join("b")).unwrap();
        std::fs::create_dir_all(root.join(".hidden")).unwrap();
        for name in [
            "b/two.MP3",
            "a.mp3",
            ".dot.mp3",
            ".hidden/x.mp3",
            "notes.txt",
        ] {
            std::fs::write(root.join(name), b"").unwrap();
        }

        let found: Vec<String> = find_mp3s(root)
            .unwrap()
            .into_iter()
            .map(|path| {
                path.strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            })
            .collect();

        assert_eq!(found, vec!["a.mp3", "b/two.MP3"]);
    }

    #[test]
    fn find_mp3s_on_a_missing_root_is_an_error() {
        let directory = tempfile::tempdir().unwrap();

        assert!(find_mp3s(&directory.path().join("gone")).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn find_mp3s_skips_unreadable_directories() {
        use std::os::unix::fs::PermissionsExt;

        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        let locked = root.join("locked");
        std::fs::create_dir_all(&locked).unwrap();
        std::fs::write(locked.join("inside.mp3"), b"").unwrap();
        std::fs::write(root.join("outside.mp3"), b"").unwrap();
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();

        let found = find_mp3s(root);

        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();

        assert_eq!(found.unwrap(), vec![root.join("outside.mp3")]);
    }
}
