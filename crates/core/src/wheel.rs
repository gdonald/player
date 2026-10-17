/// One mouse wheel notch moves a slider by `step`: rolling away from you
/// (a negative `delta_y`) raises the value, rolling toward you lowers it.
/// The result is clamped to `min..=max` and rounded to the step.
pub fn step_value(value: f64, delta_y: f64, step: f64, min: f64, max: f64) -> f64 {
    let direction = if delta_y < 0.0 {
        1.0
    } else if delta_y > 0.0 {
        -1.0
    } else {
        0.0
    };

    let steps_per_unit = step.recip();
    let moved = ((value * steps_per_unit).round() + direction) / steps_per_unit;

    moved.clamp(min, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rolling_up_raises_the_value_by_one_step() {
        assert!((step_value(0.3, -100.0, 0.05, 0.0, 1.0) - 0.35).abs() < 1e-9);
    }

    #[test]
    fn rolling_down_lowers_the_value_by_one_step() {
        assert!((step_value(0.3, 3.0, 0.05, 0.0, 1.0) - 0.25).abs() < 1e-9);
    }

    #[test]
    fn no_vertical_roll_keeps_the_value() {
        assert!((step_value(0.3, 0.0, 0.05, 0.0, 1.0) - 0.3).abs() < 1e-9);
    }

    #[test]
    fn the_value_stops_at_the_ends() {
        assert!((step_value(1.0, -1.0, 0.05, 0.0, 1.0) - 1.0).abs() < 1e-9);
        assert!((step_value(-12.0, 1.0, 0.5, -12.0, 12.0) + 12.0).abs() < 1e-9);
    }

    #[test]
    fn a_value_between_steps_lands_on_a_step() {
        assert!((step_value(0.33, -1.0, 0.05, 0.0, 1.0) - 0.4).abs() < 1e-9);
        assert!((step_value(1.2, 1.0, 0.5, -12.0, 12.0) - 0.5).abs() < 1e-9);
    }
}
