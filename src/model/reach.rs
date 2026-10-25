//! How far a raised fire light reaches, and how bright it must be to light the ground under it.

/// How much longer a raised light's range is than the slant distance it must cover, so the range
/// window has faded the light out before the cut-off (Bevy's `getRangeFalloff` is a windowed
/// inverse square: an abrupt cut at `range` reads as a circle on the ground).
///
/// Inherited from Sanctuary as a design default.
pub const RANGE_MARGIN: f32 = 1.25;

/// A fire light `over` world units above the ground that must light the ground `reach` units out,
/// measured flat: its intensity multiplier and its range.
///
/// The intensity is scaled by the square of the slant distance over `reach`, so the far edge is as
/// bright as a ground-level fire's is `reach` away — a raised fire loses light to distance twice
/// over, and the square puts it back. The range is the slant distance times [`RANGE_MARGIN`].
///
/// `reach` is clamped to at least one world unit, so a caller that passes a degenerate reach (a
/// zero-size room, a fire whose size a level designer left at 0) gets a sane light instead of an
/// infinite one. `over` is clamped to 0 from below for the same reason: a light under the ground
/// still lights the ground it is under.
///
/// Sanctuary decision 0062's placement rule, generalised: the units are the host's, and a host in
/// feet passes feet.
pub fn raised_light(intensity: f32, reach: f32, over: f32) -> (f32, f32) {
    let reach = reach.max(1.0);
    let slant = reach.hypot(over.max(0.0));
    (intensity * (slant / reach).powi(2), slant * RANGE_MARGIN)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn a_raised_light_is_brightened_by_its_slant_and_reaches_past_it() {
        let (intensity, range) = raised_light(1000.0, 30.0, 40.0);
        // Slant is 50, so (50 / 30)^2 = 25/9 of the lumens, and the range is 50 * 1.25 = 62.5.
        assert!((intensity - 1000.0 * (50.0f32 / 30.0).powi(2)).abs() < 1e-3);
        assert!((range - 62.5).abs() < 1e-5);
    }

    #[test]
    fn a_degenerate_reach_or_a_sunken_light_still_answers() {
        let (on_the_ground, range) = raised_light(500.0, 0.0, 3.0);
        assert!(
            (on_the_ground - 500.0 * 10.0).abs() < 1e-2,
            "reach clamps to one world unit: {on_the_ground}"
        );
        assert!((range - 1.25 * 10.0f32.sqrt()).abs() < 1e-4);
        let (below, _) = raised_light(500.0, 10.0, -4.0);
        assert!((below - 500.0).abs() < 1e-3, "over clamps at 0: {below}");
    }
}
