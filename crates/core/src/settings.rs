use crate::equalizer;
use crate::playback;
use crate::queue::Mode;
use crate::theme;
use crate::views::ListMode;

pub const EQUALIZER: &str = "equalizer";
pub const EQUALIZER_PRESETS: &str = "equalizer_presets";
pub const EQUALIZER_OPEN: &str = "equalizer_open";
pub const VISUALIZER_OPEN: &str = "visualizer_open";
pub const PLAYLIST_OPEN: &str = "playlist_open";
pub const THEME: &str = "theme";
pub const VOLUME: &str = "volume";
pub const LOOP_MODE: &str = "loop_mode";
pub const RESUME: &str = "resume";
pub const SEARCH: &str = "search";
pub const LIST_MODE: &str = "list_mode";

pub const ALL: [&str; 11] = [
    EQUALIZER,
    EQUALIZER_PRESETS,
    EQUALIZER_OPEN,
    VISUALIZER_OPEN,
    PLAYLIST_OPEN,
    THEME,
    VOLUME,
    LOOP_MODE,
    RESUME,
    SEARCH,
    LIST_MODE,
];

fn flag(value: &str, default: bool) -> String {
    match value {
        "true" => true,
        "false" => false,
        _ => default,
    }
    .to_string()
}

fn resume(value: &str) -> String {
    playback::parse_resume(Some(value)).map_or_else(String::new, |saved| {
        playback::serialize_resume(Some(saved.entry_id), saved.position)
    })
}

/// The value as it is kept for a known setting, read and written back in
/// its canonical form. A malformed or empty value gives the default. `None`
/// for a name that is not a setting.
pub fn normalize(name: &str, value: &str) -> Option<String> {
    let normalized = match name {
        EQUALIZER => equalizer::serialize(&equalizer::parse(Some(value))),
        EQUALIZER_PRESETS => equalizer::serialize_saved(&equalizer::parse_saved(Some(value))),
        EQUALIZER_OPEN => flag(value, false),
        VISUALIZER_OPEN | PLAYLIST_OPEN => flag(value, true),
        THEME => theme::find(Some(value)).name.to_string(),
        VOLUME => playback::parse_volume(Some(value)).to_string(),
        LOOP_MODE => Mode::parse(Some(value)).as_str().to_string(),
        RESUME => resume(value),
        SEARCH => value.to_string(),
        LIST_MODE => ListMode::parse(Some(value)).name().to_string(),
        _ => return None,
    };

    Some(normalized)
}

/// The value a setting has before anything is saved.
pub fn default(name: &str) -> Option<String> {
    normalize(name, "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equalizer_settings_are_kept_in_canonical_form() {
        assert_eq!(
            normalize(EQUALIZER, "on;20;1,2,3,4,5,6,7,8,9,10"),
            Some("on;12;1,2,3,4,5,6,7,8,9,10".to_string())
        );
    }

    #[test]
    fn malformed_equalizer_settings_become_the_defaults() {
        assert_eq!(
            normalize(EQUALIZER, "garbage"),
            Some("off;0;0,0,0,0,0,0,0,0,0,0".to_string())
        );
    }

    #[test]
    fn saved_presets_drop_lines_that_do_not_parse() {
        assert_eq!(
            normalize(
                EQUALIZER_PRESETS,
                "-3;5,4,2,-1,-2,-1,1,3,4,5;Late Night\nbroken"
            ),
            Some("-3;5,4,2,-1,-2,-1,1,3,4,5;Late Night".to_string())
        );
    }

    #[test]
    fn the_equalizer_starts_closed_and_the_other_panels_open() {
        assert_eq!(
            [EQUALIZER_OPEN, VISUALIZER_OPEN, PLAYLIST_OPEN].map(default),
            [
                Some("false".to_string()),
                Some("true".to_string()),
                Some("true".to_string())
            ]
        );
    }

    #[test]
    fn panel_flags_keep_true_and_false() {
        assert_eq!(
            [
                normalize(EQUALIZER_OPEN, "true"),
                normalize(VISUALIZER_OPEN, "false")
            ],
            [Some("true".to_string()), Some("false".to_string())]
        );
    }

    #[test]
    fn an_unknown_theme_becomes_the_default_theme() {
        assert_eq!(
            [normalize(THEME, "charcoal"), normalize(THEME, "nope")],
            [
                Some("charcoal".to_string()),
                Some(theme::THEMES[0].name.to_string())
            ]
        );
    }

    #[test]
    fn volume_is_clamped_and_defaults_to_full() {
        assert_eq!(
            [
                normalize(VOLUME, "0.25"),
                normalize(VOLUME, "7"),
                default(VOLUME)
            ],
            [
                Some("0.25".to_string()),
                Some("1".to_string()),
                Some("1".to_string())
            ]
        );
    }

    #[test]
    fn loop_mode_keeps_known_modes_and_defaults_to_play_through() {
        assert_eq!(
            [normalize(LOOP_MODE, "loop-all"), default(LOOP_MODE)],
            [Some("loop-all".to_string()), Some("play".to_string())]
        );
    }

    #[test]
    fn resume_keeps_an_entry_and_position_and_defaults_to_nothing() {
        assert_eq!(
            [normalize(RESUME, "12;30.25"), normalize(RESUME, "junk")],
            [Some("12;30.2".to_string()), Some(String::new())]
        );
    }

    #[test]
    fn search_is_kept_as_typed() {
        assert_eq!(
            normalize(SEARCH, "album:\"Live\""),
            Some("album:\"Live\"".to_string())
        );
    }

    #[test]
    fn list_mode_keeps_known_views_and_defaults_to_songs() {
        assert_eq!(
            [normalize(LIST_MODE, "albums"), default(LIST_MODE)],
            [Some("albums".to_string()), Some("songs".to_string())]
        );
    }

    #[test]
    fn every_setting_has_a_default() {
        assert!(ALL.iter().all(|name| default(name).is_some()));
    }

    #[test]
    fn unknown_names_are_not_settings() {
        assert_eq!(normalize("player.mode", "play"), None);
    }
}
