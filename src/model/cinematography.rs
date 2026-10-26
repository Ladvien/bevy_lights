//! The cinematographer's numbers: a scene's key against its fill, a practical's colour from the
//! temperature it burns at, and the lumens a fire needs for a pool of a given size.
//!
//! These are the numbers a director of photography sets before a single lamp is hung, and the ones a
//! game's night is judged by. They are engine-free on purpose: a host can compute them in its own
//! systems, a test can check them without a GPU, and the crate's examples can print them beside a
//! capture so a claim about a night is the number the frame actually has.
//!
//! Nothing here touches a light. The crate never adds a [`DirectionalLight`](https://docs.rs/bevy)
//! and never meters exposure; the fill, the sky and the camera are the host's, and this module hands
//! the host the ratio to aim for and the lumens to hang.

use core::f32::consts::TAU;

/// A night exterior's ratio of key to fill: 8:1, three stops.
///
/// The convention this crate's notes adopt for a low-key night (see `docs/research/lighting-look.md`):
/// the key is the fire, the fill is the sky, and the shadow side of a figure is three stops under the
/// lit side. It is the ratio behind "stark contrasts" in the noir literature, not a measurement.
pub const LOW_KEY: f32 = 8.0;

/// A courtyard's ratio of key to fill: 4:1, two stops.
///
/// A practical that must win the frame without going as dark as a night exterior: a figure's shadow
/// side still reads, and the fire is unmistakably the key.
pub const MOON_KEY: f32 = 4.0;

/// A lit interior's ratio of key to fill: 2:1, one stop.
///
/// The high-key end, where a fill is doing real work and the shadows are soft. Included so a host has
/// the whole craft's scale to choose from rather than one number.
pub const HIGH_KEY: f32 = 2.0;

/// The most a ratio may reach, so a zero or negative fill gives a large finite answer rather than an
/// infinity a caller then divides by.
const RATIO_CEILING: f32 = 1.0e6;

/// How many times brighter the key is than the fill: `key_lux / fill_lux`.
///
/// The illuminances are in lux (or any consistent unit; only their ratio is used). A fill of zero or
/// below is clamped to a small positive value, so the answer is large and finite rather than
/// infinite, and a `NaN` input gives `NaN` rather than a silently wrong ratio — a host with a broken
/// exposure should see it.
///
/// ```
/// # use bevy_lights::model::cinematography::*;
/// let ratio = key_to_fill(12.0, 1.5);
/// assert!((ratio - 8.0).abs() < 1e-5);
/// assert!((stops(ratio) - 3.0).abs() < 1e-5);
/// ```
pub fn key_to_fill(key_lux: f32, fill_lux: f32) -> f32 {
    if key_lux.is_nan() || fill_lux.is_nan() {
        return f32::NAN;
    }
    // A fill at or below zero has no ratio to express; the ceiling is the honest answer.
    if fill_lux <= 0.0 {
        return RATIO_CEILING;
    }
    (key_lux / fill_lux).clamp(0.0, RATIO_CEILING)
}

/// A ratio in stops (photographic exposure steps): `log2(ratio)`.
///
/// One stop is a doubling, so 8:1 is three stops. A ratio of zero or below has no logarithm and
/// answers `f32::NEG_INFINITY`, which is what a photographer would say about a scene with no key.
pub fn stops(ratio: f32) -> f32 {
    ratio.log2()
}

/// The fill illuminance that gives `ratio` against `key_lux`: the inverse of [`key_to_fill`].
///
/// This is the useful direction for a host: it knows what its fire puts on the ground and wants a
/// night, so it asks for the sky that makes 8:1 instead of hand-picking one.
///
/// ```
/// # use bevy_lights::model::cinematography::*;
/// let fill = fill_for(12.0, LOW_KEY);
/// assert!((key_to_fill(12.0, fill) - LOW_KEY).abs() < 1e-3);
/// ```
pub fn fill_for(key_lux: f32, ratio: f32) -> f32 {
    if ratio <= 0.0 || key_lux.is_nan() || ratio.is_nan() {
        return 0.0;
    }
    key_lux / ratio
}

