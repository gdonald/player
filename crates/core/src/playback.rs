const SECONDS_PER_MINUTE: u64 = 60;
const SECONDS_PER_HOUR: u64 = 3600;

pub const DEFAULT_VOLUME: f64 = 1.0;

/// `m:ss` below an hour, `h:mm:ss` from an hour up. Unknown times show `0:00`.
pub fn format_time(seconds: f64) -> String {
    if !seconds.is_finite() || seconds <= 0.0 {
        return "0:00".to_string();
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "finite and positive, checked above"
    )]
    let total = seconds.floor() as u64;

    let hours = total / SECONDS_PER_HOUR;
    let minutes = (total % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE;
    let remainder = total % SECONDS_PER_MINUTE;

    if hours > 0 {
        format!("{hours}:{minutes:02}:{remainder:02}")
    } else {
        format!("{minutes}:{remainder:02}")
    }
}

/// A playlist row: `1. Artist - Title`.
pub fn track_line(number: usize, artist: &str, title: &str) -> String {
    format!("{number}. {artist} - {title}")
}

/// The scrolling title display: `1. Artist - Title (3:50)`.
pub fn now_playing_line(number: usize, artist: &str, title: &str, length: Option<i32>) -> String {
    let line = track_line(number, artist, title);

    match length {
        Some(seconds) if seconds > 0 => format!("{line} ({})", format_time(f64::from(seconds))),
        _ => line,
    }
}

/// The queue's running time in seconds. Tracks without a length count as 0.
pub fn total_seconds(lengths: &[Option<i32>]) -> f64 {
    lengths
        .iter()
        .flatten()
        .map(|length| f64::from(*length))
        .sum()
}

/// How far through the track the position is, from 0 to 100.
pub fn percent(position: f64, duration: f64) -> f64 {
    if !position.is_finite() || !duration.is_finite() || duration <= 0.0 {
        return 0.0;
    }

    (position / duration * 100.0).clamp(0.0, 100.0)
}

/// Seconds left in the track, never below 0.
pub fn remaining(position: f64, duration: f64) -> f64 {
    (duration - position).max(0.0)
}

/// A stored volume between 0 and 1, or full volume when missing or unreadable.
pub fn parse_volume(stored: Option<&str>) -> f64 {
    stored
        .and_then(|text| text.trim().parse::<f64>().ok())
        .filter(|volume| volume.is_finite())
        .map_or(DEFAULT_VOLUME, |volume| volume.clamp(0.0, 1.0))
}

/// The bootstrap-icons name for the volume button.
pub fn volume_icon(volume: f64, muted: bool) -> &'static str {
    if muted || volume <= 0.0 {
        "volume-mute-fill"
    } else if volume < 0.5 {
        "volume-down-fill"
    } else {
        "volume-up-fill"
    }
}

/// The space bar toggles playback unless focus is on an element where space
/// types, picks, or checks. A focused button or slider still toggles, so the
/// space bar never presses the button that was last clicked.
pub fn space_toggles_playback(focused_tag: &str, input_type: &str) -> bool {
    match focused_tag.to_ascii_uppercase().as_str() {
        "INPUT" => matches!(
            input_type.to_ascii_lowercase().as_str(),
            "range" | "button" | "submit" | "reset"
        ),
        "TEXTAREA" | "SELECT" => false,
        _ => true,
    }
}

/// The queue entry that was current and how far into it playback was, kept
/// so a reload picks up there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Resume {
    pub entry_id: i64,
    pub position: f64,
}

/// Stored as `entry;seconds`, or empty when nothing is current.
pub fn serialize_resume(entry_id: Option<i64>, position: f64) -> String {
    entry_id.map_or_else(String::new, |entry_id| format!("{entry_id};{position:.1}"))
}

