pub const BAND_COUNT: usize = 10;

/// Center frequencies in hertz, an octave apart.
pub const BANDS: [f32; BAND_COUNT] = [
    32.0, 64.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];

pub const MAX_GAIN: f32 = 12.0;

/// Bandwidth of each peaking filter. 1.4 spans about an octave, so
/// neighboring bands meet without large gaps or overlaps.
pub const PEAKING_Q: f32 = 1.4;

pub type Gains = [f32; BAND_COUNT];

pub const FLAT: Gains = [0.0; BAND_COUNT];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Preset {
    pub name: &'static str,
    pub gains: Gains,
}

pub const PRESETS: [Preset; 9] = [
    Preset {
        name: "Flat",
        gains: FLAT,
    },
    Preset {
        name: "Bass Boost",
        gains: [6.0, 5.0, 4.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    },
    Preset {
        name: "Treble Boost",
        gains: [0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 2.0, 4.0, 5.0, 6.0],
    },
    Preset {
        name: "Vocal",
        gains: [-2.0, -2.0, -1.0, 0.0, 2.0, 4.0, 4.0, 3.0, 1.0, 0.0],
    },
    Preset {
        name: "Rock",
        gains: [5.0, 4.0, 2.0, -1.0, -2.0, -1.0, 1.0, 3.0, 4.0, 5.0],
    },
    Preset {
        name: "Pop",
        gains: [-1.0, 1.0, 3.0, 4.0, 3.0, 1.0, -1.0, -1.0, 0.0, 0.0],
    },
    Preset {
        name: "Jazz",
        gains: [3.0, 2.0, 1.0, 2.0, -1.0, -1.0, 0.0, 1.0, 2.0, 3.0],
    },
    Preset {
        name: "Classical",
        gains: [4.0, 3.0, 2.0, 1.0, -1.0, -1.0, 0.0, 2.0, 3.0, 4.0],
    },
    Preset {
        name: "Electronic",
        gains: [5.0, 4.0, 1.0, 0.0, -2.0, 1.0, 0.0, 1.0, 4.0, 5.0],
    },
];

pub const CUSTOM: &str = "Custom";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterKind {
    LowShelf,
    Peaking,
    HighShelf,
}

/// The lowest band shelves everything below it, the highest everything above
/// it, and the bands between boost or cut around their center.
pub fn filter_kind(index: usize) -> FilterKind {
    match index {
        0 => FilterKind::LowShelf,
        index if index == BAND_COUNT - 1 => FilterKind::HighShelf,
        _ => FilterKind::Peaking,
    }
}

pub fn clamp_gain(gain: f32) -> f32 {
    if gain.is_finite() {
        gain.clamp(-MAX_GAIN, MAX_GAIN)
    } else {
        0.0
    }
}

pub fn preset(name: &str) -> Option<Preset> {
    PRESETS.into_iter().find(|preset| preset.name == name)
}

/// The preset whose gains these are, or `Custom`.
#[allow(
    clippy::float_cmp,
    reason = "gains move in 0.5 dB steps, which f32 holds exactly"
)]
pub fn preset_name(gains: &Gains) -> &'static str {
    PRESETS
        .iter()
        .find(|preset| preset.gains == *gains)
        .map_or(CUSTOM, |preset| preset.name)
}

/// Where a gain sits on a slider running from -12 dB at 0% to +12 dB at 100%.
pub fn gain_percent(gain: f32) -> f32 {
    (clamp_gain(gain) + MAX_GAIN) / (2.0 * MAX_GAIN) * 100.0
}

/// `32`, `500`, `1k`, `16k`.
pub fn band_label(frequency: f32) -> String {
    if frequency >= 1000.0 {
        format!("{}k", frequency / 1000.0)
    } else {
        format!("{frequency}")
    }
}

/// `+3`, `0`, `-2.5`, in decibels.
pub fn gain_label(gain: f32) -> String {
    if gain > 0.0 {
        format!("+{gain}")
    } else if gain < 0.0 {
        format!("{gain}")
    } else {
        "0".to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    pub enabled: bool,
    pub preamp: f32,
    pub gains: Gains,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            enabled: false,
            preamp: 0.0,
            gains: FLAT,
        }
    }
}

impl Settings {
    /// The gains the filters run at: flat while the equalizer is off.
    pub fn applied(&self) -> Gains {
        if self.enabled { self.gains } else { FLAT }
    }

