//! How far a raised fire light reaches, and how bright it must be to light the ground under it.

/// How much longer a raised light's range is than the slant distance it must cover, so the range
/// window has faded the light out before the cut-off (Bevy's `getRangeFalloff` is a windowed
/// inverse square: an abrupt cut at `range` reads as a circle on the ground).
///
/// Inherited from Sanctuary as a design default.
pub const RANGE_MARGIN: f32 = 1.25;

/// Where a fire's own light belongs, relative to the origin of the flame drawn on it: `[0, height/2, 0]`
/// for a flame `height` world units tall, in the same axes as the flame's own `Transform`.
///
/// This is the centre of the flame billboard. `src/flame.wesl`'s vertex stage stands the quad on the
/// site's origin and lifts it `size · 0.5`, so half the flame's height is the middle of the fire: above
/// its foot, below its tip, and inside the glowing object at every height a host can give it.
///
/// It exists because a fire drawn on a prop and the real light it motivates are two entities, and a
/// host that places them by different numbers gets both defects at once — a light with no visible source
/// (which the viewer reads as an unexplained light) and an illuminance on the ground that the sizing
/// never intended, since `E = Φ/4πr²` is measured from wherever the light actually is. A carried fire
/// is where that is easiest to get wrong: the prop is a child of a moving figure, so a `PoolCandidate`
/// on it must not add the lift its own `Transform` already carries.
///
/// The height is clamped at zero from below, and a `NaN` or an infinite height reads as zero, so a
/// misconfigured flame gets a light at its foot rather than one under the floor or at infinity.
///
/// ```
/// # use bevy_lights::model::reach::*;
/// // A 0.35 m torch flame's light sits 17.5 cm above the flame's own foot.
/// assert_eq!(flame_light_offset(0.35), [0.0, 0.175, 0.0]);
/// ```
pub fn flame_light_offset(height: f32) -> [f32; 3] {
    let height = if height.is_finite() { height.max(0.0) } else { 0.0 };
    [0.0, height * 0.5, 0.0]
}

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
    fn a_fire_light_sits_at_the_middle_of_its_flame() {
        // The flame billboard's own centre: half its height above the site's origin, on its axis.
        for height in [0.05, 0.35, 0.5, 1.1, 4.0] {
            let offset = flame_light_offset(height);
            assert_eq!(offset, [0.0, height * 0.5, 0.0], "{height} m");
        }
        // A fire is a vertical thing: the light is on its axis, and a host adds nothing else to it.
        assert_eq!(flame_light_offset(0.35)[0], 0.0);
        assert_eq!(flame_light_offset(0.35)[2], 0.0);
    }

    #[test]
    fn a_degenerate_flame_height_puts_its_light_on_the_ground() {
        for height in [0.0, -2.0, f32::NAN, f32::NEG_INFINITY] {
            let offset = flame_light_offset(height);
            assert!(
                offset.iter().all(|v| v.is_finite()) && offset[1] >= 0.0,
                "a {height} m flame gave {offset:?}"
            );
            assert_eq!(offset, [0.0, 0.0, 0.0], "a {height} m flame");
        }
        // An infinite height is not a light at infinity: it reads as zero like the rest.
        assert_eq!(flame_light_offset(f32::INFINITY), [0.0, 0.0, 0.0]);
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
