use std::path::Path;

use anyhow::Context;
use lofty::config::WriteOptions;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::tag::{Accessor, Tag, TagType};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TagData {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub year: Option<u32>,
    pub track: Option<u32>,
    pub comment: Option<String>,
    pub length_seconds: i32,
}

fn present(value: Option<std::borrow::Cow<'_, str>>) -> Option<String> {
    value
        .map(std::borrow::Cow::into_owned)
        .filter(|text| !text.is_empty())
}

/// Reads each field from the first tag that has it, primary tag first. taglib's
/// `FileRef` merged `ID3v2`, `APE`, and `ID3v1` tags this way.
pub fn read(path: &Path) -> anyhow::Result<TagData> {
    let tagged_file =
        lofty::read_from_path(path).with_context(|| format!("reading {}", path.display()))?;

    let primary = tagged_file.primary_tag_type();
    let mut tags: Vec<&Tag> = tagged_file.tags().iter().collect();
    tags.sort_by_key(|tag| tag.tag_type() != primary);

    let first_text = |field: fn(&Tag) -> Option<std::borrow::Cow<'_, str>>| {
        tags.iter().find_map(|tag| present(field(tag)))
    };
    let first_number = |field: fn(&Tag) -> Option<u32>| tags.iter().find_map(|tag| field(tag));

    let length_seconds =
        i32::try_from(tagged_file.properties().duration().as_secs()).unwrap_or(i32::MAX);

    Ok(TagData {
        title: first_text(Tag::title),
        artist: first_text(Tag::artist),
        album: first_text(Tag::album),
        genre: first_text(Tag::genre),
        year: first_number(Tag::year),
        track: first_number(Tag::track),
        comment: first_text(Tag::comment),
        length_seconds,
    })
}

/// Writes title, artist, and album into the `ID3v2` tag, keeping its other frames.
pub fn write_names(path: &Path, title: &str, artist: &str, album: &str) -> anyhow::Result<()> {
    let mut tagged_file =
        lofty::read_from_path(path).with_context(|| format!("reading {}", path.display()))?;

    if tagged_file.tag(TagType::Id3v2).is_none() {
        tagged_file.insert_tag(Tag::new(TagType::Id3v2));
    }

    let tag = tagged_file
        .tag_mut(TagType::Id3v2)
        .context("ID3v2 tag missing after insert")?;

    tag.set_title(title.to_string());
    tag.set_artist(artist.to_string());
    tag.set_album(album.to_string());

    tagged_file
        .save_to_path(path, WriteOptions::default())
        .with_context(|| format!("writing {}", path.display()))
}

#[cfg(test)]
mod tests {
    use lofty::tag::TagExt;

    use super::*;
    use crate::fixtures::{FixtureTags, write_silent_mp3, write_tagged_mp3};

    #[test]
    fn reads_every_field_and_the_length() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("song.mp3");

        write_tagged_mp3(
            &path,
            3,
            &FixtureTags {
                title: Some("Song"),
                artist: Some("Band"),
                album: Some("Record"),
                genre: Some("Rock"),
                year: Some(1999),
                track: Some(4),
                comment: Some("Live"),
            },
        )
        .unwrap();

        assert_eq!(
            read(&path).unwrap(),
            TagData {
                title: Some("Song".to_string()),
                artist: Some("Band".to_string()),
                album: Some("Record".to_string()),
                genre: Some("Rock".to_string()),
                year: Some(1999),
                track: Some(4),
                comment: Some("Live".to_string()),
                length_seconds: 3,
            }
        );
    }

    #[test]
    fn untagged_file_reads_as_empty_fields() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("bare.mp3");
        write_silent_mp3(&path, 2).unwrap();

        assert_eq!(
            read(&path).unwrap(),
            TagData {
                length_seconds: 2,
                ..TagData::default()
            }
        );
    }

    #[test]
    fn fields_missing_from_the_primary_tag_come_from_another_tag() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("two-tags.mp3");

        write_tagged_mp3(
            &path,
            1,
            &FixtureTags {
                title: Some("Primary"),
                ..FixtureTags::default()
            },
        )
        .unwrap();

        let mut version_one = Tag::new(TagType::Id3v1);
        version_one.set_title("Secondary".to_string());
        version_one.set_artist("Older Band".to_string());
        version_one
            .save_to_path(&path, WriteOptions::default())
            .unwrap();

        let data = read(&path).unwrap();

        assert_eq!(
            (data.title.as_deref(), data.artist.as_deref()),
            (Some("Primary"), Some("Older Band"))
        );
    }

    #[test]
    fn unparseable_file_is_an_error() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("broken.mp3");
        std::fs::write(&path, b"not audio").unwrap();

        assert!(read(&path).is_err());
    }

    #[test]
    fn write_names_adds_an_id3v2_tag_to_an_untagged_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("bare.mp3");
        write_silent_mp3(&path, 1).unwrap();

        write_names(&path, "New Title", "New Band", "New Album").unwrap();

        let data = read(&path).unwrap();

        assert_eq!(
            (data.title, data.artist, data.album),
            (
                Some("New Title".to_string()),
                Some("New Band".to_string()),
                Some("New Album".to_string())
            )
        );
    }

    #[test]
    fn write_names_keeps_other_frames() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("song.mp3");

        write_tagged_mp3(
            &path,
            1,
            &FixtureTags {
                title: Some("Old"),
                genre: Some("Jazz"),
                track: Some(9),
                ..FixtureTags::default()
            },
        )
        .unwrap();

        write_names(&path, "New", "Band", "Album").unwrap();

        let data = read(&path).unwrap();

        assert_eq!(
            (data.title.as_deref(), data.genre.as_deref(), data.track),
            (Some("New"), Some("Jazz"), Some(9))
        );
    }

    #[test]
    fn write_names_on_a_missing_file_is_an_error() {
        let directory = tempfile::tempdir().unwrap();

        assert!(write_names(&directory.path().join("gone.mp3"), "a", "b", "c").is_err());
    }

    #[test]
    fn write_names_on_a_read_only_file_is_an_error() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("locked.mp3");
        write_silent_mp3(&path, 1).unwrap();

        let mut permissions = std::fs::metadata(&path).unwrap().permissions();
        permissions.set_readonly(true);
        std::fs::set_permissions(&path, permissions).unwrap();

        assert!(write_names(&path, "a", "b", "c").is_err());
    }
}
