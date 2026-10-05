use std::borrow::Cow;
use std::fs::File;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use anyhow::Context;
use lofty::config::{ParseOptions, WriteOptions};
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::id3::v2::{Frame, Id3v2Tag, Id3v2Version};
use lofty::mpeg::MpegFile;
use lofty::tag::{Accessor, Tag, TagExt};

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

/// The fields the edit page changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edits<'a> {
    pub title: &'a str,
    pub artist: &'a str,
    pub album: &'a str,
    pub track: Option<u32>,
}

/// The copy an edit is written to before it replaces the original, in the same
/// directory so the rename stays on one filesystem.
fn staging_path(path: &Path) -> PathBuf {
    let name = path.file_name().unwrap_or_default().to_string_lossy();

    path.with_file_name(format!(".{name}.player-edit"))
}

/// The `ID3v2` frames the edit page writes.
const EDITED_FRAMES: [&str; 4] = ["TIT2", "TPE1", "TALB", "TRCK"];

/// The frames the edit page does not change, in file order.
fn untouched_frames(tag: &Id3v2Tag) -> Vec<Frame<'static>> {
    tag.clone()
        .into_iter()
        .filter(|frame| !EDITED_FRAMES.contains(&frame.id_str()))
        .collect()
}

/// The bytes after a leading `ID3v2` tag: the audio and any trailing tags,
/// which an edit leaves alone. The tag size is a syncsafe integer, and a
/// footer adds ten bytes.
fn after_id3v2(bytes: &[u8]) -> &[u8] {
    let Some(header) = bytes.get(..10).filter(|header| header.starts_with(b"ID3")) else {
        return bytes;
    };

    let tag_size = header[6..10]
        .iter()
        .fold(0_usize, |size, byte| (size << 7) | usize::from(byte & 0x7F));
    let footer_size = if header[5] & 0x10 == 0 { 0 } else { 10 };

    bytes.get(10 + tag_size + footer_size..).unwrap_or_default()
}

/// Writes the edits into the file's `ID3v2` tag. The tag is edited in place, so
/// its other frames and its version stay as they were, and the other tags and
/// the audio are not touched. The edit goes to a copy, which is read back and
/// checked against the original before it replaces it, so a failed, wrong, or
/// interrupted write leaves the original whole.
pub fn write_edits(path: &Path, edits: &Edits<'_>) -> anyhow::Result<()> {
    let original = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let mut mpeg = MpegFile::read_from(&mut Cursor::new(&original), ParseOptions::new())
        .with_context(|| format!("reading {}", path.display()))?;

    let original_tag = mpeg.remove_id3v2().unwrap_or_default();
    let options =
        WriteOptions::default().use_id3v23(original_tag.original_version() == Id3v2Version::V3);

    let mut tag = original_tag.clone();
    tag.set_title(edits.title.to_string());
    tag.set_artist(edits.artist.to_string());
    tag.set_album(edits.album.to_string());
    match edits.track.filter(|track| *track > 0) {
        Some(track) => tag.set_track(track),
        None => tag.remove_track(),
    }

    let staging = staging_path(path);
    let staged = stage(path, &staging, &tag, options).and_then(|()| {
        verify(&staging, &original, &original_tag, edits)?;
        std::fs::rename(&staging, path)?;

        Ok(())
    });

    if staged.is_err() {
        std::fs::remove_file(&staging).ok();
    }

    staged.with_context(|| format!("writing {}", path.display()))
}

fn stage(path: &Path, staging: &Path, tag: &Id3v2Tag, options: WriteOptions) -> anyhow::Result<()> {
    std::fs::copy(path, staging)?;
    tag.save_to_path(staging, options)?;
    File::open(staging)?.sync_all()?;

    Ok(())
}