pub fn parse_resume(stored: Option<&str>) -> Option<Resume> {
    let (entry_id, position) = stored?.split_once(';')?;
    let position = position
        .parse::<f64>()
        .ok()
        .filter(|position| position.is_finite())?;

    Some(Resume {
        entry_id: entry_id.parse().ok()?,
        position: position.max(0.0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_times_show_minutes_and_padded_seconds() {
        assert_eq!(format_time(0.4), "0:00");
        assert_eq!(format_time(65.9), "1:05");
        assert_eq!(format_time(599.0), "9:59");
    }

    #[test]
    fn times_from_an_hour_show_hours() {
        assert_eq!(format_time(3600.0), "1:00:00");
        assert_eq!(format_time(3725.0), "1:02:05");
    }

    #[test]
    fn unknown_or_negative_times_show_zero() {
        assert_eq!(format_time(f64::NAN), "0:00");
        assert_eq!(format_time(f64::INFINITY), "0:00");
        assert_eq!(format_time(-3.0), "0:00");
    }

    #[test]
    fn track_line_numbers_the_artist_and_title() {
        assert_eq!(track_line(4, "Crusher-P", "Echo"), "4. Crusher-P - Echo");
    }

    #[test]
    fn now_playing_line_adds_a_known_length() {
        assert_eq!(
            now_playing_line(4, "Crusher-P", "Echo", Some(230)),
            "4. Crusher-P - Echo (3:50)"
        );
        assert_eq!(now_playing_line(1, "A", "B", None), "1. A - B");
        assert_eq!(now_playing_line(1, "A", "B", Some(0)), "1. A - B");
    }

    #[test]
    fn total_seconds_adds_known_lengths() {
        assert!((total_seconds(&[Some(60), None, Some(125)]) - 185.0).abs() < f64::EPSILON);
        assert!(total_seconds(&[]).abs() < f64::EPSILON);
    }

    #[test]
    fn percent_is_the_share_of_the_duration() {
        assert!((percent(30.0, 120.0) - 25.0).abs() < f64::EPSILON);
    }

    #[test]
    fn percent_is_clamped_to_the_track() {
        assert!((percent(200.0, 100.0) - 100.0).abs() < f64::EPSILON);
        assert!(percent(-5.0, 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn percent_is_zero_without_a_known_duration() {
        assert!(percent(5.0, 0.0).abs() < f64::EPSILON);
        assert!(percent(5.0, f64::NAN).abs() < f64::EPSILON);
        assert!(percent(f64::NAN, 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn remaining_is_the_time_left_in_the_track() {
        assert!((remaining(30.0, 120.0) - 90.0).abs() < f64::EPSILON);
    }

    #[test]
    fn remaining_is_zero_past_the_end_or_without_a_known_duration() {
        assert!(remaining(200.0, 100.0).abs() < f64::EPSILON);
        assert!(remaining(5.0, f64::NAN).abs() < f64::EPSILON);
    }

    #[test]
    fn stored_volume_is_read_and_clamped() {
        assert!((parse_volume(Some(" 0.25 ")) - 0.25).abs() < f64::EPSILON);
        assert!((parse_volume(Some("3")) - 1.0).abs() < f64::EPSILON);
        assert!(parse_volume(Some("-1")).abs() < f64::EPSILON);
    }

    #[test]
    fn missing_or_unreadable_volume_is_full() {
        assert!((parse_volume(None) - DEFAULT_VOLUME).abs() < f64::EPSILON);
        assert!((parse_volume(Some("loud")) - DEFAULT_VOLUME).abs() < f64::EPSILON);
        assert!((parse_volume(Some("NaN")) - DEFAULT_VOLUME).abs() < f64::EPSILON);
    }

    #[test]
    fn space_toggles_playback_from_the_page_links_buttons_and_sliders() {
        assert!(space_toggles_playback("BODY", ""));
        assert!(space_toggles_playback("a", ""));
        assert!(space_toggles_playback("BUTTON", "button"));

        for input_type in ["range", "Button", "submit", "reset"] {
            assert!(space_toggles_playback("input", input_type), "{input_type}");
        }
    }

    #[test]
    fn space_is_left_to_text_fields_menus_and_checkboxes() {
        for (tag, input_type) in [
            ("INPUT", "text"),
            ("INPUT", ""),
            ("INPUT", "checkbox"),
            ("textarea", ""),
            ("SELECT", ""),
        ] {
            assert!(
                !space_toggles_playback(tag, input_type),
                "{tag} {input_type}"
            );
        }
    }

    #[test]
    fn volume_icon_follows_the_level() {
        assert_eq!(volume_icon(0.8, false), "volume-up-fill");
        assert_eq!(volume_icon(0.3, false), "volume-down-fill");
        assert_eq!(volume_icon(0.0, false), "volume-mute-fill");
        assert_eq!(volume_icon(0.8, true), "volume-mute-fill");
    }

    #[test]
    fn the_resume_point_reads_back_from_storage() {
        let stored = serialize_resume(Some(42), 61.26);

        assert_eq!(stored, "42;61.3");
        assert_eq!(
            parse_resume(Some(&stored)),
            Some(Resume {
                entry_id: 42,
                position: 61.3
            })
        );
    }

    #[test]
    fn nothing_current_stores_an_empty_resume_point() {
        assert_eq!(serialize_resume(None, 12.0), "");
        assert_eq!(parse_resume(Some("")), None);
        assert_eq!(parse_resume(None), None);
    }

    #[test]
    fn a_damaged_resume_point_is_ignored() {
        assert_eq!(parse_resume(Some("x;1")), None);
        assert_eq!(parse_resume(Some("4;x")), None);
        assert_eq!(parse_resume(Some("4;inf")), None);
    }

    #[test]
    fn a_negative_resume_position_starts_at_the_beginning() {
        assert_eq!(
            parse_resume(Some("4;-3")).map(|resume| resume.position),
            Some(0.0)
        );
    }
}
