#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortField {
    Artist,
    Album,
    Track,
    Title,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sort {
    Default,
    By(SortField, SortDirection),
}

pub fn parse(param: Option<&str>) -> Sort {
    let Some(param) = param else {
        return Sort::Default;
    };

    let mut segments = param.split('_');
    let field = match segments.next() {
        Some("artist") => SortField::Artist,
        Some("album") => SortField::Album,
        Some("track") => SortField::Track,
        Some("title") => SortField::Title,
        _ => return Sort::Default,
    };

    let direction = if param.rsplit('_').next() == Some("desc") {
        SortDirection::Descending
    } else {
        SortDirection::Ascending
    };

    Sort::By(field, direction)
}

/// A column of the Albums or Artists list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListField {
    Album,
    Artist,
    Albums,
    Songs,
}

/// The sort for the Albums or Artists list, or none for its default order.
pub fn parse_list(param: Option<&str>) -> Option<(ListField, SortDirection)> {
    let param = param?;

    let field = match param.split('_').next() {
        Some("album") => ListField::Album,
        Some("artist") => ListField::Artist,
        Some("albums") => ListField::Albums,
        Some("songs") => ListField::Songs,
        _ => return None,
    };

    let direction = if param.ends_with("_desc") {
        SortDirection::Descending
    } else {
        SortDirection::Ascending
    };

    Some((field, direction))
}

/// The sort parameter a column header click asks for, given the current one.
pub fn toggle(current: Option<&str>, column: &str) -> String {
    let ascending = format!("{column}_asc");

    if current == Some(ascending.as_str()) {
        format!("{column}_desc")
    } else {
        ascending
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_list_field_parses_in_both_directions() {
        assert_eq!(
            parse_list(Some("album_asc")),
            Some((ListField::Album, SortDirection::Ascending))
        );
        assert_eq!(
            parse_list(Some("artist_desc")),
            Some((ListField::Artist, SortDirection::Descending))
        );
        assert_eq!(
            parse_list(Some("albums_desc")),
            Some((ListField::Albums, SortDirection::Descending))
        );
        assert_eq!(
            parse_list(Some("songs_asc")),
            Some((ListField::Songs, SortDirection::Ascending))
        );
    }

    #[test]
    fn a_missing_or_unknown_list_sort_is_the_default_order() {
        assert_eq!(parse_list(None), None);
        assert_eq!(parse_list(Some("title_asc")), None);
    }

    #[test]
    fn missing_parameter_is_the_default_order() {
        assert_eq!(parse(None), Sort::Default);
    }

    #[test]
    fn unknown_field_is_the_default_order() {
        assert_eq!(parse(Some("genre_asc")), Sort::Default);
    }

    #[test]
    fn empty_parameter_is_the_default_order() {
        assert_eq!(parse(Some("")), Sort::Default);
    }

    #[test]
    fn each_field_parses_ascending() {
        assert_eq!(
            parse(Some("artist_asc")),
            Sort::By(SortField::Artist, SortDirection::Ascending)
        );
        assert_eq!(
            parse(Some("album_asc")),
            Sort::By(SortField::Album, SortDirection::Ascending)
        );
        assert_eq!(
            parse(Some("track_asc")),
            Sort::By(SortField::Track, SortDirection::Ascending)
        );
        assert_eq!(
            parse(Some("title_asc")),
            Sort::By(SortField::Title, SortDirection::Ascending)
        );
    }

    #[test]
    fn desc_suffix_parses_descending() {
        assert_eq!(
            parse(Some("title_desc")),
            Sort::By(SortField::Title, SortDirection::Descending)
        );
    }

    #[test]
    fn field_without_a_direction_is_ascending() {
        assert_eq!(
            parse(Some("album")),
            Sort::By(SortField::Album, SortDirection::Ascending)
        );
    }

    #[test]
    fn toggle_from_nothing_is_ascending() {
        assert_eq!(toggle(None, "title"), "title_asc");
    }

    #[test]
    fn toggle_from_ascending_is_descending() {
        assert_eq!(toggle(Some("title_asc"), "title"), "title_desc");
    }

    #[test]
    fn toggle_from_descending_is_ascending() {
        assert_eq!(toggle(Some("title_desc"), "title"), "title_asc");
    }

    #[test]
    fn toggle_from_another_column_is_ascending() {
        assert_eq!(toggle(Some("artist_asc"), "album"), "album_asc");
    }
}
