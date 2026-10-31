//! A fire's own colour: the shipped orange ramp, moved to any hue.
//!
//! A flame's colour is a ramp — a cool `EDGE_RED` rim, the fire's own middle colour, a hot `CORE_YELLOW`
//! core — and the colour of a fire is *which ramp*: a scene's fire (`FlameLook::color`) or one fire's own
//! (`FlameSite::color`). The shipped ramp is orange, the colour of soot incandescent at wood-fire
//! temperatures. A fire that is not soot-incandescent (a gas flame, a metal salt, a magic one) has no
//! temperature to read its colour from, so it is given its middle colour and the rest of the ramp is **the
//! same flame in that hue**: [`retint`] takes a colour of the orange ramp through the transform that takes
//! [`DEFAULT_MID`] to the fire's middle colour. `docs/research/fire-colour.md` has the sources.
//!
//! The transform is in HSV on the linear values the ramp is made of: the hue turns by the difference of the
//! two middle colours' hues, the saturation scales by their ratio, the value by theirs. It is the identity
//! at [`DEFAULT_MID`] (the function returns its input, so a scene that sets no colour draws the pixels it
//! always did), and it keeps what makes the orange ramp read as fire: a rim darker and more saturated than
//! the middle, and a core whose hue sits the other way from the rim's.

/// The middle colour of the shipped ramp, a campfire's orange: `FlameLook::default().color`. A design
/// default (`docs/research/flame.md`).
pub const DEFAULT_MID: [f32; 3] = [1.0, 0.55, 0.18];
/// The shipped ramp's cool rim and tip: the coolest part of the flame, redder and darker than the middle.
pub const EDGE_RED: [f32; 3] = [0.70, 0.06, 0.005];
/// The shipped ramp's hot core: yellow-white, the other side of the middle from the rim.
pub const CORE_YELLOW: [f32; 3] = [1.0, 0.78, 0.30];
/// What the shipped ramp tends to while the fire is still catching: a young fire is redder and dimmer
/// (`docs/research/flame.md`, iteration 9).
pub const EMBER_RED: [f32; 3] = [0.95, 0.20, 0.02];
/// A spark's colour once it has cooled almost to nothing (`src/embers.wesl`).
pub const SPARK_DULL_RED: [f32; 3] = [0.80, 0.11, 0.01];
/// A spark's colour in the middle of its life (`src/embers.wesl`); a spark's hottest colour is
/// [`CORE_YELLOW`].
pub const SPARK_ORANGE: [f32; 3] = [1.0, 0.42, 0.07];

/// A colour as (hue in degrees `[0, 360)`, saturation `[0, 1]`, value `>= 0`). An achromatic colour has hue 0.
fn to_hsv(c: [f32; 3]) -> (f32, f32, f32) {
    let [r, g, b] = c;
    let max = r.max(g).max(b);
    let range = max - r.min(g).min(b);
    if range <= 0.0 || max <= 0.0 {
        return (0.0, 0.0, max);
    }
    let sector = if max == r {
        ((g - b) / range).rem_euclid(6.0)
    } else if max == g {
        (b - r) / range + 2.0
    } else {
        (r - g) / range + 4.0
    };
    (sector * 60.0, range / max, max)
}

/// The inverse of [`to_hsv`].
fn from_hsv(hue: f32, sat: f32, val: f32) -> [f32; 3] {
    let sector = hue.rem_euclid(360.0) / 60.0;
    let c = val * sat;
    let x = c * (1.0 - (sector.rem_euclid(2.0) - 1.0).abs());
    let (r, g, b) = match sector as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = val - c;
    [r + m, g + m, b + m]
}

/// A colour a ramp can be built from: finite and non-negative, so a NaN, an infinity or a negative channel
/// reads as nothing rather than as a hue.
fn sanitised(c: [f32; 3]) -> [f32; 3] {
    c.map(|v| if v.is_finite() { v.max(0.0) } else { 0.0 })
}

