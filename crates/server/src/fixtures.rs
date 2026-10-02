use std::io::Write;
use std::path::Path;

use lofty::config::WriteOptions;
use lofty::tag::{Accessor, Tag, TagExt, TagType};

/// MPEG-1 Layer III, 128 kbps, 44.1 kHz, mono, no CRC, no padding.
const FRAME_HEADER: [u8; 4] = [0xFF, 0xFB, 0x90, 0xC4];
const FRAME_BYTES: usize = 417;
const SAMPLES_PER_FRAME: u64 = 1152;
const SAMPLE_RATE: u64 = 44_100;

#[derive(Debug, Default, Clone, Copy)]
pub struct FixtureTags<'a> {
    pub title: Option<&'a str>,
    pub artist: Option<&'a str>,
    pub album: Option<&'a str>,
    pub genre: Option<&'a str>,
    pub year: Option<u32>,
    pub track: Option<u32>,
    pub comment: Option<&'a str>,
}

/// Writes silent frames: a zeroed side-info block decodes as silence. One frame
/// past the length keeps lofty's estimate above the whole second once a tag
/// is prepended.
pub fn write_silent_mp3(path: &Path, seconds: u64) -> std::io::Result<()> {
    let frame_count = (seconds * SAMPLE_RATE).div_ceil(SAMPLES_PER_FRAME) + 1;

    let mut frame = vec![0_u8; FRAME_BYTES];
    frame[..FRAME_HEADER.len()].copy_from_slice(&FRAME_HEADER);

    let mut file = std::fs::File::create(path)?;
    for _ in 0..frame_count {
        file.write_all(&frame)?;
    }

    file.flush()
}

pub fn write_tagged_mp3(path: &Path, seconds: u64, tags: &FixtureTags<'_>) -> anyhow::Result<()> {
    write_silent_mp3(path, seconds)?;

    let mut tag = Tag::new(TagType::Id3v2);

    if let Some(title) = tags.title {
        tag.set_title(title.to_string());
    }
    if let Some(artist) = tags.artist {
        tag.set_artist(artist.to_string());
    }
    if let Some(album) = tags.album {
        tag.set_album(album.to_string());
    }
    if let Some(genre) = tags.genre {
        tag.set_genre(genre.to_string());
    }
    if let Some(year) = tags.year {
        tag.set_year(year);
    }
    if let Some(track) = tags.track {
        tag.set_track(track);
    }
    if let Some(comment) = tags.comment {
        tag.set_comment(comment.to_string());
    }

    tag.save_to_path(path, WriteOptions::default())?;

    Ok(())
}

/// The library the browser tests scan. Each song outlasts a browser test, so
/// playback only advances when a test ends a track.
pub fn write_demo_library(directory: &Path) -> anyhow::Result<()> {
    let songs = [
        (
            "The Testers/First Album/01 Opening Song.mp3",
            61,
            "Opening Song",
            "The Testers",
            "First Album",
            1,
        ),
        (
            "The Testers/First Album/02 Second Song.mp3",
            62,
            "Second Song",
            "The Testers",
            "First Album",
            2,
        ),
        (
            "Other Band/Encore.mp3",
            63,
            "Encore",
            "Other Band",
            "Live",
            1,
        ),
    ];

    for (relative, seconds, title, artist, album, track) in songs {
        let path = directory.join(relative);
        std::fs::create_dir_all(path.parent().unwrap_or(directory))?;

        let tags = FixtureTags {
            title: Some(title),
            artist: Some(artist),
            album: Some(album),
            track: Some(track),
            ..FixtureTags::default()
        };
        write_tagged_mp3(&path, seconds, &tags)?;
    }

    write_silent_mp3(&directory.join("Filename Artist - Filename Song.mp3"), 64)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tags;

    #[test]
    fn silent_file_has_the_requested_length() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("five.mp3");

        write_silent_mp3(&path, 5).unwrap();

        assert_eq!(tags::read(&path).unwrap().length_seconds, 5);
    }

    #[test]
    fn tags_left_out_stay_unset() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("bare-tags.mp3");

        write_tagged_mp3(&path, 1, &FixtureTags::default()).unwrap();

        assert_eq!(tags::read(&path).unwrap().title, None);
    }

    #[test]
    fn silent_file_in_a_missing_directory_is_an_error() {
        let directory = tempfile::tempdir().unwrap();

        assert!(write_silent_mp3(&directory.path().join("no/such/dir.mp3"), 1).is_err());
    }

    #[test]
    fn tagged_file_in_a_missing_directory_is_an_error() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("no/such/dir.mp3");

        assert!(write_tagged_mp3(&path, 1, &FixtureTags::default()).is_err());
    }

    #[test]
    fn demo_library_writes_four_files() {
        let directory = tempfile::tempdir().unwrap();

        write_demo_library(directory.path()).unwrap();

        let count = walkdir::WalkDir::new(directory.path())
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file())
            .count();

        assert_eq!(count, 4);
    }

    #[test]
    fn demo_library_in_an_unwritable_place_is_an_error() {
        let directory = tempfile::tempdir().unwrap();
        let blocker = directory.path().join("file");
        std::fs::write(&blocker, b"").unwrap();

        assert!(write_demo_library(&blocker).is_err());
    }
}
