//! A fire's own colour: the shipped orange ramp, moved to any hue.
//!
//! A flame's colour is a ramp — a cool `EDGE_RED` rim, the fire's own middle colour, a hot `CORE_YELLOW`
//! core — and the colour of a fire is *which ramp*: a scene's fire (`FlameLook::color`) or one fire's own
//! (`FlameSite::color`). The shipped ramp is orange, the colour of soot incandescent at wood-fire
//! temperatures, and its rim and core are tuned for every red, orange and yellow fire (the **fire family**),
//! which therefore keep them. A fire that is not soot-incandescent (a gas flame, a metal salt, a magic one) has
//! no temperature to read its colour from, so it is given its middle colour and the rest of the ramp is **the
//! same flame in that hue**: [`retint`] takes a colour of the orange ramp through the transform that takes
//! [`DEFAULT_MID`] to the fire's middle colour, blended in over the 60° of hue beyond the family.
//! `docs/research/fire-colour.md` has the sources.
//!
//! The transform is in HSV on the linear values the ramp is made of: the hue turns by the difference of the
//! two middle colours' hues, the saturation scales by their ratio, the value by theirs. Then each stop's
//! **luminance** is put back in the ratio to the middle's it has in the orange ramp (a rim a third of the middle, a core
//! a quarter brighter), by dimming a stop that is too bright and whitening one that is too dim: hues do not weigh
//! equally (a blue's rim is *brighter* than its body after the turn, and its core dimmer), and a flame whose rim outshines
//! its core reads as outlined, not as hottest at the heart. It keeps what makes the orange ramp read as fire: a rim darker
//! and more saturated than the middle, a core paler and brighter, and the hues the other way from each other.

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

