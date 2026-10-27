//! A fuel bed's char field: how much of a bed of coals, at a point, is glowing fuel and how much is
//! spent char.
//!
//! A bed of coals is the one part of a fire that is *geometry*, and it is a surface with a
//! temperature field on it rather than a uniform emitter: some of it is burnt through and dark, some
//! of it is at the temperature that glows, and the two are interleaved at the scale of a coal.
//! Pirk et al. 2017 (*Interactive wood combustion for botanical tree models*, ACM TOG)
//! model exactly that split — virgin wood is decomposed into volatiles and **char**, the char layer
//! insulates in proportion to its thickness, and pyrolysis runs "smoothly from zero to one for
//! temperatures between $T_0 = 150\,°C$ and $T_1 = 450\,°C$" — so the fraction is a *field* over the
//! surface and never a constant.
//!
//! [`char_field`] is that field as value noise: plain `f32` maths, no engine, so a host can generate a
//! bed's texture for an [`Image`](https://docs.rs/bevy_image) at startup, and the field is testable
//! without one. [`char_share`] maps it to the share of the bed's own albedo and emissive a texel
//! carries, which is how the field reaches a material: a charred texel both reflects less and emits
//! less, and a glowing one does the opposite.
//!
//! `docs/research/fuel-bed.md` records the defect this exists to fix, its measurements, and the
//! example-side numbers a caller passes to [`char_share`].

/// The octaves in a char field: three, so a bed reads as coals rather than as a wash.
const OCTAVES: u32 = 3;

/// The first octave's frequency: four lumps across the bed, which on a 0.34 m coal bed is a lump every
/// 8.5 cm — a coal's own width.
const BASE_FREQUENCY: f32 = 4.0;

/// The frequency ratio between octaves.
const LACUNARITY: f32 = 1.6;

/// One lattice point's value in `0..=1`: an integer hash of the point, the same construction the flame
/// shader uses (`src/flame.wesl`), so a bed and the fire standing on it are the same kind of noise.
fn lattice(x: i32, y: i32, seed: u32) -> f32 {
    let mut v = (x as u32)
        .wrapping_mul(0x9e37_79b9)
        .wrapping_add((y as u32).wrapping_mul(0x85eb_ca6b))
        .wrapping_add(seed);
    v ^= v >> 16;
    v = v.wrapping_mul(0x7feb_352d);
    v ^= v >> 15;
    v = v.wrapping_mul(0x846c_a68b);
    v ^= v >> 16;
    (v & 0xffff) as f32 / 65535.0
}

/// Value noise at `(x, y)`: bilinear over the four lattice points around it, with a smoothstep fade so
/// its gradient is continuous and the octaves do not show their grid.
fn value_noise(x: f32, y: f32, seed: u32) -> f32 {
    let floor = x.floor();
    let fraction = Vec2Like { x: x - floor, y: y - y.floor() };
    let (ix, iy) = (floor as i32, y.floor() as i32);
    let fade = |t: f32| t * t * (3.0 - 2.0 * t);
    let (fx, fy) = (fade(fraction.x), fade(fraction.y));
    let corners = [
        lattice(ix, iy, seed),
        lattice(ix + 1, iy, seed),
        lattice(ix, iy + 1, seed),
        lattice(ix + 1, iy + 1, seed),
    ];
    let top = corners[0] + (corners[1] - corners[0]) * fx;
    let bottom = corners[2] + (corners[3] - corners[2]) * fx;
    top + (bottom - top) * fy
}

/// A tiny two-field struct, so this module stays free of `bevy`'s `Vec2` (see `tests/leaf.rs`).
struct Vec2Like {
    x: f32,
    y: f32,
}

/// The char field of a fuel bed at texture coordinates `(u, v)`, both in `0..=1`: `0` is fully burnt
/// through, `1` is the hottest glowing fuel, and the mean is about `0.5`.
///
/// Three octaves of value noise, seeded, so the field is reproducible from a seed and a re-run of the
/// same scene draws the same coals. `u` and `v` outside `0..=1` are not clamped: the field is
/// generated over the texture's own domain, and a caller that wants it to tile gets that from its
/// sampling, not from here.
///
/// ```
/// # use bevy_lights::model::char::*;
/// // The whole field stays in range, whatever it is asked for.
/// for i in 0..=20 {
///     for j in 0..=20 {
///         let f = char_field(i as f32 / 20.0, j as f32 / 20.0, 7);
///         assert!((0.0..=1.0).contains(&f), "field {f} out of range");
///     }
/// }
/// // Seeded: two seeds are two different beds.
/// assert_ne!(char_field(0.3, 0.7, 1), char_field(0.3, 0.7, 2));
/// ```
pub fn char_field(u: f32, v: f32, seed: u32) -> f32 {
    let mut sum = 0.0;
    let mut weight = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = BASE_FREQUENCY;
    for octave in 0..OCTAVES {
        // Each octave gets its own seed, so the octaves are not the same pattern scaled.
        sum += amplitude * value_noise(u * frequency, v * frequency, seed.wrapping_add(octave));
        weight += amplitude;
        amplitude *= 0.5;
        frequency *= LACUNARITY;
    }
    if weight <= 0.0 {
        return 0.5;
    }
    (sum / weight).clamp(0.0, 1.0)
}