    /// The preamp the gain stage runs at, in decibels: 0 while the equalizer is off.
    pub fn applied_preamp(&self) -> f32 {
        if self.enabled { self.preamp } else { 0.0 }
    }
}

/// The amplitude multiplier for a level in decibels.
pub fn decibels_to_amplitude(decibels: f32) -> f32 {
    10_f32.powf(decibels / 20.0)
}

/// Slider color from green at -12 dB through yellow to red at +12 dB, as a hue.
pub fn slider_hue(gain: f32) -> f32 {
    120.0 - gain_percent(gain) * 1.2
}

/// SVG polyline points for the gain curve, one point per band, spread across
/// the width, with +12 dB at the top.
pub fn curve_points(gains: &Gains, width: f32, height: f32) -> String {
    #[allow(
        clippy::cast_precision_loss,
        reason = "the band count is ten, far inside f32's exact integer range"
    )]
    let step = width / (BAND_COUNT - 1) as f32;

    gains
        .iter()
        .enumerate()
        .map(|(index, gain)| {
            #[allow(clippy::cast_precision_loss, reason = "index is below ten")]
            let x = index as f32 * step;
            let y = height * (1.0 - gain_percent(*gain) / 100.0);
            format!("{x},{y}")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// `on;-3;0,1.5,-2,...` or `off;...`: state, preamp, then the bands, as kept
/// in `localStorage`.
pub fn serialize(settings: &Settings) -> String {
    let gains: Vec<String> = settings.gains.iter().map(ToString::to_string).collect();
    let state = if settings.enabled { "on" } else { "off" };

    format!("{state};{};{}", settings.preamp, gains.join(","))
}

fn parse_gains(text: &str) -> Option<Gains> {
    let values: Vec<f32> = text
        .split(',')
        .map_while(|value| value.trim().parse::<f32>().ok())
        .collect();

    Gains::try_from(values.as_slice()).ok()
}

/// Reads stored settings. The preamp-less `on;bands` form from before the
/// preamp reads with the preamp at 0. Anything missing or malformed gives the
/// defaults.
pub fn parse(stored: Option<&str>) -> Settings {
    let parts: Vec<&str> = stored
        .map(|text| text.split(';').collect())
        .unwrap_or_default();

    let (state, preamp, gains) = match parts.as_slice() {
        [state, gains] => (*state, Some(0.0), *gains),
        [state, preamp, gains] => (*state, preamp.trim().parse::<f32>().ok(), *gains),
        _ => return Settings::default(),
    };

    let enabled = match state {
        "on" => true,
        "off" => false,
        _ => return Settings::default(),
    };

    let (Some(preamp), Some(gains)) = (preamp, parse_gains(gains)) else {
        return Settings::default();
    };

    Settings {
        enabled,
        preamp: clamp_gain(preamp),
        gains: gains.map(clamp_gain),
    }
}

/// A preset the user saved: a name, the preamp, and the band gains.
#[derive(Debug, Clone, PartialEq)]
pub struct SavedPreset {
    pub name: String,
    pub preamp: f32,
    pub gains: Gains,
}

/// Why a name cannot be saved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameProblem {
    Blank,
    BuiltIn,
}

impl NameProblem {
    pub fn message(self) -> &'static str {
        match self {
            NameProblem::Blank => "Enter a name",
            NameProblem::BuiltIn => "A built-in preset has that name",
        }
    }
}

