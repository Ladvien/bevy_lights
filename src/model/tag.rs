//! A flame's three per-instance numbers — its clock phase, its height and its brightness — packed into
//! the unsigned integer a mesh tag carries.
//!
//! Every flame in a scene shares one material (one draw, one clock), so the numbers that make two
//! fires burn out of step must ride on the mesh's `MeshTag` instead of in the material. The shader
//! reads them back with WGSL's `unpack2x16unorm` (for the height) and two byte extractions (for the
//! phase and the brightness).
//!
//! The layout is the height's own 16 bits high, then the phase's 8, then the brightness's 8:
//!
//! ```text
//! bit  31                          16 15       8 7        0
//!      [        height 16          ][ phase 8 ][ bright 8 ]
//! ```
//!
//! The height keeps the full 16 bits because it is the number a host chooses and can see: a 0.35 m
//! torch and a 1.1 m brazier are a factor of three apart, and the packer's job is to reproduce both at
//! any `max_height`. The phase and the brightness are both animation values with generous headroom:
//! 2π/256 is 0.025 rad, four milliseconds of clock at 6 Hz, and 1/255 is 0.4 % of a full-intensity
//! flame against a swing of 30 % of it.

use core::f32::consts::TAU;

/// The fewest brightness bits the fragment stage can read from one byte.
const BYTE: f32 = 255.0;

/// Packs a flame's `phase` radians, its `height` (in world units) and its `brightness` into a mesh tag.
///
/// `phase` is wrapped to `[0, TAU)` first, so a caller may hand in any angle. `height` is taken as a
/// fraction of `max_height`, clamped to 0..=1: a `max_height` of zero or below packs a height of 0
/// (there is no scale to express it against), and a height above `max_height` packs 1.
/// `brightness` is clamped to 0..=1 and is the flame's own intensity share, which the crate writes
/// from the fire's [`flicker_swing`](crate::model::fire::flicker_swing).
///
/// The quantisation is deliberate: the phase error is under 4e-3 rad and the height error under
/// `max_height / 65535`, both invisible at any capture size; the brightness error is under 1/255 of
/// full intensity, which is 1.3 % of the swing it carries.
pub fn pack_flame_tag(phase: f32, height: f32, brightness: f32, max_height: f32) -> u32 {
    let unorm16 = |v: f32| (v.clamp(0.0, 1.0) * 65535.0).round() as u32;
    let unorm8 = |v: f32| (v.clamp(0.0, 1.0) * BYTE).round() as u32;
    let height_share = if max_height > 0.0 { height / max_height } else { 0.0 };
    unorm16(height_share) << 16 | unorm8(phase.rem_euclid(TAU) / TAU) << 8 | unorm8(brightness)
}

/// Reads a flame tag back: `(phase radians, height in world units, brightness)`.
///
/// The phase comes back in `[0, TAU)`, not as the negative angle it may have been packed from: the
/// shader's `unpack` does the same, so the CPU and the GPU agree.
pub fn unpack_flame_tag(tag: u32, max_height: f32) -> (f32, f32, f32) {
    let height = (tag >> 16) as f32 / 65535.0;
    let phase = ((tag >> 8) & 0xff) as f32 / BYTE;
    let brightness = (tag & 0xff) as f32 / BYTE;
    (phase * TAU, height * max_height.max(0.0), brightness)
}

/// How far from its origin the flame's vertex stage can put a corner, in drawn flame heights: the
/// ball a billboard is certain to stay inside, whichever way the camera faces and however far the
/// wind leans it (`1.64` at the worst corner, rounded up).
///
/// **Mirrors `src/flame.wesl`**: the quad is `FLAME_ASPECT + |lean|` wide and shifted by half the
/// lean, so a corner sits at most `0.3 + sin θ` flame heights to the side (`sin θ ≤ 0.995`), and at
/// most one flame height up when upright, or `0.35 + 0.5` along the camera's screen-up when it turns
/// to face the camera from above. The test below walks the shader's own corner formula across the
/// camera's directions and fails when a constant here and a constant there drift apart.
pub const FLAME_REACH: f32 = 1.7;

/// The catch's power in a flame's drawn size, `src/flame.wesl`'s `CATCH_SIZE_EXPONENT`: a flame
/// lit with its light is drawn at `took^(2/3)` of its height (`docs/research/flame-catch-size.md`).
const CATCH_SIZE_EXPONENT: f32 = 2.0 / 3.0;

/// The smallest instance scale a flame's culling box divides by, the shader's own floor for the catch
/// (`max(took, 0.0001)`): a lean of `-1` would make a scale zero, and a box must stay finite.
const MIN_SCALE: f32 = 1.0e-4;

