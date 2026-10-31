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

/// The illuminance, in lux, a point light of `lumens` puts on ground `reach` world units out and
/// `over` below it: [`fire_lumens`] read the other way.
///
/// `reach` and `over` are clamped exactly as [`fire_lumens`] clamps them (a reach under 1 reads as 1),
/// so `fire_lux(fire_lumens(lux, r, h), r, h) == lux` for any `r >= 1`. This is the illuminance on a
/// surface square-on to the light, the number a light is *sized* by; flat ground at that spot gets
/// less (the cosine term), which is [`ground_lux`], the number an overlay paints.
///
/// ```
/// # use bevy_lights::model::cinematography::*;
/// let lumens = fire_lumens(12.0, 4.0, 0.9);
/// assert!((fire_lux(lumens, 4.0, 0.9) - 12.0).abs() < 1e-3);
/// // Twice as far is a quarter of the light (on flat ground, ignoring the raise).
/// let near = fire_lux(lumens, 4.0, 0.0);
/// assert!((near / fire_lux(lumens, 8.0, 0.0) - 4.0).abs() < 1e-3);
/// ```
pub fn fire_lux(lumens: f32, reach: f32, over: f32) -> f32 {
    let reach = reach.max(1.0);
    let slant = reach.hypot(over.max(0.0));
    let lumens = if lumens.is_nan() { 0.0 } else { lumens.max(0.0) };
    lumens / (4.0 * core::f32::consts::PI * slant * slant)
}

/// How far out along the ground, in world units, a point light of `lumens` and `over` world units
/// above it still lights a surface square-on to it to `lux`: the radius of an iso-illuminance circle
/// by the law a light is sized with. The circle flat ground shows is [`ground_reach`].
///
/// `0.0` where even the ground square under the light is dimmer than `lux`, or where `lux` is not
/// positive (no finite circle). Never under 1 where it answers at all, because [`fire_lumens`] and
/// [`fire_lux`] read a reach under 1 as 1.
///
/// ```
/// # use bevy_lights::model::cinematography::*;
/// let lumens = fire_lumens(12.0, 4.0, 0.9);
/// assert!((fire_reach(lumens, 12.0, 0.9) - 4.0).abs() < 1e-3);
/// // A dimmer threshold lies further out.
/// assert!(fire_reach(lumens, 3.0, 0.9) > 4.0);
/// ```
pub fn fire_reach(lumens: f32, lux: f32, over: f32) -> f32 {
    if lumens.is_nan() || lux.is_nan() || lux <= 0.0 || lumens <= 0.0 {
        return 0.0;
    }
    let over = over.max(0.0);
    let slant_sq = lumens / (4.0 * core::f32::consts::PI * lux);
    let reach = (slant_sq - over * over).max(0.0).sqrt();
    if reach < 1.0 { 0.0 } else { reach }
}

/// The illuminance, in lux, a point light of `lumens` puts on **flat horizontal ground** `distance`
/// world units out from the point under it, `over` world units above it.
///
/// [`fire_lux`] is the illuminance on a surface square-on to the light. Ground is not: a surface
/// facing straight up receives `E = I · cos θ / slant²`, with `I = Φ / 4π` the candela Bevy gives a
/// point light and `cos θ = over / slant` (Lambert's cosine law, the same `N·L` the renderer shades
/// stone with). So at 4 m out and 0.9 m up the ground gets 0.22 of what [`fire_lux`] says, and the
/// difference grows with distance: this is the number to paint over a map, [`fire_lux`] the number
/// to size a light by.
///
/// `distance` is clamped at 0. A light at or below the ground (`over <= 0`) puts no light on it, so
/// that answers `0.0`, as does a `NaN` or non-positive `lumens`.
///
/// ```
/// # use bevy_lights::model::cinematography::*;
/// let lumens = fire_lumens(12.0, 4.0, 0.9);
/// // Square-on is 12 lx at the 4 m reach; flat ground there gets the cosine of it.
/// let slant = 4.0_f32.hypot(0.9);
/// let cos = 0.9 / slant;
/// assert!((ground_lux(lumens, 4.0, 0.9) - 12.0 * cos).abs() < 1e-2);
/// // Straight underneath, slant is `over`: all of the candela, `lumens / 4π / over²`.
/// let under = lumens / (4.0 * core::f32::consts::PI) / (0.9 * 0.9);
/// assert!((ground_lux(lumens, 0.0, 0.9) - under).abs() < 1e-2 * under);
/// ```
pub fn ground_lux(lumens: f32, distance: f32, over: f32) -> f32 {
    if lumens.is_nan() || lumens <= 0.0 || over.is_nan() || over <= 0.0 {
        return 0.0;
    }
    let distance = if distance.is_nan() { 0.0 } else { distance.max(0.0) };
    let slant_sq = distance * distance + over * over;
    lumens / (4.0 * core::f32::consts::PI) * over / (slant_sq * slant_sq.sqrt())
}

