//! The shape of a flame: the spacing of the puffs up it, and how many of them a flame of a given
//! height shows.
//!
//! A buoyant diffusion flame is not one smooth teardrop. The toroidal vortex that sheds from the
//! flame sheet grows and detaches once per flicker period, and each detached puff is a bulge in the
//! flame's silhouette with a waist above it (Xia & Zhang 2018, `docs/research/flame-silhouette.md`).
//! Those bulges are what make a flame's two edges independent curves; a silhouette with no bulges is
//! a mirror, and a mirror reads as a decal.
//!
//! The spacing is not a free parameter. The vortex sheet is carried downstream by `H = U₀·τ` in one
//! period (Xia & Zhang, §II), so successive puffs sit `λ = U₀ / f` apart. With the buoyant velocity
//! scale `U₀ ≈ √(g·D)` of a fuel bed `D` across, and the crate's own `f = 1.5/√D`
//! ([`flicker_rate`](crate::model::fire::flicker_rate)), that is `λ = (√g / 1.5) · D ≈ 2.09 D`: the
//! spacing scales with the **bed**, not with the flame.

/// Gravitational acceleration, in m/s², for a metre world.
///
/// The same `g` the puffing law's constant folds in: [`PUFFING_CONSTANT`] is 1.5 because
/// `0.48·√9.8 ≈ 1.503`, so the two must not be changed independently.
///
/// [`PUFFING_CONSTANT`]: crate::model::fire::PUFFING_CONSTANT
pub const GRAVITY: f32 = 9.8;

/// The wavelength of the puffs a fire of fuel-bed `diameter` sheds, in the host's world units.
///
/// `λ = U₀ / f` with `U₀ = √(g·D)` and `f = 1.5/√D`, which simplifies to `(√g / 1.5)·D ≈ 2.09·D`: a
/// 0.34 m coal bed sheds a puff every 0.71 m, a 0.19 m torch's bed every 0.40 m. The spacing is
/// **proportional to the bed**, so a bigger fire's bulges are further apart even though its flame is
/// taller — a brazier's flame is lumpy over a longer distance, a torch's is one lump.
///
/// `diameter` is in the host's world units (metres for a metre world). A bed of zero or below, or a
/// `NaN`, gives a `NaN`-free finite answer: the same 1 cm floor [`flicker_rate`] uses, so the two
/// functions describe the same fire at every input.
///
/// [`flicker_rate`]: crate::model::fire::flicker_rate
///
/// ```
/// # use bevy_lights::model::flame::*;
/// assert!((puff_wavelength(1.0) - 2.087).abs() < 1e-2);
/// assert!((puff_wavelength(0.34) - 0.710).abs() < 1e-2, "a brazier's coals");
/// assert!(puff_wavelength(0.19) < puff_wavelength(0.34), "a smaller bed, closer puffs");
/// ```
pub fn puff_wavelength(diameter: f32) -> f32 {
    let bed = if diameter.is_nan() { 0.01 } else { diameter.clamp(0.01, 1.0e6) };
    GRAVITY.sqrt() / crate::model::fire::PUFFING_CONSTANT * bed
}

/// How many puffs fit up a flame `height` world units tall, for a fuel bed `diameter` across:
/// `height / puff_wavelength(diameter)`.
///
/// This is the number that decides whether a flame reads as a lumpy fire or a smooth tongue. Under
/// about 1 the flame is shorter than one puff and is a single tongue; a brazier's 1.1 m flame on a
/// 0.34 m bed is 1.55, so it shows a bulge and the start of a second. It is also the frequency the
/// shader's silhouette modulation has to be run at, and so the number a capture is judged against.
///
/// A height of zero or below is 0 puffs; a bed of zero or below, or a `NaN` in either, is 0.
///
/// ```
/// # use bevy_lights::model::flame::*;
/// assert!((flame_bulges(1.1, 0.34) - 1.55).abs() < 1e-2, "a brazier: one bulge and the start of another");
/// assert!((flame_bulges(0.35, 0.19) - 0.88).abs() < 1e-2, "a torch: a single tongue");
/// assert_eq!(flame_bulges(0.0, 0.34), 0.0);
/// ```
pub fn flame_bulges(height: f32, diameter: f32) -> f32 {
    if !height.is_finite() || height <= 0.0 || !diameter.is_finite() || diameter <= 0.0 {
        return 0.0;
    }
    height / puff_wavelength(diameter)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn the_puff_spacing_follows_the_puffing_law() {
        // λ = U₀ / f, and the crate's f is the law's: the two cannot drift apart.
        for bed in [0.05, 0.19, 0.34, 1.0, 4.0] {
            // λ = U₀ / f with U₀ = √(g·D), so the buoyant velocity is the *bed's*, not the world's: √D is in it.
            let buoyant = (GRAVITY * bed).sqrt();
            let law = buoyant / crate::model::fire::flicker_rate(bed);
            assert!(
                (puff_wavelength(bed) - law).abs() < 1e-4,
                "puff_wavelength({bed}) = {} but U₀/f = {law}",
                puff_wavelength(bed)
            );
        }
    }

    #[test]
    fn the_puff_spacing_scales_with_the_bed() {
        assert!((puff_wavelength(1.0) - 2.087).abs() < 1e-2);
        assert!((puff_wavelength(0.34) - 0.710).abs() < 1e-2, "a brazier's coals");
        assert!((puff_wavelength(0.19) - 0.397).abs() < 1e-2, "a torch's head");
        let mut previous = 0.0;
        for step in 1..200 {
            let bed = step as f32 * 0.01;
            let now = puff_wavelength(bed);
            assert!(now > previous, "λ must grow with the bed: {bed} gave {now} after {previous}");
            previous = now;
        }
    }

    #[test]
    fn a_degenerate_bed_or_a_nan_still_gives_a_finite_spacing() {
        for bed in [0.0, -1.0, -1e9, f32::NAN] {
            let lambda = puff_wavelength(bed);
            assert!(lambda.is_finite() && lambda > 0.0, "bed {bed} gave {lambda}");
        }
        // A huge bed is clamped, not an infinity.
        assert!(puff_wavelength(1e30).is_finite());
    }

    #[test]
    fn bulges_are_the_flame_over_a_puff() {
        assert!((flame_bulges(1.1, 0.34) - 1.55).abs() < 1e-2, "a brazier");
        assert!((flame_bulges(0.35, 0.19) - 0.88).abs() < 1e-2, "a torch");
        assert!((flame_bulges(2.0, 1.0) - 0.958).abs() < 1e-2, "a tall flame on a big bed");
        assert!(flame_bulges(1.1, 0.19) > flame_bulges(1.1, 0.34), "a small bed, more bulges");
    }

    #[test]
    fn bulges_of_nothing_are_zero() {
        for (height, bed) in [(0.0, 0.34), (-1.0, 0.34), (1.1, 0.0), (1.1, -1.0)] {
            assert_eq!(flame_bulges(height, bed), 0.0, "{height} m on a {bed} m bed");
        }
        for (height, bed) in [(f32::NAN, 0.34), (1.1, f32::NAN), (f32::INFINITY, 0.34)] {
            assert_eq!(flame_bulges(height, bed), 0.0, "{height} m on a {bed} m bed");
        }
    }
}
