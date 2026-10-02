#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanMode {
    Create,
    Update,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TitleAndArtist {
    pub title: Option<String>,
    pub artist: Option<String>,
}

fn is_blank(value: Option<&str>) -> bool {
    value.is_none_or(|text| text.trim().is_empty())
}

fn file_stem(filepath: &str) -> String {
    filepath
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .replace(".mp3", "")
}

/// Splits like Ruby's `String#split('-')`, which drops trailing empty fields.
fn split_on_dashes(stem: &str) -> Vec<String> {
    let mut fields: Vec<&str> = stem.split('-').collect();

    while fields.last().is_some_and(|field| field.is_empty()) {
        fields.pop();
    }

    fields
        .into_iter()
        .map(|field| field.trim().to_string())
        .collect()
}

/// Fills a blank tag title or artist from a `Artist - Title.mp3` filename.
pub fn resolve(
    filepath: &str,
    tag_title: Option<&str>,
    tag_artist: Option<&str>,
    mode: ScanMode,
) -> TitleAndArtist {
    let mut title = tag_title.map(String::from);
    let mut artist = tag_artist.map(String::from);

    if is_blank(tag_title) || is_blank(tag_artist) {
        let stem = file_stem(filepath);
        let fields = split_on_dashes(&stem);

        if let [artist_field, title_field] = fields.as_slice() {
            artist = Some(artist_field.clone());
            title = Some(title_field.clone());
        } else if mode == ScanMode::Create || is_blank(title.as_deref()) {
            title = Some(stem);
        }
    }

    TitleAndArtist { title, artist }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolved(title: &str, artist: &str) -> TitleAndArtist {
        TitleAndArtist {
            title: Some(title.to_string()),
            artist: Some(artist.to_string()),
        }
    }

    #[test]
    fn tag_values_are_kept_when_title_and_artist_are_present() {
        assert_eq!(
            resolve(
                "/m/Other - Name.mp3",
                Some("Song"),
                Some("Band"),
                ScanMode::Create
            ),
            resolved("Song", "Band")
        );
    }

    #[test]
    fn artist_dash_title_filename_fills_both_on_create() {
        assert_eq!(
            resolve("/m/Band - Song.mp3", None, None, ScanMode::Create),
            resolved("Song", "Band")
        );
    }

    #[test]
    fn artist_dash_title_filename_overrides_a_present_title_when_artist_is_blank() {
        assert_eq!(
            resolve(
                "/m/Band - Song.mp3",
                Some("Tagged"),
                Some(" "),
                ScanMode::Update
            ),
            resolved("Song", "Band")
        );
    }

    #[test]
    fn filename_without_a_dash_becomes_the_title_on_create() {
        assert_eq!(
            resolve("/m/Song.mp3", None, Some("Band"), ScanMode::Create),
            resolved("Song", "Band")
        );
    }

    #[test]
    fn filename_with_three_fields_becomes_the_title_on_create() {
        assert_eq!(
            resolve("/m/A - B - C.mp3", Some("Tagged"), None, ScanMode::Create),
            TitleAndArtist {
                title: Some("A - B - C".to_string()),
                artist: None,
            }
        );
    }

    #[test]
    fn filename_with_three_fields_keeps_the_tag_title_on_update() {
        assert_eq!(
            resolve("/m/A - B - C.mp3", Some("Tagged"), None, ScanMode::Update),
            TitleAndArtist {
                title: Some("Tagged".to_string()),
                artist: None,
            }
        );
    }

    #[test]
    fn blank_title_takes_the_filename_on_update() {
        assert_eq!(
            resolve("/m/Song.mp3", None, Some("Band"), ScanMode::Update),
            resolved("Song", "Band")
        );
    }

    #[test]
    fn trailing_dash_is_dropped_before_counting_fields() {
        assert_eq!(
            resolve("/m/Band-Song-.mp3", None, None, ScanMode::Create),
            resolved("Song", "Band")
        );
    }

    #[test]
    fn uppercase_extension_is_not_removed() {
        assert_eq!(
            resolve("/m/Song.MP3", None, None, ScanMode::Create)
                .title
                .as_deref(),
            Some("Song.MP3")
        );
    }
}
