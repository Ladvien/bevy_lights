//! A flame's two per-instance numbers — its clock phase and its height — packed into the unsigned
//! integer a mesh tag carries.
//!
//! Every flame in a scene shares one material (one draw, one clock), so the numbers that make two
//! fires burn out of step must ride on the mesh's `MeshTag` instead of in the material. The shader
//! reads them back with WGSL's `unpack2x16unorm`, which is exactly this layout: phase
//! unsigned-normalised in the low 16 bits, height in the high 16.

use core::f32::consts::TAU;

/// Packs a flame's `phase` radians and its `height` (in world units) into a mesh tag.
///
/// `phase` is wrapped to `[0, TAU)` first, so a caller may hand in any angle. `height` is taken as a
/// fraction of `max_height`, clamped to 0..=1: a `max_height` of zero or below packs a height of 0
/// (there is no scale to express it against), and a height above `max_height` packs 1.
///
/// The one-per-65535 quantisation is deliberate: the phase error is under 1e-4 rad and the height
/// error under `max_height / 65535`, both invisible at any capture size.
pub fn pack_flame_tag(phase: f32, height: f32, max_height: f32) -> u32 {
    let unorm = |v: f32| (v.clamp(0.0, 1.0) * 65535.0).round() as u32;
    let height_share = if max_height > 0.0 { height / max_height } else { 0.0 };
    unorm(phase.rem_euclid(TAU) / TAU) | (unorm(height_share) << 16)
}

/// Reads a flame tag back: `(phase radians, height in world units)`, where the height is the packed
/// fraction times `max_height`.
///
/// The phase comes back in `[0, TAU)`, not as the negative angle it may have been packed from:
/// `unpack2x16unorm` in the shader does the same, so the CPU and the GPU agree.
pub fn unpack_flame_tag(tag: u32, max_height: f32) -> (f32, f32) {
    let low = (tag & 0xffff) as f32 / 65535.0;
    let high = (tag >> 16) as f32 / 65535.0;
    (low * TAU, high * max_height.max(0.0))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn a_flame_tag_round_trips_its_phase_and_height() {
        for (phase, height) in [(0.0, 0.0), (1.0, 2.5), (3.0, 8.0), (TAU - 0.001, 0.35), (-2.0, 1.5)] {
            let tag = pack_flame_tag(phase, height, 8.0);
            let (read_phase, read_height) = unpack_flame_tag(tag, 8.0);
            let expected = phase.rem_euclid(TAU);
            assert!(
                (read_phase - expected).abs() < 1e-3,
                "phase {phase} read back as {read_phase}, wanted {expected}"
            );
            assert!((read_height - height).abs() < 1e-3, "height {height} read back as {read_height}");
        }
    }

    #[test]
    fn a_height_over_the_maximum_packs_full_and_a_zero_maximum_packs_nothing() {
        assert_eq!(pack_flame_tag(0.0, 20.0, 8.0) >> 16, 65535);
        assert_eq!(pack_flame_tag(0.0, 4.0, 0.0) >> 16, 0);
        assert_eq!(pack_flame_tag(0.0, 4.0, -3.0) >> 16, 0);
        assert_eq!(unpack_flame_tag(pack_flame_tag(0.0, 4.0, 0.0), 0.0).1, 0.0);
    }
}