/// Saved presets as kept in `localStorage`: one `preamp;bands;name` line
/// each. The name comes last so it can hold semicolons.
pub fn serialize_saved(presets: &[SavedPreset]) -> String {
    presets
        .iter()
        .map(|preset| {
            let gains: Vec<String> = preset.gains.iter().map(ToString::to_string).collect();
            format!("{};{};{}", preset.preamp, gains.join(","), preset.name)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Reads stored saved presets, skipping lines that do not parse.
pub fn parse_saved(stored: Option<&str>) -> Vec<SavedPreset> {
    stored
        .unwrap_or_default()
        .lines()
        .filter_map(|line| {
            let mut parts = line.splitn(3, ';');
            let preamp = parts.next()?.trim().parse::<f32>().ok()?;
            let gains = parse_gains(parts.next()?)?;
            let name = parts.next()?.trim().to_string();

            (!name.is_empty()).then(|| SavedPreset {
                name,
                preamp: clamp_gain(preamp),
                gains: gains.map(clamp_gain),
            })
        })
        .collect()
}

/// The saved presets with this one added, or replacing a saved preset of the
/// same name. A blank name or a built-in preset's name cannot be saved.
pub fn save_preset(
    presets: &[SavedPreset],
    name: &str,
    preamp: f32,
    gains: Gains,
) -> Result<Vec<SavedPreset>, NameProblem> {
    let name = name.trim();

    if name.is_empty() {
        return Err(NameProblem::Blank);
    }
    if name == CUSTOM || preset(name).is_some() {
        return Err(NameProblem::BuiltIn);
    }

    let mut saved: Vec<SavedPreset> = presets
        .iter()
        .filter(|preset| preset.name != name)
        .cloned()
        .collect();
    saved.push(SavedPreset {
        name: name.to_string(),
        preamp,
        gains,
    });

    Ok(saved)
}

/// The gains a menu choice sets, and the preamp when the choice is a saved
/// preset. Built-in presets leave the preamp alone.
pub fn choose(name: &str, saved: &[SavedPreset]) -> Option<(Gains, Option<f32>)> {
    preset(name).map(|preset| (preset.gains, None)).or_else(|| {
        saved
            .iter()
            .find(|preset| preset.name == name)
            .map(|preset| (preset.gains, Some(preset.preamp)))
    })
}

/// The menu choice that matches these settings: a saved preset with the same
/// bands and preamp, a built-in preset with the same bands, or `Custom`.
#[allow(
    clippy::float_cmp,
    reason = "gains move in 0.5 dB steps, which f32 holds exactly"
)]
pub fn selected(settings: &Settings, saved: &[SavedPreset]) -> String {
    saved
        .iter()
        .find(|preset| preset.gains == settings.gains && preset.preamp == settings.preamp)
        .map_or_else(
            || preset_name(&settings.gains).to_string(),
            |preset| preset.name.clone(),
        )
}

#[cfg(test)]
#[allow(
    clippy::float_cmp,
    reason = "gains move in 0.5 dB steps, which f32 holds exactly"
)]
mod tests {
    use super::*;

    #[test]
    fn bands_are_an_octave_apart() {
        for pair in BANDS.windows(2) {
            let ratio = pair[1] / pair[0];
            assert!((1.95..=2.05).contains(&ratio), "{pair:?}");
        }
    }

    #[test]
    fn every_preset_stays_within_the_gain_range() {
        for preset in PRESETS {
            assert!(
                preset.gains.iter().all(|gain| gain.abs() <= MAX_GAIN),
                "{}",
                preset.name
            );
        }
    }

    #[test]
    fn preset_names_are_unique() {
        for (index, preset) in PRESETS.iter().enumerate() {
            assert!(
                PRESETS[index + 1..]
                    .iter()
                    .all(|other| other.name != preset.name)
            );
        }
    }

    #[test]
    fn outer_bands_shelve_and_inner_bands_peak() {
        assert_eq!(filter_kind(0), FilterKind::LowShelf);
        assert_eq!(filter_kind(1), FilterKind::Peaking);
        assert_eq!(filter_kind(8), FilterKind::Peaking);
        assert_eq!(filter_kind(9), FilterKind::HighShelf);
    }

    #[test]
    fn gains_are_clamped_to_the_range() {
        assert!((clamp_gain(20.0) - 12.0).abs() < f32::EPSILON);
        assert!((clamp_gain(-20.0) + 12.0).abs() < f32::EPSILON);
        assert!((clamp_gain(3.5) - 3.5).abs() < f32::EPSILON);
        assert!(clamp_gain(f32::NAN).abs() < f32::EPSILON);
    }

    #[test]
    fn presets_are_found_by_name() {
        assert_eq!(preset("Rock").map(|preset| preset.gains[0]), Some(5.0));
        assert_eq!(preset("Polka"), None);
    }

    #[test]
    fn gains_matching_a_preset_are_named_for_it() {
        assert_eq!(preset_name(&FLAT), "Flat");
        assert_eq!(preset_name(&PRESETS[1].gains), "Bass Boost");
    }

    #[test]
    fn other_gains_are_custom() {
        let mut gains = FLAT;
        gains[4] = 1.5;

        assert_eq!(preset_name(&gains), "Custom");
    }

