const HEX_DIGITS: &[u8; 16] = b"0123456789ABCDEF";

/// Percent-encodes a query string value. Unreserved characters pass through.
pub fn encode_component(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());

    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push(char::from(HEX_DIGITS[usize::from(byte >> 4)]));
            encoded.push(char::from(HEX_DIGITS[usize::from(byte & 0x0F)]));
        }
    }

    encoded
}

/// The list request for the MP3s view: a search when there is a query.
pub fn mp3s(query: Option<&str>, sort: Option<&str>) -> String {
    let sort = encode_component(sort.unwrap_or_default());

    match query.filter(|text| !text.is_empty()) {
        Some(text) => format!("/api/mp3s/search?q={}&sort={sort}", encode_component(text)),
        None => format!("/api/mp3s?sort={sort}"),
    }
}

/// The list request for the Albums view.
pub fn albums(query: Option<&str>, sort: Option<&str>) -> String {
    with_query("/api/albums", query, sort)
}

/// The list request for the Artists view.
pub fn artists(query: Option<&str>, sort: Option<&str>) -> String {
    with_query("/api/artists", query, sort)
}

fn with_query(path: &str, query: Option<&str>, sort: Option<&str>) -> String {
    let params: Vec<String> = [("q", query), ("sort", sort)]
        .into_iter()
        .filter_map(|(name, value)| {
            value
                .filter(|text| !text.is_empty())
                .map(|text| format!("{name}={}", encode_component(text)))
        })
        .collect();

    if params.is_empty() {
        path.to_string()
    } else {
        format!("{path}?{}", params.join("&"))
    }
}

pub fn play(mp3_id: i64) -> String {
    format!("/api/mp3s/{mp3_id}/play")
}

/// The search a click on an album or artist name runs.
pub fn filter_query(field: &str, name: &str) -> String {
    format!("{field}:\"{name}\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unreserved_characters_are_kept() {
        assert_eq!(encode_component("Az09-_.~"), "Az09-_.~");
    }

    #[test]
    fn other_characters_are_percent_encoded_as_utf8() {
        assert_eq!(encode_component("a b&\"é"), "a%20b%26%22%C3%A9");
    }

    #[test]
    fn albums_without_a_query_or_sort_lists_every_album() {
        assert_eq!(albums(None, None), "/api/albums");
        assert_eq!(albums(Some(""), Some("")), "/api/albums");
    }

    #[test]
    fn albums_with_a_query_searches() {
        assert_eq!(
            albums(Some("iron maiden"), None),
            "/api/albums?q=iron%20maiden"
        );
    }

    #[test]
    fn albums_with_a_sort_and_a_query_send_both() {
        assert_eq!(
            albums(Some("dirt"), Some("songs_desc")),
            "/api/albums?q=dirt&sort=songs_desc"
        );
    }

    #[test]
    fn artists_with_and_without_a_query_or_sort() {
        assert_eq!(artists(None, None), "/api/artists");
        assert_eq!(
            artists(Some("artist:\"Band\""), None),
            "/api/artists?q=artist%3A%22Band%22"
        );
        assert_eq!(
            artists(None, Some("albums_asc")),
            "/api/artists?sort=albums_asc"
        );
    }

    #[test]
    fn no_query_lists_everything() {
        assert_eq!(mp3s(None, Some("title_asc")), "/api/mp3s?sort=title_asc");
    }

    #[test]
    fn empty_query_lists_everything() {
        assert_eq!(mp3s(Some(""), None), "/api/mp3s?sort=");
    }

    #[test]
    fn query_searches() {
        assert_eq!(
            mp3s(Some("artist:\"AC DC\""), Some("track_desc")),
            "/api/mp3s/search?q=artist%3A%22AC%20DC%22&sort=track_desc"
        );
    }

    #[test]
    fn play_path_names_the_mp3() {
        assert_eq!(play(12), "/api/mp3s/12/play");
    }

    #[test]
    fn filter_query_quotes_the_name() {
        assert_eq!(filter_query("album", "Abbey Road"), "album:\"Abbey Road\"");
    }
}