/// The lumens a point light needs for the ground `reach` world units out, `over` world units below
/// it, to receive `lux`.
///
/// A [`PointLight`](https://docs.rs/bevy) with `intensity` lumens puts `Φ / 4π` candela out, and the
/// illuminance on a surface `slant` away and square-on to it is `candela / slant²`: so the lumens for
/// a target illuminance are `lux · 4π · slant²`. `reach` and `over` are clamped the same way
/// [`raised_light`](crate::model::reach::raised_light) clamps them, and for the same reason — a
/// degenerate geometry gets a sane light rather than an infinite one.
///
/// This is the number that gives a fire its **size**: say "12 lx at 4 m" and the lumens fall out,
/// instead of picking 60 000 because it looked right on one brazier.
///
/// ```
/// # use bevy_lights::model::cinematography::*;
/// let lumens = fire_lumens(12.0, 4.0, 0.0);
/// // On the ground at 4 m, square-on: 12 lx.
/// let candela = lumens / (4.0 * core::f32::consts::PI);
/// assert!((candela / 16.0 - 12.0).abs() < 1e-2);
/// ```
pub fn fire_lumens(lux: f32, reach: f32, over: f32) -> f32 {
    let reach = reach.max(1.0);
    let slant = reach.hypot(over.max(0.0));
    let lux = if lux.is_nan() { 0.0 } else { lux.max(0.0) };
    lux * 4.0 * core::f32::consts::PI * slant * slant
}

/// The linear RGB colour of a blackbody at `kelvin`, scaled by `strength`.
///
/// The colour a light source burns at, from its temperature: Planck's law sampled across the visible
/// spectrum, integrated against the CIE 1931 colour-matching functions and converted to linear sRGB,
/// then divided by its own largest channel so `strength` is the only scale. 1900 K is a deep
/// red-orange (a wood fire), 2700 K a warm lamp, 8000 K a cold blue (a night sky), and the three are
/// on the same curve rather than three hand-picked triples.
///
/// `strength` is clamped to `0..=1`: a light's colour is a colour, and an over-bright one is the
/// light's intensity, not its hue. A temperature at or below zero is clamped to 1000 K.
///
/// ```
/// # use bevy_lights::model::cinematography::*;
/// let fire = blackbody_colour(1900.0, 1.0);
/// let sky = blackbody_colour(8000.0, 1.0);
/// // Each is normalised to its brightest channel, so a fire's red and a sky's blue are both 1.0:
/// // the temperature shows in how little comes with them.
/// assert!(fire[2] < sky[2], "a fire has almost no blue: {fire:?} against {sky:?}");
/// assert!(fire[1] < sky[1], "and less green: {fire:?} against {sky:?}");
/// ```
pub fn blackbody_colour(kelvin: f32, strength: f32) -> [f32; 3] {
    let strength = if strength.is_nan() { 0.0 } else { strength.clamp(0.0, 1.0) };
    let linear = blackbody_linear(kelvin);
    // A blackbody's spectrum leaves the sRGB gamut at the ends (a deep red has no blue at all). Each
    // channel is clamped to what a display can show, without changing the hue's direction.
    let peak = linear.iter().fold(0.0f32, |a, b| a.max(*b));
    if peak <= 0.0 || !peak.is_finite() {
        return [strength, strength, strength];
    }
    let scale = strength / peak;
    [linear[0].max(0.0) * scale, linear[1].max(0.0) * scale, linear[2].max(0.0) * scale]
}

/// A blackbody's colour in linear sRGB **before** the gamut clamp, normalised so its brightest channel
/// is 1.
///
/// The clamp is what makes a displayable colour; it is not what a white balance divides by. Dividing
/// a clamped zero by a zero is the arithmetic that made a 1900 K balance come out grey, so the
/// balance works on this and clamps afterwards.
fn blackbody_linear(kelvin: f32) -> [f32; 3] {
    let kelvin = if kelvin.is_nan() { 6500.0 } else { kelvin.clamp(1000.0, 40_000.0) };
    let mut rgb = [0.0f32; 3];
    // 380..780 nm in 10 nm steps: 41 samples is plenty for a smooth Planckian curve.
    let mut total = 0.0f32;
    for step in 0..=40 {
        let nm = 380.0 + step as f32 * 10.0;
        let radiance = planck(kelvin, nm * 1e-9);
        rgb[0] += radiance * cie_x(nm);
        rgb[1] += radiance * cie_y(nm);
        rgb[2] += radiance * cie_z(nm);
        total += radiance;
    }
    if total <= 0.0 || !total.is_finite() {
        return [1.0, 1.0, 1.0];
    }
    // CIE XYZ to linear sRGB (the sRGB primaries' matrix), normalised to its own maximum.
    let [x, y, z] = [rgb[0] / total, rgb[1] / total, rgb[2] / total];
    let linear = [
        3.2406 * x - 1.5372 * y - 0.4986 * z,
        -0.9689 * x + 1.8758 * y + 0.0415 * z,
        0.0557 * x - 0.2040 * y + 1.0570 * z,
    ];
    let peak = linear.iter().fold(0.0f32, |a, b| a.max(*b));
    if peak <= 0.0 || !peak.is_finite() {
        return [1.0, 1.0, 1.0];
    }
    [linear[0] / peak, linear[1] / peak, linear[2] / peak]
}

