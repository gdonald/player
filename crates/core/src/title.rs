pub const DEFAULT_TITLE: &str = "Player";

pub fn page_title(playing: Option<(&str, &str)>) -> String {
    match playing {
        Some((artist, title)) => format!("{artist}: {title}"),
        None => DEFAULT_TITLE.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_names_the_playing_artist_and_track() {
        assert_eq!(page_title(Some(("Band", "Song"))), "Band: Song");
    }

    #[test]
    fn title_is_player_when_nothing_plays() {
        assert_eq!(page_title(None), "Player");
    }
}