/// The half-extents of the box, in the flame entity's **local** axes, that a frustum test must use so
/// that it holds everything the flame's vertex stage draws.
///
/// A flame is a billboard the vertex stage builds from three per-instance numbers it reads out of the
/// entity's transform: its catch `took` (the scale's `x`, in `0..=1`) and the wind's `lean` (the scale's
/// `y` and `z` are `1 +` its world `x` and `z`). The CPU never sees the quad, only the mesh's unit square
/// and the transform, so Bevy's own `Aabb` neither covers the drawn flame nor survives a scale that is
/// data rather than size. This is the box that does: a cube of side `2 · FLAME_REACH · size` about the
/// flame's origin, where `size` is the height the shader draws (`height · took^(2/3)`), divided axis by
/// axis by the scale the frustum test will multiply it by. The test's world-space box is therefore the
/// same cube whatever the lean, the catch or the parent's rotation, and it holds the flame's ball.
///
/// `height` is the flame's own height in world units ([`unpack_flame_tag`]'s second value).
///
/// ```
/// # use bevy_lights::model::tag::*;
/// let [x, y, z] = flame_cull_extents(1.0, 1.0, [0.0, 0.0]);
/// assert!((x - FLAME_REACH).abs() < 1e-6 && x == y && y == z, "still air: a cube about the flame");
/// let [_, y, _] = flame_cull_extents(1.0, 1.0, [-0.9, 0.0]);
/// assert!((y - 10.0 * FLAME_REACH).abs() < 1e-3, "a lean squashes the scale to 0.1, so the box is stretched tenfold");
/// ```
pub fn flame_cull_extents(height: f32, took: f32, lean: [f32; 2]) -> [f32; 3] {
    cull_extents(FLAME_REACH, height, took, lean)
}

/// The ember quad's width either side of the flame's axis, in flame heights, in still air: `HALF_WIDTH` in `src/embers.wesl`.
const EMBER_HALF_WIDTH: f32 = 0.7;
/// How far up the ember quad reaches, in flame heights: `RISE_MAX` in `src/embers.wesl`.
const EMBER_RISE_MAX: f32 = 2.6;
/// The ember quad's margin round its sparks, in flame heights: `MARGIN` in `src/embers.wesl`.
const EMBER_MARGIN: f32 = 0.35;
/// How far a spark's drift outruns the flame's lean: `DRIFT` in `src/embers.wesl`.
const EMBER_DRIFT: f32 = 1.5;

/// How far from its origin the ember quad's vertex stage can put a corner, in drawn flame heights, for a flame leaning by
/// `lean` (the scale's `y` and `z` less one): the ball the quad is certain to stay inside whichever way the camera faces.
///
/// **Mirrors `src/embers.wesl`**: the quad is a rectangle in the plane facing the camera, `across` the horizontal camera
/// axis and `climb` its perpendicular, so a corner is `sqrt(across² + climb²)` from the foot. `climb` reaches at most
/// `RISE_MAX + MARGIN` (`2.95`, the camera level with the source) and `across` at most `HALF_WIDTH + DRIFT · RISE_MAX · |lean|`,
/// because the quad widens downwind by the drift a spark has at the top of its rise. That is `3.03` flame heights in still air
/// and `5.27` at the lean limit `sin 70° = 0.94`. A `lean` that is not finite counts as none.
pub fn ember_reach(lean: [f32; 2]) -> f32 {
    let wind = lean[0].hypot(lean[1]);
    let wind = if wind.is_finite() { wind } else { 0.0 };
    let across = EMBER_HALF_WIDTH + EMBER_DRIFT * EMBER_RISE_MAX * wind;
    across.hypot(EMBER_RISE_MAX + EMBER_MARGIN)
}

/// [`flame_cull_extents`] for the flame's ember quad (`FlameEmbers`): the half-extents, in the quad's **local** axes, of the box
/// a frustum test must use to hold every corner the embers' vertex stage can draw.
///
/// The quad is a child of the flame at the identity, so it inherits the flame's instance scale, and the box is divided by it as
/// the flame's is. The ball is wider than the flame's ([`ember_reach`] against [`FLAME_REACH`]) and widens with the lean, so a
/// box built once for still air would cull a downwind fire's sparks while they are still on screen.
///
/// ```
/// # use bevy_lights::model::tag::*;
/// let [x, y, z] = ember_cull_extents(1.0, 1.0, [0.0, 0.0]);
/// assert!((x - ember_reach([0.0, 0.0])).abs() < 1e-6 && x == y && y == z, "still air: a cube about the foot");
/// assert!(x > FLAME_REACH, "wider than the flame it hangs on");
/// let lean = [0.0, 0.5];
/// let [_, y, z] = ember_cull_extents(1.0, 1.0, lean);
/// assert!((y - ember_reach(lean)).abs() < 1e-5, "the scale's y is 1, so the box is the ball");
/// assert!((z * 1.5 - ember_reach(lean)).abs() < 1e-5, "the frustum test multiplies z by its scale, 1.5, and gets the ball back");
/// ```
pub fn ember_cull_extents(height: f32, took: f32, lean: [f32; 2]) -> [f32; 3] {
    cull_extents(ember_reach(lean), height, took, lean)
}