/// Planck's law: the spectral radiance of a blackbody at `kelvin` for a wavelength in metres, up to a
/// constant factor (the factor cancels in [`blackbody_colour`]'s normalisation).
fn planck(kelvin: f32, metres: f32) -> f32 {
    // h c / k, in metre-kelvin: 1.4387769e-2.
    const H_C_OVER_K: f32 = 1.438_776_9e-2;
    let exponent = H_C_OVER_K / (metres * kelvin);
    // expm1 keeps the curve accurate as the exponent grows; the 1e-9 scale only keeps the values in
    // a comfortable f32 range, and the normalisation removes it.
    1.0 / (metres.powi(5) * (exponent).exp_m1()) * 1e-9
}

/// The CIE 1931 `x̄` colour-matching function, a Gaussian sum (Wyman, Sloan & Shirley 2013).
fn cie_x(nm: f32) -> f32 {
    gaussian(nm, 1.056, 599.8, 37.9, 0.362, 442.0, 16.0, -0.065, 501.1, 20.4)
}

/// The CIE 1931 `ȳ` colour-matching function, a Gaussian sum (Wyman, Sloan & Shirley 2013).
fn cie_y(nm: f32) -> f32 {
    gaussian(nm, 0.821, 568.8, 46.9, 0.286, 530.9, 16.3, 0.0, 0.0, 1.0)
}

/// The CIE 1931 `z̄` colour-matching function, a Gaussian sum (Wyman, Sloan & Shirley 2013).
fn cie_z(nm: f32) -> f32 {
    gaussian(nm, 1.217, 437.0, 11.8, 0.681, 459.0, 26.0, 0.0, 0.0, 1.0)
}

/// The three-lobe Gaussian fit the three CIE functions share.
#[allow(clippy::too_many_arguments)]
fn gaussian(nm: f32, a: f32, b: f32, c: f32, d: f32, e: f32, f: f32, g: f32, h: f32, i: f32) -> f32 {
    a * gauss(nm, b, c) + d * gauss(nm, e, f) + g * gauss(nm, h, i)
}

/// One Gaussian lobe of the CIE fit.
fn gauss(nm: f32, centre: f32, width: f32) -> f32 {
    if width <= 0.0 {
        return 0.0;
    }
    let t = (nm - centre) / width;
    (-0.5 * t * t).exp()
}