/// Reads the written copy back and checks it against the original: the bytes
/// after the `ID3v2` tag are identical, the edited fields read back as
/// entered, and every other `ID3v2` frame is unchanged.
fn verify(
    staging: &Path,
    original: &[u8],
    original_tag: &Id3v2Tag,
    edits: &Edits<'_>,
) -> anyhow::Result<()> {
    let written = std::fs::read(staging)?;
    anyhow::ensure!(
        after_id3v2(&written) == after_id3v2(original),
        "the audio in the written copy differs from the original"
    );

    let tag = MpegFile::read_from(&mut Cursor::new(&written), ParseOptions::new())?
        .id3v2()
        .cloned()
        .context("the written copy has no ID3v2 tag")?;

    let read_back = (
        tag.title().map(Cow::into_owned),
        tag.artist().map(Cow::into_owned),
        tag.album().map(Cow::into_owned),
        tag.track(),
    );
    let entered = (
        Some(edits.title.to_string()),
        Some(edits.artist.to_string()),
        Some(edits.album.to_string()),
        edits.track.filter(|track| *track > 0),
    );
    anyhow::ensure!(
        read_back == entered,
        "the written copy reads back as {read_back:?} instead of {entered:?}"
    );

    anyhow::ensure!(
        untouched_frames(&tag) == untouched_frames(original_tag),
        "the written copy changed ID3v2 frames the edit does not touch"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use lofty::TextEncoding;
    use lofty::id3::v1::Id3v1Tag;
    use lofty::id3::v2::{
        CommentFrame, ExtendedTextFrame, FrameId, PopularimeterFrame, PrivateFrame,
        UnsynchronizedTextFrame,
    };
    use lofty::picture::{MimeType, Picture, PictureType};
    use lofty::tag::TagType;

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

    const EDITS: Edits<'static> = Edits {
        title: "New Title",
        artist: "New Band",
        album: "New Album",
        track: Some(4),
    };

    fn read_id3v2(path: &Path) -> Option<Id3v2Tag> {
        let mut file = File::open(path).unwrap();

        MpegFile::read_from(&mut file, ParseOptions::new())
            .unwrap()
            .id3v2()
            .cloned()
    }

    fn read_id3v1(path: &Path) -> Option<Id3v1Tag> {
        let mut file = File::open(path).unwrap();

        MpegFile::read_from(&mut file, ParseOptions::new())
            .unwrap()
            .id3v1()
            .cloned()
    }

    fn file_names(directory: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        names.sort();

        names
    }

    /// An `ID3v2` tag with the frames a ripped and tagged library carries:
    /// cover art, lyrics, two comments told apart by description, a `ReplayGain` value,
    /// a rating, a private frame, a genre, and a track with a total.
    fn library_tag() -> Id3v2Tag {
        let mut tag = Id3v2Tag::new();

        tag.set_title("Old Title".to_string());
        tag.set_artist("Old Band".to_string());
        tag.set_album("Old Album".to_string());
        tag.set_genre("Jazz".to_string());
        tag.set_track(3);
        tag.set_track_total(12);

        tag.insert_picture(Picture::new_unchecked(
            PictureType::CoverFront,
            Some(MimeType::Png),
            Some("Front cover".to_string()),
            vec![0x89, b'P', b'N', b'G', 1, 2, 3, 4, 5, 6, 7, 8],
        ));
        tag.insert(Frame::UnsynchronizedText(UnsynchronizedTextFrame::new(
            TextEncoding::UTF16,
            *b"eng",
            String::new(),
            "First line\nSecond line".to_string(),
        )));
        tag.insert(Frame::Comment(CommentFrame::new(
            TextEncoding::UTF16,
            *b"eng",
            "Ripped".to_string(),
            "From the 1999 pressing".to_string(),
        )));
        tag.insert(Frame::Comment(CommentFrame::new(
            TextEncoding::UTF16,
            *b"deu",
            "Notiz".to_string(),
            "Zweite Seite".to_string(),
        )));
        tag.insert(Frame::UserText(ExtendedTextFrame::new(
            TextEncoding::UTF16,
            "REPLAYGAIN_TRACK_GAIN".to_string(),
            "-6.20 dB".to_string(),
        )));
        tag.insert(Frame::Popularimeter(PopularimeterFrame::new(
            "listener@example.com".to_string(),
            196,
            42,
        )));
        tag.insert(Frame::Private(PrivateFrame::new(
            "com.example.library".to_string(),
            vec![1, 2, 3, 4],
        )));

        tag
    }

    fn write_library_mp3(path: &Path, version: Id3v2Version) {
        write_silent_mp3(path, 2).unwrap();

        library_tag()
            .save_to_path(
                path,
                WriteOptions::default().use_id3v23(version == Id3v2Version::V3),
            )
            .unwrap();
    }

    #[test]
    fn the_library_tag_carries_eight_frames_the_edit_page_does_not_change() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("song.mp3");
        write_library_mp3(&path, Id3v2Version::V4);

        let ids: Vec<String> = untouched_frames(&read_id3v2(&path).unwrap())
            .iter()
            .map(|frame| frame.id_str().to_string())
            .collect();

        assert_eq!(
            ids,
            vec![
                "TCON", "APIC", "USLT", "COMM", "COMM", "TXXX", "POPM", "PRIV"
            ]
        );
    }

    /// A copy of the original with the edits applied to its `ID3v2` tag, then
    /// `change` applied on top, standing in for what lofty wrote.
    fn written_copy(original: &Path, copy: &Path, change: impl FnOnce(&mut Id3v2Tag)) {
        std::fs::copy(original, copy).unwrap();

        let mut tag = read_id3v2(original).unwrap_or_default();
        tag.set_title(EDITS.title.to_string());
        tag.set_artist(EDITS.artist.to_string());
        tag.set_album(EDITS.album.to_string());
        tag.set_track(4);
        change(&mut tag);

        tag.save_to_path(copy, WriteOptions::default()).unwrap();
    }

    fn verify_copy(original: &Path, copy: &Path) -> anyhow::Result<()> {
        verify(
            copy,
            &std::fs::read(original).unwrap(),
            &read_id3v2(original).unwrap_or_default(),
            &EDITS,
        )
    }

    #[test]
    fn after_id3v2_is_the_whole_input_without_a_tag() {
        assert_eq!(after_id3v2(b"audio frames"), b"audio frames");
    }

    #[test]
    fn after_id3v2_skips_the_header_and_the_tag_body() {
        let bytes = b"ID3\x04\x00\x00\x00\x00\x00\x03tagaudio";

        assert_eq!(after_id3v2(bytes), b"audio");
    }

    #[test]
    fn after_id3v2_reads_the_size_as_a_syncsafe_integer() {
        let mut bytes = b"ID3\x04\x00\x00\x00\x00\x01\x00".to_vec();
        bytes.extend([0_u8; 128]);
        bytes.extend(b"audio");

        assert_eq!(after_id3v2(&bytes), b"audio");
    }

    #[test]
    fn after_id3v2_skips_a_footer() {
        let bytes = b"ID3\x04\x00\x10\x00\x00\x00\x03tag3DI\x04\x00\x10\x00\x00\x00\x03audio";

        assert_eq!(after_id3v2(bytes), b"audio");
    }

    #[test]
    fn after_id3v2_of_a_tag_longer_than_the_file_is_empty() {
        let bytes = b"ID3\x04\x00\x00\x00\x00\x00\x7Fshort";

        assert_eq!(after_id3v2(bytes), b"");
    }

    #[test]
    fn verify_accepts_a_correct_copy() {
        let directory = tempfile::tempdir().unwrap();
        let original = directory.path().join("song.mp3");
        let copy = directory.path().join("copy.mp3");
        write_library_mp3(&original, Id3v2Version::V4);
        written_copy(&original, &copy, |_| {});

        verify_copy(&original, &copy).unwrap();
    }

    #[test]
    fn verify_rejects_a_copy_whose_audio_differs() {
        let directory = tempfile::tempdir().unwrap();
        let original = directory.path().join("song.mp3");
        let copy = directory.path().join("copy.mp3");
        write_library_mp3(&original, Id3v2Version::V4);
        written_copy(&original, &copy, |_| {});

        let mut bytes = std::fs::read(&copy).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 0xFF;
        std::fs::write(&copy, bytes).unwrap();

        assert_eq!(
            verify_copy(&original, &copy).unwrap_err().to_string(),
            "the audio in the written copy differs from the original"
        );
    }

    #[test]
    fn verify_rejects_a_copy_with_no_id3v2_tag() {
        let directory = tempfile::tempdir().unwrap();
        let original = directory.path().join("bare.mp3");
        let copy = directory.path().join("copy.mp3");
        write_silent_mp3(&original, 1).unwrap();
        std::fs::copy(&original, &copy).unwrap();

        assert_eq!(
            verify_copy(&original, &copy).unwrap_err().to_string(),
            "the written copy has no ID3v2 tag"
        );
    }

    #[test]
    fn verify_rejects_a_copy_whose_fields_read_back_wrong() {
        let directory = tempfile::tempdir().unwrap();
        let original = directory.path().join("song.mp3");
        let copy = directory.path().join("copy.mp3");
        write_library_mp3(&original, Id3v2Version::V4);
        written_copy(&original, &copy, |tag| tag.set_title("Mangled".to_string()));

        assert!(
            verify_copy(&original, &copy)
                .unwrap_err()
                .to_string()
                .starts_with("the written copy reads back as")
        );
    }

    #[test]
    fn verify_rejects_a_copy_that_lost_another_frame() {
        let directory = tempfile::tempdir().unwrap();
        let original = directory.path().join("song.mp3");
        let copy = directory.path().join("copy.mp3");
        write_library_mp3(&original, Id3v2Version::V4);
        written_copy(&original, &copy, |tag| {
            tag.remove(&FrameId::Valid(Cow::Borrowed("APIC")))
                .for_each(drop);
        });

        assert_eq!(
            verify_copy(&original, &copy).unwrap_err().to_string(),
            "the written copy changed ID3v2 frames the edit does not touch"
        );
    }

    #[test]
    fn verify_rejects_a_copy_that_changed_another_frame() {
        let directory = tempfile::tempdir().unwrap();
        let original = directory.path().join("song.mp3");
        let copy = directory.path().join("copy.mp3");
        write_library_mp3(&original, Id3v2Version::V4);
        written_copy(&original, &copy, |tag| tag.set_genre("Rock".to_string()));

        assert_eq!(
            verify_copy(&original, &copy).unwrap_err().to_string(),
            "the written copy changed ID3v2 frames the edit does not touch"
        );
    }

    #[test]
    fn write_edits_sets_title_artist_album_and_track() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("song.mp3");
        write_library_mp3(&path, Id3v2Version::V4);

        write_edits(&path, &EDITS).unwrap();

        let data = read(&path).unwrap();
        assert_eq!(
            (
                data.title.as_deref(),
                data.artist.as_deref(),
                data.album.as_deref(),
                data.track
            ),
            (
                Some("New Title"),
                Some("New Band"),
                Some("New Album"),
                Some(4)
            )
        );
    }

    #[test]
    fn write_edits_adds_an_id3v2_tag_to_an_untagged_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("bare.mp3");
        write_silent_mp3(&path, 1).unwrap();

        write_edits(&path, &EDITS).unwrap();

        let data = read(&path).unwrap();
        assert_eq!(
            (
                data.title.as_deref(),
                data.artist.as_deref(),
                data.album.as_deref()
            ),
            (Some("New Title"), Some("New Band"), Some("New Album"))
        );
    }

    #[test]
    fn write_edits_leaves_the_audio_bytes_and_length_unchanged() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("song.mp3");
        write_library_mp3(&path, Id3v2Version::V4);
        let audio_before = after_id3v2(&std::fs::read(&path).unwrap()).to_vec();
        let length_before = read(&path).unwrap().length_seconds;

        write_edits(&path, &EDITS).unwrap();

        assert!(
            after_id3v2(&std::fs::read(&path).unwrap()) == audio_before,
            "the MPEG frames changed"
        );
        assert_eq!(read(&path).unwrap().length_seconds, length_before);
    }

    #[test]
    fn write_edits_keeps_every_other_id3v2_4_frame() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("song.mp3");
        write_library_mp3(&path, Id3v2Version::V4);
        let before = read_id3v2(&path).unwrap();

        write_edits(&path, &EDITS).unwrap();

        assert_eq!(
            untouched_frames(&read_id3v2(&path).unwrap()),
            untouched_frames(&before)
        );
    }

    #[test]
    fn write_edits_keeps_every_other_id3v2_3_frame() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("song.mp3");
        write_library_mp3(&path, Id3v2Version::V3);
        let before = read_id3v2(&path).unwrap();

        write_edits(&path, &EDITS).unwrap();

        assert_eq!(
            untouched_frames(&read_id3v2(&path).unwrap()),
            untouched_frames(&before)
        );
    }

    #[test]
    fn write_edits_keeps_an_id3v2_3_tag_at_version_3() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("song.mp3");
        write_library_mp3(&path, Id3v2Version::V3);

        write_edits(&path, &EDITS).unwrap();

        assert_eq!(
            read_id3v2(&path).unwrap().original_version(),
            Id3v2Version::V3
        );
    }

    #[test]
    fn write_edits_keeps_an_id3v2_4_tag_at_version_4() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("song.mp3");
        write_library_mp3(&path, Id3v2Version::V4);

        write_edits(&path, &EDITS).unwrap();

        assert_eq!(
            read_id3v2(&path).unwrap().original_version(),
            Id3v2Version::V4
        );
    }

    #[test]
    fn write_edits_keeps_the_comment_languages() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("song.mp3");
        write_library_mp3(&path, Id3v2Version::V4);

        write_edits(&path, &EDITS).unwrap();

        let languages: Vec<[u8; 3]> = read_id3v2(&path)
            .unwrap()
            .into_iter()
            .filter_map(|frame| match frame {
                Frame::Comment(comment) => Some(comment.language),
                _ => None,
            })
            .collect();
        assert_eq!(languages, vec![*b"eng", *b"deu"]);
    }

    #[test]
    fn write_edits_keeps_the_track_total() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("song.mp3");
        write_library_mp3(&path, Id3v2Version::V4);

        write_edits(&path, &EDITS).unwrap();

        let tag = read_id3v2(&path).unwrap();
        assert_eq!((tag.track(), tag.track_total()), (Some(4), Some(12)));
    }

    #[test]
    fn write_edits_without_a_track_removes_the_track_frame() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("song.mp3");
        write_library_mp3(&path, Id3v2Version::V4);

        write_edits(
            &path,
            &Edits {
                track: None,
                ..EDITS
            },
        )
        .unwrap();

        assert_eq!(read_id3v2(&path).unwrap().track(), None);
    }

    #[test]
    fn write_edits_treats_track_zero_as_no_track() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("song.mp3");
        write_library_mp3(&path, Id3v2Version::V4);

        write_edits(
            &path,
            &Edits {
                track: Some(0),
                ..EDITS
            },
        )
        .unwrap();

        assert_eq!(read_id3v2(&path).unwrap().track(), None);
    }

    #[test]
    fn write_edits_leaves_the_id3v1_tag_unchanged() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("song.mp3");
        write_library_mp3(&path, Id3v2Version::V4);

        let mut version_one = Tag::new(TagType::Id3v1);
        version_one.set_title("Version One Title".to_string());
        version_one.set_artist("Version One Band".to_string());
        version_one
            .save_to_path(&path, WriteOptions::default())
            .unwrap();
        let before = read_id3v1(&path).unwrap();

        write_edits(&path, &EDITS).unwrap();

        assert_eq!(read_id3v1(&path).unwrap(), before);
    }

    #[test]
    fn write_edits_twice_gives_the_same_file_as_once() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("song.mp3");
        write_library_mp3(&path, Id3v2Version::V4);

        write_edits(&path, &EDITS).unwrap();
        let once = std::fs::read(&path).unwrap();
        write_edits(&path, &EDITS).unwrap();

        assert!(
            std::fs::read(&path).unwrap() == once,
            "a repeat edit changed the file"
        );
    }

    #[test]
    fn write_edits_leaves_no_staging_file_behind() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("song.mp3");
        write_library_mp3(&path, Id3v2Version::V4);

        write_edits(&path, &EDITS).unwrap();

        assert_eq!(file_names(directory.path()), vec!["song.mp3"]);
    }

    #[test]
    fn write_edits_keeps_the_file_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("song.mp3");
        write_library_mp3(&path, Id3v2Version::V4);
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640)).unwrap();

        write_edits(&path, &EDITS).unwrap();

        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o640
        );
    }

    #[test]
    fn the_staging_copy_sits_beside_the_original_as_a_hidden_file() {
        assert_eq!(
            staging_path(Path::new("/music/Band/song.mp3")),
            PathBuf::from("/music/Band/.song.mp3.player-edit")
        );
    }

    #[test]
    fn write_edits_on_a_missing_file_is_an_error() {
        let directory = tempfile::tempdir().unwrap();

        assert!(write_edits(&directory.path().join("gone.mp3"), &EDITS).is_err());
    }

    #[test]
    fn write_edits_on_an_unparseable_file_is_an_error_and_leaves_it() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("broken.mp3");
        std::fs::write(&path, b"not audio").unwrap();

        assert!(write_edits(&path, &EDITS).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"not audio");
    }

    #[test]
    fn a_failed_write_leaves_the_original_unchanged_and_no_staging_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("locked.mp3");
        write_library_mp3(&path, Id3v2Version::V4);
        let before = std::fs::read(&path).unwrap();

        let mut permissions = std::fs::metadata(&path).unwrap().permissions();
        permissions.set_readonly(true);
        std::fs::set_permissions(&path, permissions).unwrap();

        assert!(write_edits(&path, &EDITS).is_err());
        assert!(
            std::fs::read(&path).unwrap() == before,
            "the original changed"
        );
        assert_eq!(file_names(directory.path()), vec!["locked.mp3"]);
    }
}