/// A cube of side `2 · reach · size` about the foot, divided axis by axis by the instance scale; `size` is the drawn height.
fn cull_extents(reach: f32, height: f32, took: f32, lean: [f32; 2]) -> [f32; 3] {
    let took = if took.is_nan() { 0.0 } else { took.clamp(0.0, 1.0) };
    let size = height.max(0.0) * took.max(1.0e-4).powf(CATCH_SIZE_EXPONENT);
    let reach = reach * size;
    [took, 1.0 + lean[0], 1.0 + lean[1]].map(|scale| reach / scale.max(MIN_SCALE))
}

/// A flame tag with only its brightness written: `tag`'s height and phase untouched.
///
/// The brightness is the one number a running scene rewrites every frame, and decoding the other two
/// to re-pack them would put their quantisation through a round trip for no reason. `brightness` is
/// clamped to 0..=1 like [`pack_flame_tag`]'s.
pub fn set_flame_brightness(tag: u32, brightness: f32) -> u32 {
    let unorm8 = (brightness.clamp(0.0, 1.0) * BYTE).round() as u32;
    (tag & !0xff) | unorm8
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    /// The largest error the phase's 8 bits can carry, in radians.
    const PHASE_STEP: f32 = TAU / 256.0;

    #[test]
    fn a_flame_tag_round_trips_its_phase_height_and_brightness() {
        for (phase, height, brightness) in
            [(0.0, 0.0, 0.0), (1.0, 2.5, 0.85), (3.0, 8.0, 1.0), (TAU - 0.001, 0.35, 0.70), (-2.0, 1.5, 0.93)]
        {
            let tag = pack_flame_tag(phase, height, brightness, 8.0);
            let (read_phase, read_height, read_brightness) = unpack_flame_tag(tag, 8.0);
            let expected = phase.rem_euclid(TAU);
            assert!(
                (read_phase - expected).abs() <= PHASE_STEP / 2.0 + f32::EPSILON,
                "phase {phase} read back as {read_phase}, wanted {expected}"
            );
            assert!((read_height - height).abs() < 1e-3, "height {height} read back as {read_height}");
            assert!(
                (read_brightness - brightness).abs() <= 0.5 / BYTE + f32::EPSILON,
                "brightness {brightness} read back as {read_brightness}"
            );
        }
    }

    #[test]
    fn the_three_fields_do_not_bleed_into_each_other() {
        // The height at full, the phase and the brightness at zero: the two low bytes must be 0.
        let tag = pack_flame_tag(0.0, 8.0, 0.0, 8.0);
        assert_eq!(tag >> 16, 65535);
        assert_eq!(tag & 0xffff, 0);
        // The brightness alone must not disturb the height or the phase.
        let bright = pack_flame_tag(0.0, 8.0, 1.0, 8.0);
        assert_eq!(bright >> 16, tag >> 16);
        assert_eq!(bright & 0xff, 255);
        assert_eq!((bright >> 8) & 0xff, 0);
    }

    #[test]
    fn a_height_over_the_maximum_packs_full_and_a_zero_maximum_packs_nothing() {
        assert_eq!(pack_flame_tag(0.0, 20.0, 1.0, 8.0) >> 16, 65535);
        assert_eq!(pack_flame_tag(0.0, 4.0, 1.0, 0.0) >> 16, 0);
        assert_eq!(pack_flame_tag(0.0, 4.0, 1.0, -3.0) >> 16, 0);
        assert_eq!(unpack_flame_tag(pack_flame_tag(0.0, 4.0, 1.0, 0.0), 0.0).1, 0.0);
    }

    #[test]
    fn a_brightness_outside_its_range_clamps() {
        assert_eq!(pack_flame_tag(0.0, 1.0, 3.0, 8.0) & 0xff, 255);
        assert_eq!(pack_flame_tag(0.0, 1.0, -3.0, 8.0) & 0xff, 0);
    }

    #[test]
    fn a_flames_box_is_the_same_cube_about_its_foot_whatever_the_scale_does_to_it() {
        // The frustum test multiplies the box by the instance scale, which is the catch on `x` and `1 +` the lean
        // on `y` and `z`. What it builds is the cube round the drawn flame, whatever those are.
        for (height, took, lean) in [
            (0.35, 1.0, [0.0, 0.0]),
            (0.35, 0.2, [0.5, -0.3]),
            (2.0, 0.001, [-0.94, 0.0]),
            (1.1, 0.7, [0.3, 0.94]),
        ] {
            let half = flame_cull_extents(height, took, lean);
            let scale = [took, 1.0 + lean[0], 1.0 + lean[1]];
            let size = height * took.powf(2.0 / 3.0);
            for axis in 0..3 {
                let world = half[axis] * scale[axis];
                assert!(
                    (world - FLAME_REACH * size).abs() < 1e-4 * (1.0 + world),
                    "axis {axis}: height {height}, catch {took}, lean {lean:?}: {world} against {}",
                    FLAME_REACH * size
                );
            }
        }
    }

    #[test]
    fn a_flames_box_stays_finite_for_inputs_the_shader_clamps() {
        // A flare overshoots the catch, a lean of -1 zeroes a scale, a `NaN` is a host's bug: none may make a box
        // that is infinite or negative, which would cull everything or nothing.
        for (height, took, lean) in [
            (1.0, 0.0, [0.0, 0.0]),
            (1.0, f32::NAN, [0.0, 0.0]),
            (-1.0, 1.0, [0.0, 0.0]),
            (1.0, 1.0, [-1.0, -1.0]),
            (1.0, 7.0, [3.0, 3.0]),
            (1.0, 1.0, [f32::NAN, 0.2]),
            (1.0, 1.0, [f32::INFINITY, 0.0]),
        ] {
            for half in [flame_cull_extents(height, took, lean), ember_cull_extents(height, took, lean)] {
                assert!(
                    half.iter().all(|h| h.is_finite() && *h >= 0.0),
                    "{height} {took} {lean:?} gave {half:?}"
                );
            }
        }
        // The catch above 1 is a flare, which the shader clamps: the cube stays the full-grown flame's.
        assert_eq!(flame_cull_extents(1.0, 7.0, [0.0, 0.0]), flame_cull_extents(1.0, 1.0, [0.0, 0.0]));
        assert_eq!(ember_cull_extents(1.0, 7.0, [0.0, 0.0]), ember_cull_extents(1.0, 1.0, [0.0, 0.0]));
    }

    #[test]
    fn an_embers_box_is_the_same_cube_about_its_foot_whatever_the_scale_does_to_it() {
        // As the flame's, with the reach the lean makes: the world box the frustum test builds is the ball's cube.
        for (height, took, lean) in [
            (0.35, 1.0, [0.0, 0.0]),
            (0.35, 0.2, [0.5, -0.3]),
            (2.0, 0.001, [-0.94, 0.0]),
            (1.1, 0.7, [0.3, 0.94]),
        ] {
            let half = ember_cull_extents(height, took, lean);
            let scale = [took, 1.0 + lean[0], 1.0 + lean[1]];
            let size = height * took.powf(2.0 / 3.0);
            let want = ember_reach(lean) * size;
            for axis in 0..3 {
                let world = half[axis] * scale[axis];
                assert!(
                    (world - want).abs() < 1e-4 * (1.0 + world),
                    "axis {axis}: height {height}, catch {took}, lean {lean:?}: {world} against {want}"
                );
            }
        }
    }

    #[test]
    fn an_embers_reach_grows_with_the_lean_and_never_falls_short_of_the_flames() {
        let still = ember_reach([0.0, 0.0]);
        let gale = ember_reach([0.94, 0.0]);
        assert!(
            still > FLAME_REACH,
            "the sparks' quad is wider than the flame: {still} against {FLAME_REACH}"
        );
        assert!((still - 3.03).abs() < 0.01, "still air: {still}");
        assert!((gale - 5.27).abs() < 0.01, "at the lean limit: {gale}");
        // The lean's direction does not matter, only its length: the quad widens along the camera's horizontal axis, which
        // may be any way round the fire.
        assert!((ember_reach([0.6, 0.8]) - ember_reach([1.0, 0.0])).abs() < 1e-6);
        assert!((ember_reach([-0.6, 0.8]) - ember_reach([1.0, 0.0])).abs() < 1e-6);
    }
}