/// The colour of a light at `kelvin` as a camera **white-balanced** to `balance_kelvin` records it:
/// each channel divided by the same channel of the reference blackbody, normalised to `strength`.
///
/// This is how a night scene gets its warm-against-cool separation from temperatures rather than from
/// two hand-picked colours, and it is the cinematographer's own tool: a camera balanced for the fire
/// (a tungsten balance of 1900 K, say) records the sky as blue and the fire as neutral; balanced for
/// the sky, it records the fire as orange and the sky as neutral. Photographing a firelit night
/// balanced to the fire is the convention, and it is why moonlight reads blue in a shot.
///
/// `balance_kelvin` at or below zero is clamped to 1000 K. A balance equal to `kelvin` gives a
/// neutral grey at `strength` in every channel, which is the definition of white balance.
///
/// ```
/// # use bevy_lights::model::cinematography::*;
/// // Tungsten, CIE Illuminant A: the balance a firelit night is usually shot on.
/// let lamp = white_balanced(2856.0, 2856.0, 1.0);
/// let fire = white_balanced(1900.0, 2856.0, 1.0);
/// let sky = white_balanced(8000.0, 2856.0, 1.0);
/// assert!((lamp[0] - lamp[1]).abs() < 1e-2 && (lamp[1] - lamp[2]).abs() < 1e-2, "neutral: {lamp:?}");
/// assert!(fire[0] > fire[2], "the fire reads orange: {fire:?}");
/// assert!(sky[2] > fire[2], "the sky reads bluer: {sky:?}");
/// ```
pub fn white_balanced(kelvin: f32, balance_kelvin: f32, strength: f32) -> [f32; 3] {
    let light = blackbody_linear(kelvin);
    let reference = blackbody_linear(balance_kelvin);
    let strength = if strength.is_nan() { 0.0 } else { strength.clamp(0.0, 1.0) };
    let mut out = [0.0f32; 3];
    for (channel, value) in out.iter_mut().enumerate() {
        // A reference channel of zero has no ratio to give: the light's own channel is already the
        // gamut's edge, so it divides by a hair rather than by nothing.
        *value = light[channel] / reference[channel].max(1e-4);
    }
    // Gamut, then normalise: the clamp belongs after the division, not before it.
    let peak = out.iter().fold(0.0f32, |a, b| a.max(*b));
    if peak <= 0.0 || !peak.is_finite() {
        return [0.0, 0.0, 0.0];
    }
    let scale = strength / peak;
    [out[0].max(0.0) * scale, out[1].max(0.0) * scale, out[2].max(0.0) * scale]
}