    #[test]
    fn gain_percent_puts_zero_in_the_middle() {
        assert_eq!(gain_percent(-12.0), 0.0);
        assert_eq!(gain_percent(0.0), 50.0);
        assert_eq!(gain_percent(6.0), 75.0);
        assert_eq!(gain_percent(30.0), 100.0);
    }

    #[test]
    fn band_labels_use_k_for_thousands() {
        assert_eq!(band_label(32.0), "32");
        assert_eq!(band_label(500.0), "500");
        assert_eq!(band_label(1000.0), "1k");
        assert_eq!(band_label(16000.0), "16k");
    }

    #[test]
    fn gain_labels_sign_boosts() {
        assert_eq!(gain_label(3.0), "+3");
        assert_eq!(gain_label(0.0), "0");
        assert_eq!(gain_label(-2.5), "-2.5");
    }

    #[test]
    fn default_settings_are_off_and_flat() {
        assert_eq!(
            Settings::default(),
            Settings {
                enabled: false,
                preamp: 0.0,
                gains: FLAT
            }
        );
    }

    #[test]
    fn applied_gains_are_flat_while_off() {
        let mut settings = Settings {
            enabled: false,
            preamp: -4.0,
            gains: PRESETS[1].gains,
        };

        assert_eq!(settings.applied(), FLAT);
        assert_eq!(settings.applied_preamp(), 0.0);

        settings.enabled = true;

        assert_eq!(settings.applied(), PRESETS[1].gains);
        assert_eq!(settings.applied_preamp(), -4.0);
    }

    #[test]
    fn settings_round_trip_through_storage_text() {
        let settings = Settings {
            enabled: true,
            preamp: -3.5,
            gains: [1.5, -2.0, 0.0, 3.0, 0.0, 0.0, 0.0, 0.0, -12.0, 12.0],
        };

        assert_eq!(serialize(&settings), "on;-3.5;1.5,-2,0,3,0,0,0,0,-12,12");
        assert_eq!(parse(Some(&serialize(&settings))), settings);
    }

    #[test]
    fn off_settings_are_stored_as_off() {
        assert_eq!(serialize(&Settings::default()), "off;0;0,0,0,0,0,0,0,0,0,0");
    }