/// How far out along flat ground, in world units, a point light of `lumens` and `over` world units
/// above it still lights the ground to `lux`: [`ground_lux`] solved for the distance, the radius of
/// the iso-illuminance circle an overlay draws.
///
/// Unlike [`fire_reach`] it carries the cosine term, so it is the circle the ground really shows.
/// `0.0` where even the spot under the light is dimmer than `lux`, where `lux` is not positive, or
/// where the light is not above the ground. Unlike [`fire_reach`] there is no floor at 1: this is a
/// geometric answer, not a light size.
///
/// ```
/// # use bevy_lights::model::cinematography::*;
/// let lumens = fire_lumens(12.0, 4.0, 0.9);
/// let r = ground_reach(lumens, 5.0, 0.9);
/// assert!((ground_lux(lumens, r, 0.9) - 5.0).abs() < 1e-2);
/// ```
pub fn ground_reach(lumens: f32, lux: f32, over: f32) -> f32 {
    if lumens.is_nan() || lux.is_nan() || over.is_nan() || lux <= 0.0 || lumens <= 0.0 || over <= 0.0 {
        return 0.0;
    }
    // E = I·h / s³ with s the slant, so s² = (I·h / E)^(2/3).
    let intensity = lumens / (4.0 * core::f32::consts::PI);
    let slant_sq = (intensity * over / lux).powf(2.0 / 3.0);
    (slant_sq - over * over).max(0.0).sqrt()
}