/// The share of its own albedo and emissive a texel of a fuel bed carries, from its [`char_field`]
/// sample: `low` for fully charred, `high` for the hottest fuel, linear between them.
///
/// `low` and `high` are the art direction and belong to the host. `docs/research/fuel-bed.md` records
/// the pair the courtyard uses and what each end means physically: `low` is charcoal, which absorbs
/// almost all the light that lands on it and emits little, and `high` is the glowing fuel, which is
/// the brightest thing on the bed. The result is clamped to `0..=1` because it becomes a texel.
///
/// A `low` above `high` is not an error but an inversion — a bed whose glowing part is darker than its
/// char — and is returned as asked, so a caller that wants a cold, mostly-charred bed can have one by
/// passing `low > high`.
///
/// ```
/// # use bevy_lights::model::char::*;
/// assert_eq!(char_share(0.0, 0.25, 1.0), 0.25);
/// assert_eq!(char_share(1.0, 0.25, 1.0), 1.0);
/// assert!((char_share(0.5, 0.25, 1.0) - 0.625).abs() < 1e-6);
/// ```
pub fn char_share(field: f32, low: f32, high: f32) -> f32 {
    let field = if field.is_nan() { 0.5 } else { field.clamp(0.0, 1.0) };
    (low + (high - low) * field).clamp(0.0, 1.0)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    /// A grid of samples over the field, for the statistics below.
    fn samples(seed: u32) -> Vec<f32> {
        let mut out = Vec::new();
        for i in 0..64 {
            for j in 0..64 {
                out.push(char_field(i as f32 / 64.0, j as f32 / 64.0, seed));
            }
        }
        out
    }

    /// The field must spend its time on both sides of the middle: a bed whose field sat at 0.5
    /// everywhere would be the flat emissive this exists to replace, at half the brightness.
    #[test]
    fn a_char_field_has_a_middle_and_both_sides_of_it() {
        for seed in [0, 7, 1234] {
            let values = samples(seed);
            let mean = values.iter().sum::<f32>() / values.len() as f32;
            assert!((0.30..=0.70).contains(&mean), "seed {seed}: mean {mean} is not a middle");
            let low = values.iter().filter(|v| **v < 0.25).count();
            let high = values.iter().filter(|v| **v > 0.75).count();
            assert!(low > values.len() / 20, "seed {seed}: only {low} samples below 0.25");
            assert!(high > values.len() / 20, "seed {seed}: only {high} samples above 0.75");
        }
    }

    /// Structure, not noise per texel: neighbouring samples must be correlated, or the bed reads as
    /// static rather than as coals. A per-texel random field would have a neighbour difference near one
    /// third of the range; this asserts the field is far smoother than that.
    #[test]
    fn neighbouring_samples_are_correlated() {
        let values = samples(7);
        let mut steps = 0.0;
        let mut count = 0.0;
        for i in 0..64 {
            for j in 0..63 {
                steps += (values[i * 64 + j + 1] - values[i * 64 + j]).abs();
                count += 1.0;
            }
        }
        let mean_step = steps / count;
        assert!(mean_step < 0.12, "mean step between neighbours is {mean_step}, too rough to be coals");
        assert!(mean_step > 0.0, "the field is constant: {mean_step}");
    }

    #[test]
    fn the_field_is_reproducible_from_its_seed() {
        assert_eq!(char_field(0.37, 0.81, 7), char_field(0.37, 0.81, 7));
        assert_ne!(char_field(0.37, 0.81, 7), char_field(0.37, 0.81, 8));
    }

    #[test]
    fn a_share_maps_the_field_between_its_two_ends_and_clamps() {
        assert!((char_share(0.25, 0.0, 1.0) - 0.25).abs() < 1e-6);
        // Out-of-range fields and a NaN are clamped rather than escaping into a texture.
        assert_eq!(char_share(-3.0, 0.0, 1.0), 0.0);
        assert_eq!(char_share(3.0, 0.0, 1.0), 1.0);
        assert!((char_share(f32::NAN, 0.0, 1.0) - 0.5).abs() < 1e-6);
        // The share is a texel, so it never leaves 0..=1 however the ends are given.
        assert_eq!(char_share(0.0, 2.0, 3.0), 1.0);
        assert_eq!(char_share(1.0, -2.0, -3.0), 0.0);
    }
}
