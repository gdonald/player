/// Bars in the analyzer, as Winamp drew about twenty.
pub const BAR_COUNT: usize = 20;

pub const LOWEST_HERTZ: f32 = 40.0;
pub const HIGHEST_HERTZ: f32 = 16000.0;

/// The analyser's byte range spans these decibels. A -15 dB ceiling, above
/// the Web Audio default of -30, leaves loud passages room before the bars
/// reach the top.
pub const MIN_DECIBELS: f64 = -85.0;
pub const MAX_DECIBELS: f64 = -15.0;

/// How much the lowest bar is lowered, as a share of the full height. Music
/// carries far more energy in the bass than the treble, so without this the
/// left bars sit at the top. The cut shrinks evenly to nothing at the top bar.
pub const BASS_TILT: f32 = 0.18;

/// How far a peak cap falls per frame, as a share of the full height.
pub const PEAK_FALL: f32 = 0.02;

/// One level per bar, 0 to 1, from the analyser's byte frequency bins. Bars
/// are spaced evenly in pitch from 40 Hz to 16 kHz, and each takes the
/// loudest bin in its range, the end bin belonging to the next bar.
/// Scanline spacing in CSS pixels when a theme sets none.
pub const DEFAULT_SCANLINE_SPACING: f64 = 3.0;

/// The scanline spacing a theme gives in `--vis-scanline-spacing`, or the
/// default when it is missing, unreadable, or under one pixel.
pub fn scanline_spacing(value: &str) -> f64 {
    value
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|spacing| *spacing >= 1.0)
        .unwrap_or(DEFAULT_SCANLINE_SPACING)
}

pub fn bar_levels(bins: &[u8], sample_rate: f32, bar_count: usize) -> Vec<f32> {
    if bins.is_empty() || bar_count == 0 || sample_rate <= 0.0 {
        return vec![0.0; bar_count];
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "bin and bar counts are in the thousands at most"
    )]
    let bin_hertz = sample_rate / 2.0 / bins.len() as f32;
    let ratio = HIGHEST_HERTZ / LOWEST_HERTZ;

    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "bin indexes are small, positive, and clamped to the bin count"
    )]
    let bin_at = |hertz: f32| ((hertz / bin_hertz) as usize).min(bins.len());

    (0..bar_count)
        .map(|bar| {
            #[allow(clippy::cast_precision_loss, reason = "bar counts are small")]
            let (from, to) = (
                bar as f32 / bar_count as f32,
                (bar + 1) as f32 / bar_count as f32,
            );

            let start = bin_at(LOWEST_HERTZ * ratio.powf(from)).min(bins.len() - 1);
            let end = bin_at(LOWEST_HERTZ * ratio.powf(to)).max(start + 1);

            let loudest = bins[start..end].iter().copied().max().unwrap_or(0);
            f32::from(loudest) / 255.0
        })
        .collect()
}

/// Lowers each bar by its share of the bass tilt, the lowest bar the most.
pub fn tilt(levels: &[f32]) -> Vec<f32> {
    let last = levels.len().saturating_sub(1).max(1);

    levels
        .iter()
        .enumerate()
        .map(|(index, level)| {
            #[allow(clippy::cast_precision_loss, reason = "bar counts are small")]
            let share = 1.0 - index as f32 / last as f32;
            (level - BASS_TILT * share).max(0.0)
        })
        .collect()
}

/// Peak caps jump up to a louder level and otherwise fall a little each frame.
pub fn fall_peaks(peaks: &mut Vec<f32>, levels: &[f32]) {
    peaks.resize(levels.len(), 0.0);

    for (peak, level) in peaks.iter_mut().zip(levels) {
        *peak = level.max(*peak - PEAK_FALL).max(0.0);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp, reason = "levels here are exact byte fractions")]
mod tests {
    use super::*;

    const SAMPLE_RATE: f32 = 44_100.0;

    #[test]
    fn a_theme_sets_the_scanline_spacing() {
        assert_eq!(scanline_spacing(" 2 "), 2.0);
    }