    #[test]
    fn stored_off_settings_keep_their_gains() {
        let parsed = parse(Some("off;2;1,2,3,4,5,6,7,8,9,10"));

        assert!(!parsed.enabled);
        assert!((parsed.gains[9] - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn stored_gains_and_preamp_are_clamped() {
        let parsed = parse(Some("on;-40;99,0,0,0,0,0,0,0,0,0"));

        assert_eq!((parsed.preamp, parsed.gains[0]), (-12.0, 12.0));
    }

    #[test]
    fn settings_stored_before_the_preamp_read_with_it_at_zero() {
        let parsed = parse(Some("on;1,2,3,4,5,6,7,8,9,10"));

        assert!(parsed.enabled);
        assert_eq!((parsed.preamp, parsed.gains[9]), (0.0, 10.0));
    }

    #[test]
    fn decibels_convert_to_amplitude() {
        assert!((decibels_to_amplitude(0.0) - 1.0).abs() < 1e-6);
        assert!((decibels_to_amplitude(-6.0) - 0.501_187).abs() < 1e-5);
        assert!((decibels_to_amplitude(20.0) - 10.0).abs() < 1e-4);
    }

    #[test]
    fn slider_hue_runs_from_green_to_red() {
        assert!((slider_hue(-12.0) - 120.0).abs() < 1e-4);
        assert!((slider_hue(0.0) - 60.0).abs() < 1e-4);
        assert!(slider_hue(12.0).abs() < 1e-4);
    }

    #[test]
    fn curve_points_span_the_box_with_boosts_at_the_top() {
        let mut gains = FLAT;
        gains[0] = 12.0;
        gains[9] = -12.0;

        let points = curve_points(&gains, 90.0, 20.0);

        assert_eq!(
            points,
            "0,0 10,10 20,10 30,10 40,10 50,10 60,10 70,10 80,10 90,20"
        );
    }

    #[test]
    fn missing_or_malformed_settings_are_the_defaults() {
        for stored in [
            None,
            Some(""),
            Some("on"),
            Some("maybe;0,0,0,0,0,0,0,0,0,0"),
            Some("on;0,0,0"),
            Some("on;0,0,0,0,0,0,0,0,0,0,0"),
            Some("on;0,0,0,0,x,0,0,0,0,0"),
            Some("on;loud;0,0,0,0,0,0,0,0,0,0"),
            Some("on;0;0;0"),
        ] {
            assert_eq!(parse(stored), Settings::default(), "{stored:?}");
        }
    }

    fn saved(name: &str, preamp: f32, first_gain: f32) -> SavedPreset {
        let mut gains = FLAT;
        gains[0] = first_gain;

        SavedPreset {
            name: name.to_string(),
            preamp,
            gains,
        }
    }

    #[test]
    fn saved_presets_read_back_from_storage() {
        let presets = vec![saved("Late Night", -3.0, 4.5), saved("A;B", 0.0, -2.0)];

        assert_eq!(parse_saved(Some(&serialize_saved(&presets))), presets);
    }

    #[test]
    fn stored_lines_that_do_not_parse_are_skipped() {
        let stored = "nope\n0;1,2;Short\n0;0,0,0,0,0,0,0,0,0,0;   \n2;0,0,0,0,0,0,0,0,0,0;Kept\n0;0,0,0,0,0,0,0,0,0,0\nx;0,0,0,0,0,0,0,0,0,0;Bad";

        let names: Vec<String> = parse_saved(Some(stored))
            .into_iter()
            .map(|preset| preset.name)
            .collect();

        assert_eq!(names, vec!["Kept"]);
        assert_eq!(parse_saved(None), Vec::new());
    }

    #[test]
    fn stored_gains_are_clamped() {
        let preset = &parse_saved(Some("40;99,0,0,0,0,0,0,0,0,0;Loud"))[0];

        assert_eq!((preset.preamp, preset.gains[0]), (MAX_GAIN, MAX_GAIN));
    }

    #[test]
    fn saving_adds_a_preset_with_a_trimmed_name() {
        let presets = save_preset(&[], "  Late Night ", -3.0, FLAT).unwrap();

        assert_eq!(presets, vec![saved("Late Night", -3.0, 0.0)]);
    }

    #[test]
    fn saving_under_a_saved_name_replaces_that_preset() {
        let presets = save_preset(
            &[saved("Mine", 0.0, 1.0), saved("Other", 0.0, 2.0)],
            "Mine",
            1.0,
            FLAT,
        )
        .unwrap();

        assert_eq!(
            presets,
            vec![saved("Other", 0.0, 2.0), saved("Mine", 1.0, 0.0)]
        );
    }

    #[test]
    fn blank_and_built_in_names_cannot_be_saved() {
        assert_eq!(save_preset(&[], "  ", 0.0, FLAT), Err(NameProblem::Blank));
        assert_eq!(
            save_preset(&[], "Rock", 0.0, FLAT),
            Err(NameProblem::BuiltIn)
        );
        assert_eq!(
            save_preset(&[], CUSTOM, 0.0, FLAT),
            Err(NameProblem::BuiltIn)
        );
        assert_eq!(NameProblem::Blank.message(), "Enter a name");
        assert_eq!(
            NameProblem::BuiltIn.message(),
            "A built-in preset has that name"
        );
    }

    #[test]
    fn choosing_a_built_in_preset_keeps_the_preamp_and_a_saved_one_sets_it() {
        let presets = vec![saved("Mine", -4.0, 3.0)];

        assert_eq!(
            choose("Rock", &presets),
            preset("Rock").map(|rock| (rock.gains, None))
        );
        assert_eq!(
            choose("Mine", &presets),
            Some((presets[0].gains, Some(-4.0)))
        );
        assert_eq!(choose("Nothing", &presets), None);
    }

    #[test]
    fn the_selection_names_the_matching_preset() {
        let presets = vec![saved("Mine", -4.0, 3.0)];
        let mine = Settings {
            enabled: true,
            preamp: -4.0,
            gains: presets[0].gains,
        };

        assert_eq!(selected(&Settings::default(), &presets), "Flat");
        assert_eq!(
            selected(&Settings::default(), &[saved("Quiet Flat", 0.0, 0.0)]),
            "Quiet Flat"
        );
        assert_eq!(selected(&mine, &presets), "Mine");
        assert_eq!(
            selected(
                &Settings {
                    preamp: 0.0,
                    ..mine
                },
                &presets
            ),
            CUSTOM
        );
    }
}
