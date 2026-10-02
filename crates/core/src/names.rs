pub const UNKNOWN: &str = "Unknown";

pub const RECENTLY_PLAYED: &str = "Recently Played";

/// Artist and album names: trimmed, with blank becoming `Unknown`.
pub fn normalize(name: Option<&str>) -> String {
    match name.map(str::trim) {
        Some(trimmed) if !trimmed.is_empty() => trimmed.to_string(),
        _ => UNKNOWN.to_string(),
    }
}

/// The name prefilled for a new playlist made from a `key:"value"` search.
pub fn playlist_name_from_query(query: Option<&str>) -> String {
    let Some(query) = query else {
        return String::new();
    };

    let fields: Vec<&str> = query.split(':').collect();

    match fields.as_slice() {
        [_, value] => value.replace('"', ""),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_name_is_unknown() {
        assert_eq!(normalize(None), "Unknown");
    }

    #[test]
    fn blank_name_is_unknown() {
        assert_eq!(normalize(Some("   ")), "Unknown");
    }

    #[test]
    fn name_is_trimmed() {
        assert_eq!(normalize(Some("  Band ")), "Band");
    }

    #[test]
    fn playlist_name_is_empty_without_a_query() {
        assert_eq!(playlist_name_from_query(None), "");
    }

    #[test]
    fn playlist_name_comes_from_a_filter_value() {
        assert_eq!(
            playlist_name_from_query(Some("album:\"Abbey Road\"")),
            "Abbey Road"
        );
    }

    #[test]
    fn playlist_name_is_empty_for_a_plain_query() {
        assert_eq!(playlist_name_from_query(Some("help")), "");
    }

    #[test]
    fn playlist_name_is_empty_when_the_query_has_two_colons() {
        assert_eq!(playlist_name_from_query(Some("a:b:c")), "");
    }
}