/// How much of the **fire family** a colour is, `0` to `1`: the family is the red-to-yellow arc of hues the shipped ramp was
/// tuned for (a wood fire, a candle, a coal bed, a deep-red ember), where its rim and its core are already the red and the
/// yellow they should be, so a fire of any of those hues keeps them. A colour fades out of the family over the 60° of hue
/// beyond either end of the arc, and a colour too pale to have a hue is not in it.
fn fire_family(hue: f32, sat: f32) -> f32 {
    let smoothstep = |low: f32, high: f32, x: f32| {
        let t = ((x - low) / (high - low)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    };
    // The hue's angular distance from the arc [0°, 60°].
    let beyond = if hue <= 60.0 { 0.0 } else { (hue - 60.0).min(360.0 - hue) };
    (1.0 - smoothstep(0.0, 60.0, beyond)) * smoothstep(0.1, 0.4, sat)
}

/// The luminance of a linear colour (Rec. 709), what the ramp's brightness structure is kept in.
fn luminance(c: [f32; 3]) -> f32 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

/// `turned` with its luminance set to `target`, keeping its hue: dimmed in proportion when it is too bright, and when it is
/// too dim lightened towards the grey of its own value (its largest channel) as far as that grey allows.
fn with_luminance(turned: [f32; 3], target: f32) -> [f32; 3] {
    let have = luminance(turned);
    if target > have {
        let value = turned[0].max(turned[1]).max(turned[2]);
        let toward = ((target - have) / (value - have).max(f32::MIN_POSITIVE)).clamp(0.0, 1.0);
        turned.map(|c| c + toward * (value - c))
    } else if have > 0.0 {
        turned.map(|c| c * target / have)
    } else {
        turned
    }
}

/// `stop`, a colour of the shipped orange ramp, moved to the ramp of a fire whose middle colour is `mid`:
/// `retint(EDGE_RED, mid)` is that fire's rim, `retint(CORE_YELLOW, mid)` its core, `retint(EMBER_RED, mid)` its colour
/// while it catches.
///
/// A `mid` in the **fire family** (a red, orange or yellow of any depth: hue within `[0°, 60°]` and saturation from 0.4) leaves `stop` as it is: that
/// is the ramp every scene has drawn, whatever its fire colour. Further from the family the ramp turns with the fire: the
/// hue by the difference of the two middle colours' hues, the saturation by the ratio of their saturations (clamped to full)
/// and the value by the ratio of their values (so an HDR fire is a brighter one), in HSV on the linear values, then the stop's
/// luminance kept in the ratio to the middle's it has in the orange ramp, blended in over
/// the 60° of hue beyond the family's ends. A `mid` with no hue (a grey or white fire) is not in the family: its ramp is grey, the
/// rim a dark grey and the core a bright one. A `mid` that is not finite and non-negative is read channel by channel as zero.
///
/// ```
/// # use bevy_lights::model::fire_colour::*;
/// // A fire in the orange family changes nothing, whatever its exact colour.
/// assert_eq!(retint(EDGE_RED, DEFAULT_MID), EDGE_RED);
/// assert_eq!(retint(CORE_YELLOW, [1.0, 0.18, 0.0]), CORE_YELLOW);
/// // A blue fire's rim is a deep cyan-blue: no red left in it.
/// let rim = retint(EDGE_RED, [0.1, 0.3, 1.0]);
/// assert!(rim[2] > rim[1] && rim[1] > rim[0], "{rim:?}");
/// ```
pub fn retint(stop: [f32; 3], mid: [f32; 3]) -> [f32; 3] {
    let mid = sanitised(mid);
    let (mid_hue, mid_sat, mid_val) = to_hsv(mid);
    let family = fire_family(mid_hue, mid_sat);
    if family >= 1.0 {
        return stop;
    }
    let (default_hue, default_sat, default_val) = to_hsv(DEFAULT_MID);
    let (hue, sat, val) = to_hsv(sanitised(stop));
    // A fire with no hue has no hue to turn the ramp to: it keeps the ramp's own and loses its saturation.
    let turn = if mid_sat > 0.0 { mid_hue - default_hue } else { 0.0 };
    let turned =
        from_hsv(hue + turn, (sat * mid_sat / default_sat).clamp(0.0, 1.0), val * mid_val / default_val);
    let from = sanitised(stop);
    let turned = with_luminance(turned, luminance(from) / luminance(DEFAULT_MID) * luminance(mid));
    [0, 1, 2].map(|i| from[i] + (turned[i] - from[i]) * (1.0 - family))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    const STOPS: [[f32; 3]; 6] =
        [EDGE_RED, CORE_YELLOW, EMBER_RED, SPARK_DULL_RED, SPARK_ORANGE, DEFAULT_MID];
    /// Hues well outside the fire family, where the ramp has turned all the way: blue, green, violet, cyan.
    const OTHER_HUES: [[f32; 3]; 5] =
        [[0.1, 0.3, 1.0], [0.2, 1.0, 0.3], [0.6, 0.2, 1.0], [0.1, 0.9, 0.9], [3.0, 1.0, 0.2]];

    fn near(a: [f32; 3], b: [f32; 3], tolerance: f32) -> bool {
        a.iter().zip(&b).all(|(a, b)| (a - b).abs() <= tolerance)
    }

    #[test]
    fn a_fire_of_any_red_orange_or_yellow_keeps_the_shipped_ramp() {
        // The scene's orange, a 1900 K blackbody (the courtyard's: red, no blue), a candle's yellow, a deep red coal and an HDR
        // fire: every scene that has set `FlameLook::color` has drawn these stops, and must go on drawing them.
        for mid in [
            DEFAULT_MID,
            [1.0, 0.18, 0.0],
            [1.0, 0.8, 0.35],
            [1.0, 0.05, 0.0],
            [3.0, 1.2, 0.3],
            [1.0, 1.0, 0.1],
        ] {
            for stop in STOPS {
                assert_eq!(retint(stop, mid), stop, "{stop:?} through {mid:?}");
            }
        }
    }

    #[test]
    fn any_other_hue_takes_the_middle_of_the_orange_ramp_to_itself() {
        for mid in [[0.1, 0.3, 1.0], [0.2, 1.0, 0.3], [0.6, 0.2, 1.0], [0.1, 0.9, 0.9]] {
            let moved = retint(DEFAULT_MID, mid);
            assert!(near(moved, mid, 1e-5), "the orange middle should become {mid:?}, not {moved:?}");
        }
    }

    #[test]
    fn a_hue_keeps_the_ramps_own_offsets() {
        // The rim sits 22.3 degrees below the middle and the core 14.1 degrees above it, in the shipped
        // ramp; a fire of another hue keeps those offsets (the hue is the only thing that turned).
        let hue_of = |c: [f32; 3]| to_hsv(c).0;
        let offset = |stop: [f32; 3], mid: [f32; 3]| {
            (hue_of(retint(stop, mid)) - hue_of(mid) + 540.0).rem_euclid(360.0) - 180.0
        };
        for mid in [[0.1, 0.3, 1.0], [0.2, 1.0, 0.3], [0.6, 0.2, 1.0]] {
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

    /// The ramp must not jump anywhere on the wheel (the family's edge included): a fire whose hue a host slides across it (a
    /// slider, a day-night script) changes its rim and core a little at a time, never in a step. A quarter of a degree moves a
    /// stop by at most 0.03 of a channel (the steepest place is a dark blue rim, whose luminance falls with the hue).
    #[test]
    fn the_ramp_changes_smoothly_across_the_hue_wheel() {
        let mut last = [EDGE_RED, CORE_YELLOW, EMBER_RED];
        for step in 1..=1440 {
            let mid = from_hsv(step as f32 * 0.25, 0.9, 1.0);
            let now = [retint(EDGE_RED, mid), retint(CORE_YELLOW, mid), retint(EMBER_RED, mid)];
            for (was, is) in last.iter().zip(&now) {
                assert!(near(*was, *is, 0.03), "hue {}: {was:?} became {is:?}", step as f32 * 0.25);
            }
            last = now;
        }
    }

    /// A flame is hottest at its heart: outside the fire family a fire's rim is dimmer than its middle and its core brighter,
    /// in luminance (the turn alone made a blue rim *brighter* than the blue body, an outlined flame), in the same ratios the
    /// orange ramp has.
    #[test]
    fn a_rim_is_dimmer_and_a_core_brighter_than_the_middle_in_every_hue_outside_the_family() {
        let ratio = |stop: [f32; 3]| luminance(stop) / luminance(DEFAULT_MID);
        for step in 0..=36 {
            let mid = from_hsv(120.0 + step as f32 * 5.0, 0.9, 1.0);
            let y = luminance(mid);
            let (rim, core) = (luminance(retint(EDGE_RED, mid)), luminance(retint(CORE_YELLOW, mid)));
            assert!(rim < y && y < core + 1e-4, "hue {}: rim {rim}, middle {y}, core {core}", 120 + step * 5);
            assert!(
                (rim / y - ratio(EDGE_RED)).abs() < 0.02,
                "the rim is {} of the middle, the orange ramp's {}",
                rim / y,
                ratio(EDGE_RED)
            );
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
    fn a_brighter_fire_of_another_hue_is_a_brighter_ramp() {
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
        for mid in hostile.into_iter().chain(OTHER_HUES) {
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
