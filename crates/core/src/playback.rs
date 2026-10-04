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
/// already types, picks, or presses.
pub fn space_toggles_playback(focused_tag: &str) -> bool {
    !matches!(
        focused_tag.to_ascii_uppercase().as_str(),
        "INPUT" | "TEXTAREA" | "SELECT" | "BUTTON"
    )
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
    fn space_toggles_playback_from_the_page_and_links() {
        assert!(space_toggles_playback("BODY"));
        assert!(space_toggles_playback("a"));
    }

    #[test]
    fn space_is_left_to_form_controls() {
        for tag in ["INPUT", "textarea", "SELECT", "BUTTON"] {
            assert!(!space_toggles_playback(tag), "{tag}");
        }
    }

    #[test]
    fn volume_icon_follows_the_level() {
        assert_eq!(volume_icon(0.8, false), "volume-up-fill");
        assert_eq!(volume_icon(0.3, false), "volume-down-fill");
        assert_eq!(volume_icon(0.0, false), "volume-mute-fill");
        assert_eq!(volume_icon(0.8, true), "volume-mute-fill");
    }
}