    #[test]
    fn a_missing_unreadable_or_tiny_spacing_uses_the_default() {
        for value in ["", "wide", "0.5", "NaN"] {
            assert_eq!(scanline_spacing(value), DEFAULT_SCANLINE_SPACING);
        }
    }

    #[test]
    fn silence_gives_empty_bars() {
        assert_eq!(bar_levels(&[0; 512], SAMPLE_RATE, 4), vec![0.0; 4]);
    }

    #[test]
    fn no_bins_or_no_rate_gives_empty_bars() {
        assert_eq!(bar_levels(&[], SAMPLE_RATE, 3), vec![0.0; 3]);
        assert_eq!(bar_levels(&[255; 8], 0.0, 2), vec![0.0; 2]);
        assert!(bar_levels(&[255; 8], SAMPLE_RATE, 0).is_empty());
    }

    #[test]
    fn a_full_signal_fills_every_bar() {
        assert_eq!(
            bar_levels(&[255; 512], SAMPLE_RATE, BAR_COUNT),
            vec![1.0; BAR_COUNT]
        );
    }

    #[test]
    fn a_low_tone_lights_the_first_bar_only() {
        let mut bins = [0_u8; 2048];
        let bin_hertz = SAMPLE_RATE / 2.0 / 2048.0;
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "small positive index"
        )]
        let fifty_hertz = (50.0 / bin_hertz) as usize;
        bins[fifty_hertz] = 255;

        let levels = bar_levels(&bins, SAMPLE_RATE, BAR_COUNT);

        assert_eq!(levels[0], 1.0);
        assert!(levels[1..].iter().all(|level| *level == 0.0));
    }

    #[test]
    fn a_high_tone_lights_the_last_bar() {
        let mut bins = [0_u8; 2048];
        let bin_hertz = SAMPLE_RATE / 2.0 / 2048.0;
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "small positive index"
        )]
        let fifteen_kilohertz = (15_000.0 / bin_hertz) as usize;
        bins[fifteen_kilohertz] = 128;

        let levels = bar_levels(&bins, SAMPLE_RATE, BAR_COUNT);

        assert_eq!(levels[BAR_COUNT - 1], 128.0 / 255.0);
        assert_eq!(levels[0], 0.0);
    }

    #[test]
    fn bars_above_the_last_bin_read_the_last_bin() {
        assert_eq!(
            bar_levels(&[0, 0, 0, 200], 100.0, 2),
            vec![200.0 / 255.0; 2]
        );
    }

    #[test]
    fn tilt_lowers_the_bass_most_and_leaves_the_top_bar() {
        let tilted = tilt(&[1.0, 1.0, 1.0]);

        assert!((tilted[0] - (1.0 - BASS_TILT)).abs() < 1e-6);
        assert!((tilted[1] - (1.0 - BASS_TILT / 2.0)).abs() < 1e-6);
        assert_eq!(tilted[2], 1.0);
    }

    #[test]
    fn tilt_never_goes_below_zero() {
        assert_eq!(tilt(&[0.05, 0.0]), vec![0.0, 0.0]);
    }

    #[test]
    fn tilt_of_one_bar_lowers_it_fully() {
        assert!((tilt(&[0.5])[0] - (0.5 - BASS_TILT)).abs() < 1e-6);
        assert!(tilt(&[]).is_empty());
    }

    #[test]
    fn peaks_jump_to_louder_levels() {
        let mut peaks = vec![0.1, 0.5];

        fall_peaks(&mut peaks, &[0.8, 0.2]);

        assert!((peaks[0] - 0.8).abs() < 1e-6);
        assert!((peaks[1] - (0.5 - PEAK_FALL)).abs() < 1e-6);
    }

    #[test]
    fn peaks_stop_falling_at_zero() {
        let mut peaks = vec![0.01];

        fall_peaks(&mut peaks, &[0.0]);

        assert_eq!(peaks, vec![0.0]);
    }

    #[test]
    fn peaks_grow_to_the_bar_count() {
        let mut peaks = Vec::new();

        fall_peaks(&mut peaks, &[0.3, 0.4]);

        assert_eq!(peaks.len(), 2);
    }
}