/// `stop`, a colour of the shipped orange ramp, moved by the transform that takes [`DEFAULT_MID`] to `mid`:
/// the fire's ramp in its own hue. `retint(EDGE_RED, mid)` is that fire's rim, `retint(CORE_YELLOW, mid)`
/// its core, `retint(EMBER_RED, mid)` its colour while it catches.
///
/// The hue turns by the difference of the two middle colours' hues, the saturation scales by the ratio of
/// their saturations (clamped to full) and the value by the ratio of their values (so an HDR fire is a
/// brighter one). A `mid` with no hue (a grey or white fire) takes the saturation to zero: the rim is a dark
/// grey and the core a bright one. A `mid` that is not finite and non-negative is read channel by channel
/// as zero. At `mid == DEFAULT_MID` it returns `stop` unchanged.
///
/// ```
/// # use bevy_lights::model::fire_colour::*;
/// // The shipped colour changes nothing.
/// assert_eq!(retint(EDGE_RED, DEFAULT_MID), EDGE_RED);
/// // A blue fire's rim is a deep cyan-blue: no red left in it.
/// let rim = retint(EDGE_RED, [0.1, 0.3, 1.0]);
/// assert!(rim[2] > rim[1] && rim[1] > rim[0], "{rim:?}");
/// ```
pub fn retint(stop: [f32; 3], mid: [f32; 3]) -> [f32; 3] {
    let mid = sanitised(mid);
    if mid == DEFAULT_MID {
        return stop;
    }
    let (default_hue, default_sat, default_val) = to_hsv(DEFAULT_MID);
    let (mid_hue, mid_sat, mid_val) = to_hsv(mid);
    let (hue, sat, val) = to_hsv(sanitised(stop));
    // A fire with no hue has no hue to turn the ramp to: it keeps the ramp's own and loses its saturation.
    let turn = if mid_sat > 0.0 { mid_hue - default_hue } else { 0.0 };
    from_hsv(hue + turn, (sat * mid_sat / default_sat).clamp(0.0, 1.0), val * mid_val / default_val)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    const STOPS: [[f32; 3]; 6] =
        [EDGE_RED, CORE_YELLOW, EMBER_RED, SPARK_DULL_RED, SPARK_ORANGE, DEFAULT_MID];

    fn near(a: [f32; 3], b: [f32; 3], tolerance: f32) -> bool {
        a.iter().zip(&b).all(|(a, b)| (a - b).abs() <= tolerance)
    }

    #[test]
    fn the_shipped_colour_changes_nothing_and_any_other_moves_the_middle_to_itself() {
        for stop in STOPS {
            assert_eq!(retint(stop, DEFAULT_MID), stop);
        }
        for mid in [
            [0.1, 0.3, 1.0],
            [0.2, 1.0, 0.3],
            [0.6, 0.2, 1.0],
            [1.0, 0.9, 0.5],
            [0.4, 0.4, 0.4],
            [3.0, 1.0, 0.2],
        ] {
            let moved = retint(DEFAULT_MID, mid);
            assert!(near(moved, mid, 1e-5), "the orange middle should become {mid:?}, not {moved:?}");
        }
    }

    #[test]
    fn a_hue_keeps_the_ramps_own_offsets() {
        // The rim sits 22.3 degrees below the middle and the core 14.1 degrees above it, in the shipped
        // ramp; any other fire keeps those offsets (the hue is the only thing that turned).
        let hue_of = |c: [f32; 3]| to_hsv(c).0;
        let offset = |stop: [f32; 3], mid: [f32; 3]| {
            (hue_of(retint(stop, mid)) - hue_of(mid) + 540.0).rem_euclid(360.0) - 180.0
        };
        for mid in [[0.1, 0.3, 1.0], [0.2, 1.0, 0.3], [0.6, 0.2, 1.0], [1.0, 0.2, 0.3]] {
            assert!((offset(EDGE_RED, mid) + 22.3).abs() < 0.5, "{mid:?}: {}", offset(EDGE_RED, mid));
            assert!((offset(CORE_YELLOW, mid) - 14.1).abs() < 0.5, "{mid:?}: {}", offset(CORE_YELLOW, mid));
        }
    }

    #[test]
    fn a_rim_is_darker_and_more_saturated_than_the_middle_whatever_the_hue() {
        for step in 0..24 {
            let mid = from_hsv(step as f32 * 15.0, 0.9, 1.0);
            let (_, mid_sat, mid_val) = to_hsv(mid);
            let (_, rim_sat, rim_val) = to_hsv(retint(EDGE_RED, mid));
            assert!(rim_val < mid_val, "hue {}: rim value {rim_val} vs {mid_val}", step * 15);
            assert!(rim_sat >= mid_sat - 1e-4, "hue {}: rim saturation {rim_sat} vs {mid_sat}", step * 15);
        }
    }

    #[test]
    fn a_grey_or_white_fire_has_a_grey_ramp() {
        for mid in [[1.0, 1.0, 1.0], [0.5, 0.5, 0.5]] {
            for stop in STOPS {
                let out = retint(stop, mid);
                assert!(near([out[0]; 3], out, 1e-6), "{stop:?} through {mid:?} is {out:?}");
            }
        }
        // The core of a white fire is brighter than its rim.
        assert!(retint(CORE_YELLOW, [1.0; 3])[0] > retint(EDGE_RED, [1.0; 3])[0]);
    }

    #[test]
    fn a_brighter_fire_is_a_brighter_ramp() {
        let dim = retint(CORE_YELLOW, [0.1, 0.3, 1.0]);
        let hot = retint(CORE_YELLOW, [0.2, 0.6, 2.0]);
        assert!(near(hot, dim.map(|v| v * 2.0), 1e-5), "{dim:?} doubled is not {hot:?}");
    }

    #[test]
    fn nothing_a_host_can_pass_makes_a_ramp_that_is_not_a_colour() {
        let hostile = [
            [0.0; 3],
            [-1.0, 0.5, 2.0],
            [f32::NAN, 1.0, 0.0],
            [f32::INFINITY, f32::NEG_INFINITY, 0.3],
            [1.0e30, 1.0e-30, 0.0],
            [0.0, 0.0, 1.0e-45],
        ];
        for mid in hostile {
            for stop in STOPS {
                let out = retint(stop, mid);
                assert!(
                    out.iter().all(|v| v.is_finite() && *v >= 0.0),
                    "{stop:?} through {mid:?} is {out:?}"
                );
            }
        }
    }
}