/// A turn in radians, so a caller passing a circle's fraction does not import a constant.
pub const TURN: f32 = TAU;

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn the_key_to_fill_ratio_is_the_crafts_and_reads_in_stops() {
        assert!((key_to_fill(12.0, 1.5) - 8.0).abs() < 1e-5);
        assert!((stops(LOW_KEY) - 3.0).abs() < 1e-5);
        assert!((stops(MOON_KEY) - 2.0).abs() < 1e-5);
        assert!((stops(HIGH_KEY) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn a_fill_of_zero_is_large_and_finite_and_a_nan_shows_through() {
        let ratio = key_to_fill(12.0, 0.0);
        assert!(ratio.is_finite() && ratio >= 1.0e6 - 1.0, "no fill: {ratio}");
        assert!(stops(ratio).is_finite());
        assert!(key_to_fill(f32::NAN, 1.0).is_nan(), "a broken exposure must not be hidden");
        assert!(key_to_fill(12.0, f32::NAN).is_nan());
        assert_eq!(key_to_fill(-3.0, 1.0), 0.0, "a negative key clamps at no light");
    }

    #[test]
    fn the_fill_for_a_ratio_is_the_inverse_of_the_ratio() {
        for ratio in [LOW_KEY, MOON_KEY, HIGH_KEY, 1.0, 100.0] {
            let fill = fill_for(12.0, ratio);
            assert!((key_to_fill(12.0, fill) - ratio).abs() < 1e-3, "ratio {ratio} round-trip");
        }
        assert_eq!(fill_for(12.0, 0.0), 0.0, "a ratio of zero has no fill");
        assert_eq!(fill_for(f32::NAN, LOW_KEY), 0.0);
    }

    #[test]
    fn the_lumens_for_a_pool_are_the_inverse_of_a_raised_lights_illuminance() {
        // A fire on the ground lighting 12 lx at 4 m: the lumens `fire_lumens` gives, read back as
        // illuminance through the same geometry, must be 12 lx.
        let lumens = fire_lumens(12.0, 4.0, 0.0);
        let candela = lumens / (4.0 * core::f32::consts::PI);
        assert!((candela / 16.0 - 12.0).abs() < 1e-2, "{candela} cd");

        // Raised 1.2 m, the slant is longer and the lumens larger by its square: still 12 lx at the
        // ground point 4 m out.
        let raised = fire_lumens(12.0, 4.0, 1.2);
        let slant = 4.0f32.hypot(1.2);
        assert!((raised - lumens * (slant / 4.0).powi(2)).abs() < 1e-1, "raised {raised} against {lumens}");
        assert!(raised > lumens, "a raised fire needs more lumens for the same pool");

        // Monotone, and a degenerate reach clamps rather than exploding.
        assert!(fire_lumens(12.0, 6.0, 0.0) > lumens);
        assert!(fire_lumens(24.0, 4.0, 0.0) > lumens);
        assert!(fire_lumens(12.0, 0.0, 0.0).is_finite());
        assert_eq!(fire_lumens(-1.0, 4.0, 0.0), 0.0, "a negative lux is no light");
    }

    #[test]
    fn a_blackbody_is_warm_at_a_fires_temperature_and_cool_at_a_skys() {
        let fire = blackbody_colour(1900.0, 1.0);
        let lamp = blackbody_colour(2700.0, 1.0);
        let sky = blackbody_colour(8000.0, 1.0);
        // Each colour is normalised so its brightest channel is the strength, so red is 1.0 at every
        // temperature: warmth is how little green and blue come with it.
        let warmth = |c: [f32; 3]| c[2] / c[0];
        let green = |c: [f32; 3]| c[1] / c[0];
        assert!(warmth(fire) < warmth(lamp), "1900 K is warmer than 2700 K: {fire:?} {lamp:?}");
        assert!(warmth(lamp) < warmth(sky), "2700 K is warmer than 8000 K: {lamp:?} {sky:?}");
        assert!(green(fire) < green(sky), "the fire has less green: {fire:?} {sky:?}");
        assert!(sky[2] > 0.5, "a night sky keeps real blue: {sky:?}");
        for c in fire.iter().chain(sky.iter()) {
            assert!(c.is_finite() && (0.0..=1.0).contains(c), "{c}");
        }
        // The peak channel is the strength, so a colour is never brighter than asked.
        let strong = blackbody_colour(1900.0, 0.5);
        assert!((strong.iter().fold(0.0f32, |a, b| a.max(*b)) - 0.5).abs() < 1e-3);
        assert_eq!(blackbody_colour(1900.0, 0.0), [0.0, 0.0, 0.0]);
    }

    #[test]
    fn a_white_balance_makes_the_balance_neutral_and_everything_hotter_or_cooler_sided() {
        // A camera balanced to a tungsten lamp (CIE Illuminant A, 2856 K) is the standing convention
        // for a firelit night: the lamp is neutral, the fire reads orange and the sky reads blue.
        const TUNGSTEN: f32 = 2856.0;
        let lamp = white_balanced(TUNGSTEN, TUNGSTEN, 1.0);
        assert!((lamp[0] - lamp[1]).abs() < 1e-3 && (lamp[1] - lamp[2]).abs() < 1e-3, "neutral: {lamp:?}");
        assert!((lamp[0] - 1.0).abs() < 1e-3, "and full strength: {lamp:?}");

        let fire = white_balanced(1900.0, TUNGSTEN, 1.0);
        let sky = white_balanced(8000.0, TUNGSTEN, 1.0);
        assert!(fire[0] > fire[2], "the fire reads orange against tungsten: {fire:?}");
        assert!(sky[2] >= sky[0], "the sky reads blue against tungsten: {sky:?}");
        assert!(fire[2] < 0.3, "and the fire really has no blue: {fire:?}");
        assert!(sky[2] > 0.8, "and the sky is nearly neutral-cool: {sky:?}");

        // Balanced the other way round, the fire is the warm one by a mile: red stays the peak and
        // blue goes out of gamut (a negative in the linear colour, clamped here to nothing).
        let fire_daylight = white_balanced(1900.0, 8000.0, 1.0);
        assert!(fire_daylight[0] > fire_daylight[2], "a fire against daylight is orange: {fire_daylight:?}");
        assert!(fire_daylight[2] < 1e-3, "and has no blue at all: {fire_daylight:?}");
        assert!(fire_daylight[1] < 0.3, "and little green: {fire_daylight:?}");
        assert_eq!(white_balanced(2856.0, TUNGSTEN, 0.0), [0.0, 0.0, 0.0]);
        assert!(white_balanced(f32::NAN, TUNGSTEN, 1.0).iter().all(|c| c.is_finite()));
    }

    #[test]
    fn the_blackbody_curve_moves_monotonically_from_red_to_blue() {
        // Between 2000 K and 12000 K the blue share climbs and the red share falls. A fit that got
        // the direction wrong would make every fire the same orange.
        let mut last = blackbody_colour(2000.0, 1.0);
        for kelvin in [2500.0, 3000.0, 4000.0, 6000.0, 8000.0, 12000.0] {
            let now = blackbody_colour(kelvin, 1.0);
            assert!(now[2] >= last[2] - 1e-4, "blue fell at {kelvin} K");
            assert!(now[0] <= last[0] + 1e-4, "red rose at {kelvin} K");
            last = now;
        }
    }
}
