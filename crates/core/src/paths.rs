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
