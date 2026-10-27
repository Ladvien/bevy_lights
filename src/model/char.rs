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
//! without one. [`char_radiance`] maps it to the **colour** a texel of the bed emits, which is how the
//! field reaches a material — and why the mapping returns a colour rather than a share: a coal's three
//! states are three colours. Charred coal has stopped emitting and shows only what lands on it, so it
//! is grey; a blackbody's blue channel is zero at a fire's temperature, so any scalar scaling of one
//! fire colour can never make it. `docs/research/fuel-bed.md` has the measurements.
//!
//! `docs/research/fuel-bed.md` records the defect this exists to fix, its measurements, and the
//! example-side numbers a caller passes to [`char_radiance`].

/// The octaves in a char field: four, so a bed reads as coals rather than as a wash: three octaves
/// left lumps a coal-and-a-half wide, which read as patches of colour instead of as coals.
const OCTAVES: u32 = 4;

/// The first octave's frequency: eight lumps across the bed, which on a 0.34 m coal bed is a lump every
/// 4.2 cm — half a coal, so the bed reads as a *bed of coals* rather than as a few patches of colour.
const BASE_FREQUENCY: f32 = 8.0;

/// The frequency ratio between octaves.
const LACUNARITY: f32 = 1.6;

/// The raw noise's range, stretched to the field's full `0..=1`, as the pair of values that map to `0`
/// and `1`.
///
/// Four octaves of value noise average towards the middle — measured over a 64×64 grid, the raw field
/// has a mean near `0.5` and a standard deviation of only `0.135`, so a linear map to a share gives a
/// bed that reads as a smooth wash rather than as coals. A smoothstep between these two points is what
/// turns the same noise into a patchwork: it is the field's own ±std, so the split is where the noise
/// actually puts its mass and not an eyeballed threshold. `docs/research/fuel-bed.md` records the
/// measurements either side of it.
const STRETCH: (f32, f32) = (0.34, 0.66);

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
/// Four octaves of value noise, seeded, so the field is reproducible from a seed and a re-run of the
/// same scene draws the same coals. The octaves' average is then stretched by [`STRETCH`], because
/// noise alone has too little spread to read as a bed of coals: it is a *field with two states*,
/// glowing and charred, not a smooth gradient. `u` and `v` outside `0..=1` are not clamped: the field
/// is generated over the texture's own domain, and a caller that wants it to tile gets that from its
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
    let raw = (sum / weight).clamp(0.0, 1.0);
    let (low, high) = STRETCH;
    let t = ((raw - low) / (high - low)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The radiance of a coal at [`char_field`] sample `field`, as linear RGB: `char` at `0`, the coldest
/// dull ember at `0.5`, and `fuel` at `1`.
///
/// A coal's three states are three colours, not one colour at three brightnesses, and that is why this
/// returns a colour. A blackbody's blue channel is **zero** below about 2000 K, so scaling one fire
/// colour by a field produces a crimson bed with no way to be anything else. Charred coal is not a dim
/// red: it has stopped emitting and shows only the light that lands on it, which is grey. So:
///
/// - `char` is the host's charred colour — ash-grey, its diffuse answer to the fire above it;
/// - the middle is a dull ember, the cool red of fuel that is alight but has not caught;
/// - `fuel` is the bed's hottest glow, which the host scales past the display's white.
///
/// The middle is interpolated as a *ramp* (`smoothstep` on either side of `0.5`), so the bed reads as
/// patches with a hot core and a charred rim rather than as a wash. `field` is clamped, and a `NaN`
/// reads as a half-burnt coal.
///
/// ```
/// # use bevy_lights::model::char::*;
/// let char = [0.02, 0.018, 0.016];
/// let fuel = [4.0, 0.7, 0.0];
/// let ember = [0.25, 0.02, 0.0];
/// assert_eq!(char_radiance(0.0, char, ember, fuel), char);
/// assert_eq!(char_radiance(1.0, char, ember, fuel), fuel);
/// // The middle is the dull ember, exactly.
/// assert_eq!(char_radiance(0.5, char, ember, fuel), ember);
/// ```
pub fn char_radiance(field: f32, char: [f32; 3], ember: [f32; 3], fuel: [f32; 3]) -> [f32; 3] {
    let field = if field.is_nan() { 0.5 } else { field.clamp(0.0, 1.0) };
    let (from, to, t) = if field < 0.5 {
        (char, ember, field * 2.0)
    } else {
        (ember, fuel, (field - 0.5) * 2.0)
    };
    // Smoothstep, so neither stop shows as a crease where the two halves of the ramp meet.
    let t = t * t * (3.0 - 2.0 * t);
    [0, 1, 2].map(|i| from[i] + (to[i] - from[i]) * t)
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

    /// The field must spend its time on both sides of the middle, and *at* the ends: a bed whose field
    /// sat at 0.5 everywhere would be the flat emissive this exists to replace, at half the brightness,
    /// and one whose noise only wandered near the middle would read as a wash. Measured over the 64×64
    /// grid, a stretch that is too weak puts under a tenth of the bed at either end; this one puts
    /// about a quarter.
    #[test]
    fn a_char_field_has_a_middle_and_both_ends_of_it() {
        for seed in [0, 7, 1234] {
            let values = samples(seed);
            let count = values.len() as f32;
            let mean = values.iter().sum::<f32>() / count;
            assert!((0.30..=0.70).contains(&mean), "seed {seed}: mean {mean} is not a middle");
            let low = values.iter().filter(|v| **v < 0.25).count() as f32 / count;
            let high = values.iter().filter(|v| **v > 0.75).count() as f32 / count;
            assert!(low > 0.15, "seed {seed}: only {low:.3} of the bed is charred");
            assert!(high > 0.10, "seed {seed}: only {high:.3} of the bed is glowing fuel");
            // Both ends are reached, so the bed's brightest coals can sit at the top of the curve.
            assert_eq!(values.iter().cloned().fold(f32::MIN, f32::max), 1.0, "seed {seed}: no coal at full glow");
            assert_eq!(values.iter().cloned().fold(f32::MAX, f32::min), 0.0, "seed {seed}: no burnt-through coal");
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

    /// The three stops, and the two properties that make the ramp a ramp rather than a lerp with a
    /// crease in it: the ends are the ends exactly, and the middle is reached from both sides.
    #[test]
    fn a_radiance_ramp_passes_through_its_three_stops() {
        let char = [0.1, 0.05, 0.02];
        let ember = [2.2, 0.32, 0.05];
        let fuel = [9.0, 1.7, 0.25];
        assert_eq!(char_radiance(0.0, char, ember, fuel), char);
        assert_eq!(char_radiance(1.0, char, ember, fuel), fuel);
        assert_eq!(char_radiance(0.5, char, ember, fuel), ember);
        // Out-of-range fields and a NaN are clamped rather than escaping into a texture.
        assert_eq!(char_radiance(-3.0, char, ember, fuel), char);
        assert_eq!(char_radiance(3.0, char, ember, fuel), fuel);
        assert_eq!(char_radiance(f32::NAN, char, ember, fuel), ember);
        // Monotone in every channel between char and fuel, so the ramp never reverses: a hotter coal
        // is never dimmer than a cooler one.
        for i in 0..=100 {
            let a = char_radiance(i as f32 / 100.0, char, ember, fuel);
            let b = char_radiance((i + 1) as f32 / 100.0, char, ember, fuel);
            for c in 0..3 {
                assert!(b[c] >= a[c], "channel {c} fell between {i} and {}", i + 1);
            }
        }
    }
}