/// A linear ramp `peak * (1 - d / reach)` that crosses the true ground illuminance of a point light
/// at exactly two circles: the `dim_lux` one and the `bright_lux` one. Returns `(peak, reach)`, the
/// lux under the ramp's apex and the world-unit distance where it falls to zero.
///
/// A coarse illuminance grid (a stealth or AI tile map) bakes each fire as a disc that fades
/// linearly to nothing, and a linear ramp cannot follow the inverse-square spike of [`ground_lux`]:
/// fitted from its peak it over-reads the middle of the pool several-fold. The fit is therefore made
/// where a grid is read, at the lux a tile counts as lit (`dim_lux`) and as bright (`bright_lux`):
/// the ramp passes through `(`[`ground_reach`]`(dim_lux), dim_lux)` and
/// `(`[`ground_reach`]`(bright_lux), bright_lux)`, so a grid thresholding at either value draws the
/// same circle the renderer's ground shows. Nearer the light than the bright circle it under-reads
/// the spike; past the dim circle it keeps falling to zero at `reach`.
///
/// Units stay the host's: divide `reach` by a cell size, scale `peak` to a grid's own units.
/// `(0.0, 0.0)` where the light is not above the ground, a threshold is not positive or is `NaN`,
/// `bright_lux <= dim_lux`, or the bright circle does not exist (`bright_lux` above the spot under
/// the light).
///
/// ```
/// # use bevy_lights::model::cinematography::*;
/// let lumens = fire_lumens(12.0, 4.0, 0.9);
/// let (peak, reach) = ground_ramp(lumens, 0.9, 3.0, 8.0);
/// let at = |d: f32| peak * (1.0 - d / reach);
/// let (dim, bright) = (ground_reach(lumens, 3.0, 0.9), ground_reach(lumens, 8.0, 0.9));
/// assert!((at(dim) - 3.0).abs() < 1e-2 && (at(bright) - 8.0).abs() < 1e-2);
/// assert!(reach > dim);
/// ```
pub fn ground_ramp(lumens: f32, over: f32, dim_lux: f32, bright_lux: f32) -> (f32, f32) {
    if dim_lux.is_nan() || bright_lux.is_nan() || bright_lux <= dim_lux {
        return (0.0, 0.0);
    }
    let r_dim = ground_reach(lumens, dim_lux, over);
    let r_bright = ground_reach(lumens, bright_lux, over);
    // A bright circle of radius 0 is the spot under the light; one that does not exist is also 0
    // but with `ground_lux` at the spot below the threshold.
    if r_dim <= r_bright || ground_lux(lumens, 0.0, over) < bright_lux {
        return (0.0, 0.0);
    }
    let slope = (bright_lux - dim_lux) / (r_dim - r_bright);
    let peak = bright_lux + slope * r_bright;
    (peak, peak / slope)
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

/// A blackbody's **luminance** at `kelvin`, in candelas per square metre: the number a surface's
/// emissive needs, where [`blackbody_colour`] gives only its hue.
///
/// [`blackbody_colour`] normalises to its brightest channel, so it says what colour a source is and
/// throws its magnitude away. That is the right answer for a **light**, whose intensity a host sets in
/// lumens. It is the wrong answer for a **surface**, because `StandardMaterial::emissive` is a
/// *radiance*: Bevy documents its units as cd/m² (`bevy_pbr-0.20.0/src/pbr_material.rs`, on
/// `emissive`) and multiplies it by the camera's exposure. A glowing surface written from
/// `blackbody_colour` alone has a temperature that sets its hue and a strength someone guessed.
///
/// Here the temperature sets both. Planck's law is integrated across the visible band against the CIE
/// `ȳ` curve, weighted by the luminous efficacy of 683 lm/W, and divided by π because a blackbody
/// emitter is Lambertian and emits into a hemisphere. The result is absolute, not normalised: a
/// **1200 K** surface radiates about **177 cd/m²** and a 1500 K one about **9 050 cd/m²**, a factor of
/// 51 for 300 K, which is why a fire's glow moves so fast with temperature and why a coal bed is so
/// much brighter than a stove's ironwork.
///
/// The band and the sampling are [`blackbody_colour`]'s own (380–780 nm at 10 nm), and each step is
/// weighted by its own width, so the two functions are one model read two ways rather than two
/// spectra that happen to look alike.
///
/// ```
/// # use bevy_lights::model::cinematography::*;
/// let coal = blackbody_luminance(1200.0);
/// let ember = blackbody_luminance(1000.0);
/// // Luminance climbs steeply with temperature: 200 K more is two orders of magnitude, not a shade.
/// assert!(coal > ember * 20.0, "a coal at 1200 K is far brighter than one at 1000 K: {coal} vs {ember}");
/// // Absolute, not a share of anything: a surface this hot is a light source.
/// assert!((coal - 176.5).abs() < 2.0, "a 1200 K blackbody radiates {coal} cd/m²");
/// ```
pub fn blackbody_luminance(kelvin: f32) -> f32 {
    /// The luminous efficacy of radiation at 555 nm, in lumens per watt: what turns a spectrum's
    /// radiant power into the candelas a camera records.
    const LUMINOUS_EFFICACY: f32 = 683.0;
    /// `2hc²` (W·m²), the numerator of Planck's law, times the 1e-9 [`planck`] strips to keep f32
    /// comfortable and the 10 nm sample width: what converts the sum below into W/m³ of spectral
    /// radiance integrated over the band. [`blackbody_colour`] never needs it, because it normalises.
    const SPECTRAL_SCALE: f32 = 1.191_043e-16 * 1e9 * 1e-8;

    let kelvin = if kelvin.is_nan() { 6500.0 } else { kelvin.clamp(1000.0, 40_000.0) };
    let mut luminous = 0.0f32;
    for step in 0..=40 {
        let nm = 380.0 + step as f32 * 10.0;
        luminous += planck(kelvin, nm * 1e-9) * cie_y(nm);
    }
    // The integral of the spectrum against `ȳ`, in W/(m²·sr), times the efficacy: candelas per square
    // metre. There is no π here — `planck` is already a *radiance*, so the Lambertian division that
    // would turn an exitance into one has nothing to undo.
    LUMINOUS_EFFICACY * SPECTRAL_SCALE * luminous
}

/// The lowest temperature [`blackbody_radiance`] takes, in kelvin: well under the point (about 800 K, the Draper point) where a body first
/// glows visibly red, so a surface cooling past that point can be followed to where it is dark.
pub const MIN_RADIANCE_KELVIN: f32 = 500.0;

/// A blackbody's **radiance** at `kelvin` as linear sRGB, in candelas per square metre: its luminance and its hue in one triple, down to
/// [`MIN_RADIANCE_KELVIN`].
///
/// [`blackbody_luminance`] and [`blackbody_colour`] stop at 1000 K, where a coal bed's lowest stop sits, because they describe a fire's own
/// colour. A bed of coals *cooling* goes through the range below it (the glow is red, then a deeper red, then nothing), so this one
/// integrates the same Planck spectrum against all three colour-matching functions and keeps the magnitude: the XYZ tristimulus in cd/m²,
/// turned to linear sRGB and clamped at zero where a deep red leaves the gamut (a blue channel below zero is no light, not negative light).
///
/// ```
/// # use bevy_lights::model::cinematography::*;
/// let hot = blackbody_radiance(1300.0);
/// let cool = blackbody_radiance(900.0);
/// // 400 K is three orders of magnitude, and the cooler body is the redder one.
/// assert!(hot[0] > cool[0] * 300.0);
/// assert!(cool[1] <= cool[0] * 0.1 && cool[2] <= cool[0] * 0.01);
/// // Under 1000 K it keeps falling where `blackbody_luminance` has stopped.
/// assert!(blackbody_radiance(800.0)[0] < cool[0]);
/// assert_eq!(blackbody_luminance(800.0), blackbody_luminance(1000.0));
/// ```
pub fn blackbody_radiance(kelvin: f32) -> [f32; 3] {
    const LUMINOUS_EFFICACY: f32 = 683.0;
    const SPECTRAL_SCALE: f32 = 1.191_043e-16 * 1e9 * 1e-8;

    let kelvin = if kelvin.is_nan() { 6500.0 } else { kelvin.clamp(MIN_RADIANCE_KELVIN, 40_000.0) };
    let (mut x, mut y, mut z) = (0.0f32, 0.0f32, 0.0f32);
    for step in 0..=40 {
        let nm = 380.0 + step as f32 * 10.0;
        let radiance = planck(kelvin, nm * 1e-9);
        x += radiance * cie_x(nm);
        y += radiance * cie_y(nm);
        z += radiance * cie_z(nm);
    }
    let scale = LUMINOUS_EFFICACY * SPECTRAL_SCALE;
    let (x, y, z) = (x * scale, y * scale, z * scale);
    [
        (3.2406 * x - 1.5372 * y - 0.4986 * z).max(0.0),
        (-0.9689 * x + 1.8758 * y + 0.0415 * z).max(0.0),
        (0.0557 * x - 0.2040 * y + 1.0570 * z).max(0.0),
    ]
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
