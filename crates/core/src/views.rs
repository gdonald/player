/// What the MP3s page lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListMode {
    Songs,
    Albums,
    Artists,
}

impl ListMode {
    pub const ALL: [ListMode; 3] = [ListMode::Songs, ListMode::Albums, ListMode::Artists];

    /// The stored form of the mode.
    pub fn name(self) -> &'static str {
        match self {
            ListMode::Songs => "songs",
            ListMode::Albums => "albums",
            ListMode::Artists => "artists",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ListMode::Songs => "Songs",
            ListMode::Albums => "Albums",
            ListMode::Artists => "Artists",
        }
    }

    /// A stored mode, or songs when it is missing or unknown.
    pub fn parse(stored: Option<&str>) -> ListMode {
        ListMode::ALL
            .into_iter()
            .find(|mode| Some(mode.name()) == stored)
            .unwrap_or(ListMode::Songs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_mode_reads_back_from_its_name() {
        for mode in ListMode::ALL {
            assert_eq!(ListMode::parse(Some(mode.name())), mode);
        }
    }

    #[test]
    fn a_missing_or_unknown_mode_is_songs() {
        assert_eq!(ListMode::parse(None), ListMode::Songs);
        assert_eq!(ListMode::parse(Some("genres")), ListMode::Songs);
    }

    #[test]
    fn each_mode_has_a_label() {
        let labels: Vec<&str> = ListMode::ALL.into_iter().map(ListMode::label).collect();

        assert_eq!(labels, vec!["Songs", "Albums", "Artists"]);
    }
}
