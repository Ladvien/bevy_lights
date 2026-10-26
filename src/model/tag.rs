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
}
