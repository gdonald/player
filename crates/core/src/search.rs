#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SearchQuery {
    pub parts: Vec<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
}

const ALLOWED_PUNCTUATION: &str = "-_ .,():'\"";

pub fn clean(text: &str) -> String {
    text.chars()
        .filter(|character| {
            character.is_ascii_alphanumeric() || ALLOWED_PUNCTUATION.contains(*character)
        })
        .collect()
}

pub fn parse(query: &str) -> SearchQuery {
    let (artists, remainder) = extract_quoted(query, "artist:\"");
    let remainder = remainder.trim();

    let (albums, remainder) = extract_quoted(remainder, "album:\"");
    let remainder = clean(&remainder.trim().to_lowercase());

    let (phrases, remainder) = extract_quoted(&remainder, "\"");

    let mut parts: Vec<String> = remainder.split_whitespace().map(String::from).collect();
    parts.extend(phrases);
    parts.retain(|part| !part.trim().is_empty());

    SearchQuery {
        parts,
        artist: artists.into_iter().next().map(|artist| clean(&artist)),
        album: albums.into_iter().next().map(|album| clean(&album)),
    }
}

/// Returns each `prefix..."` capture and the text with those matches removed.
/// A capture holds at least one character and ends at the next double quote.
fn extract_quoted(text: &str, prefix: &str) -> (Vec<String>, String) {
    let mut captures = Vec::new();
    let mut remainder = String::new();
    let mut cursor = 0;

    while let Some(offset) = text[cursor..].find(prefix) {
        let start = cursor + offset;
        let content_start = start + prefix.len();

        let Some(first_char) = text[content_start..].chars().next() else {
            break;
        };

        let search_from = content_start + first_char.len_utf8();
        let Some(close_offset) = text[search_from..].find('"') else {
            break;
        };

        let close = search_from + close_offset;
        remainder.push_str(&text[cursor..start]);
        captures.push(text[content_start..close].to_string());
        cursor = close + 1;
    }

    remainder.push_str(&text[cursor..]);
    (captures, remainder)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(list: &[&str]) -> Vec<String> {
        list.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn empty_query_has_no_parts_or_filters() {
        assert_eq!(parse(""), SearchQuery::default());
    }

    #[test]
    fn words_are_lowercased_and_split_on_whitespace() {
        assert_eq!(parse("Hello  World").parts, words(&["hello", "world"]));
    }

    #[test]
    fn single_word_without_spaces_is_one_part() {
        assert_eq!(parse("Beatles").parts, words(&["beatles"]));
    }

    #[test]
    fn quoted_phrases_are_kept_whole_and_appended_after_words() {
        assert_eq!(
            parse("red \"yellow submarine\" blue").parts,
            words(&["red", "blue", "yellow submarine"])
        );
    }

    #[test]
    fn query_made_only_of_a_phrase_yields_the_phrase() {
        assert_eq!(parse("\"let it be\"").parts, words(&["let it be"]));
    }

    #[test]
    fn whitespace_only_phrase_is_dropped() {
        assert_eq!(parse("\" \"").parts, Vec::<String>::new());
    }

    #[test]
    fn characters_outside_the_allowed_set_are_stripped() {
        assert_eq!(parse("ab!c% d&e").parts, words(&["abc", "de"]));
    }

    #[test]
    fn non_ascii_letters_are_stripped() {
        assert_eq!(parse("café").parts, words(&["caf"]));
    }

    #[test]
    fn artist_filter_is_extracted_and_removed_from_the_parts() {
        let query = parse("artist:\"The Beatles\" help");

        assert_eq!(query.artist.as_deref(), Some("The Beatles"));
        assert_eq!(query.parts, words(&["help"]));
    }

    #[test]
    fn album_filter_is_extracted_and_removed_from_the_parts() {
        let query = parse("album:\"Abbey Road\"");

        assert_eq!(query.album.as_deref(), Some("Abbey Road"));
        assert_eq!(query.parts, Vec::<String>::new());
    }

    #[test]
    fn both_filters_are_extracted_and_only_the_first_of_each_is_used() {
        let query = parse("album:\"One\" artist:\"Two\" album:\"Three\" artist:\"Four\"");

        assert_eq!(query.artist.as_deref(), Some("Two"));
        assert_eq!(query.album.as_deref(), Some("One"));
        assert_eq!(query.parts, Vec::<String>::new());
    }

    #[test]
    fn filter_values_are_cleaned_but_keep_their_case() {
        assert_eq!(parse("artist:\"AC/DC!\"").artist.as_deref(), Some("ACDC"));
    }

    #[test]
    fn filter_without_a_closing_quote_stays_in_the_text() {
        let query = parse("artist:\"open");

        assert_eq!(query.artist, None);
        assert_eq!(query.parts, words(&["artist:\"open"]));
    }

    #[test]
    fn filter_with_empty_quotes_and_no_later_quote_is_not_a_filter() {
        assert_eq!(parse("artist:\"\"").artist, None);
    }

    #[test]
    fn filter_prefix_at_the_end_of_the_text_is_not_a_filter() {
        assert_eq!(parse("x artist:\"").artist, None);
    }

    #[test]
    fn capture_spans_a_multibyte_first_character() {
        assert_eq!(parse("artist:\"é\"").artist.as_deref(), Some(""));
    }
}
