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
//! [`char_field`] is that field as a heap of coal lumps: plain `f32` maths, no engine, so a host can
//! generate a bed's texture for an [`Image`](https://docs.rs/bevy_image) at startup, and the field is
//! testable without one. [`char_radiance`] maps it to the **colour** a texel of the bed emits, which is how the
//! field reaches a material — and why the mapping returns a colour rather than a share: a coal's three
//! states are three colours. Charred coal has stopped emitting and shows only what lands on it, so it
//! is grey; a blackbody's blue channel is zero at a fire's temperature, so any scalar scaling of one
//! fire colour can never make it. `docs/research/fuel-bed.md` has the measurements.
//!
//! `docs/research/fuel-bed.md` records the defect this exists to fix, its measurements, and the
//! example-side numbers a caller passes to [`char_radiance`].

/// Where the ember stop sits on the field, in `0..=1`: the fraction of a bed that is charred.
///
/// A bed of coals that has been burning for hours is **mostly glowing**, with char in the cracks
/// between the lumps — not half and half. At `0.5` the bed read as a dark pit with bright patches, and
/// its mean radiance fell *below* the lit stone of the bowl it sits in, which is a motivated-source
/// failure of exactly the kind this work exists to fix: the source must be the brightest thing on its
/// own prop. A quarter is what a mostly-alight bed measures.
const EMBER_AT: f32 = 0.25;

/// The field's mean over a bed, measured: what a caller can expect from any seed, and what the tests
/// assert. A heap of lumps puts most of its area on a rim and a seam, so the mean sits well above the
/// half a noise field would give, and a bed that flickers between glowing coal and char is what makes
/// it read as coals at all.
pub const FIELD_MEAN: (f32, f32) = (0.55, 0.75);

/// The lumps across a bed: six, so a lump is one coal wide. The bed is 0.34 m and a coal is ~5 cm,
/// which is six across; the showcase's nearest camera is 4 m away, so a lump is ~17 px.
const LUMPS: f32 = 6.0;

/// How far a lump's centre may wander from its cell's centre, as a share of a cell: the lumps of a
/// bed are not a grid, and a jitter of this much makes the seams wander without letting any cell fall
/// empty.
const JITTER: f32 = 0.45;

/// How wide a seam is between two lumps, in cells: the glowing crack where air reaches the fuel.
///
/// A coal's brightest edge is where the two nearest lumps are equidistant — the seam between them is
/// a *ridge*, not a random blob. That is why this field is cellular rather than value noise: the
/// previous version's four octaves of noise produced round, cloud-like patches that read as
/// camouflage at 4×, and a real coal bed's lumps have **straight fracture edges** with hot seams
/// between them. This is the note's recorded gap (`docs/research/fuel-bed.md`, `## Pending index`).
const SEAM: f32 = 0.36;

/// How deep into a lump its hot rim reaches, in cells: the face of a coal that the fire is on, against
/// the charred core behind it.
const RIM: f32 = 0.50;

/// The share of a lump's rim that is at the field's top and its charred centre, relative to `EMBER_AT`.
/// A lump is not one temperature: its rim is where the air is and its centre is spent, so the field
/// gives the rim 1.0 and the centre this. Read against [`EMBER_AT`] it makes a lump a *ring* of heat
/// with a dark heart, which is what a coal looks like from above.
const LUMP_RIM: f32 = 0.95;
const LUMP_CENTRE: f32 = 0.10;
/// The spread of a lump's own brightness, so all six-by-six lumps are not the same coal.
const LUMP_SPREAD: f32 = 0.20;

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

/// A smoothstep on `0..=1`, clamped: the field's only easing curve.
fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Where lump `(cx, cy)`'s centre sits, in cell units: its cell's centre plus a seeded jitter.
fn lump_centre(cx: i32, cy: i32, seed: u32) -> (f32, f32) {
    let dx = (lattice(cx, cy, seed) - 0.5) * 2.0 * JITTER;
    let dy = (lattice(cx, cy, seed ^ 0x9e37) - 0.5) * 2.0 * JITTER;
    (cx as f32 + 0.5 + dx, cy as f32 + 0.5 + dy)
}

/// The char field of a fuel bed at texture coordinates `(u, v)`, both in `0..=1`: `0` is fully burnt
/// through, `1` is the hottest glowing fuel.
///
/// A bed of coals is **a heap of lumps**, not a field of noise. Two nearest-lump distances are enough
/// to draw it: the distance to the nearest lump's centre (`f1`) and to the second (`f2`). Their
/// difference is small exactly on the boundary between two lumps, which is where a crack runs, and air
/// reaches the fuel along it, so the seam is the hottest part of the bed. Inside one lump the field
/// falls from its rim to a charred core, so a lump reads as a ring of heat rather than a dot.
///
/// The distance is **Chebyshev** (the larger of the two axis differences), which is why the seams are
/// straight lines meeting at angles rather than the smooth curves a Euclidean field gives: a coal's
/// fracture edges are straight, and four octaves of value noise — this function's previous form —
/// produced round, cloud-like patches that read as camouflage at 4×. `docs/research/fuel-bed.md`
/// records both measurements.
///
/// Seeded, so the field is reproducible from a seed and a re-run of the same scene draws the same
/// coals. `u` and `v` outside `0..=1` are not clamped: the field is generated over the texture's own
/// domain, and a caller that wants it to tile gets that from its sampling, not from here.
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
    let (x, y) = (u * LUMPS, v * LUMPS);
    let (ix, iy) = (x.floor() as i32, y.floor() as i32);
    let mut nearest = f32::MAX;
    let mut second = f32::MAX;
    let mut owner = (ix, iy);
    // The cell of the nearest lump and its eight neighbours: a jitter under half a cell means no
    // lump outside this 3×3 block can hold the nearest centre.
    for dy in -1..=1 {
        for dx in -1..=1 {
            let (cx, cy) = (ix + dx, iy + dy);
            let (px, py) = lump_centre(cx, cy, seed);
            let d = (x - px).abs().max((y - py).abs());
            if d < nearest {
                second = nearest;
                nearest = d;
                owner = (cx, cy);
            } else if d < second {
                second = d;
            }
        }
    }
    // The seam: 0 well inside a lump, 1 where the two nearest centres are equidistant. Narrow it to
    // `SEAM` so the hot crack is a crack and not a wash.
    let seam = 1.0 - smooth((second - nearest) / SEAM);
    // The lump's own heat: its rim is at the top of the field and its core is charred. `LUMP_SPREAD`
    // gives each lump its own brightness, so the bed is not six-by-six copies of one coal.
    let own = (lattice(owner.0, owner.1, seed ^ 0x51ed) - 0.5) * LUMP_SPREAD;
    let core = (LUMP_CENTRE + own).clamp(0.0, 1.0);
    let rim = (LUMP_RIM + own).clamp(0.0, 1.0);
    let lump = core + (rim - core) * smooth(1.0 - nearest / RIM);
    seam.max(lump).clamp(0.0, 1.0)
}

/// The radiance of a coal at [`char_field`] sample `field`, as linear RGB: `char` at `0`, the dull
/// ember at [`EMBER_AT`], and `fuel` at `1`.
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
/// The ramp is a *smoothstep on either side of [`EMBER_AT`]*, so the bed reads as patches with a hot
/// core and a charred rim rather than as a wash. `field` is clamped, and a `NaN` reads as the ember.
///
/// ```
/// # use bevy_lights::model::char::*;
/// let char = [0.02, 0.018, 0.016];
/// let fuel = [4.0, 0.7, 0.0];
/// let ember = [0.25, 0.02, 0.0];
/// assert_eq!(char_radiance(0.0, char, ember, fuel), char);
/// assert_eq!(char_radiance(1.0, char, ember, fuel), fuel);
/// // The ember stop, exactly.
/// assert_eq!(char_radiance(EMBER_AT, char, ember, fuel), ember);
/// ```
pub fn char_radiance(field: f32, char: [f32; 3], ember: [f32; 3], fuel: [f32; 3]) -> [f32; 3] {
    let field = if field.is_nan() { EMBER_AT } else { field.clamp(0.0, 1.0) };
    let (from, to, t) = if field < EMBER_AT {
        (char, ember, field / EMBER_AT)
    } else {
        let t = (field - EMBER_AT) / (1.0 - EMBER_AT);
        // The fuel's own stop is reached only by the hottest few percent: a bed whose whole surface
        // sits at the top of the curve is a blown slab again, whatever its texture. A cubic holds the
        // bed at its ember colour and saves white-hot for the coals that really are.
        (ember, fuel, t * t * t)
    };
    // Smoothstep, so no stop shows as a crease where the two halves of the ramp meet.
    let t = smooth(t);
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
    /// and one whose lumps only wandered near the middle would read as a wash. A heap of coals puts far
    /// more of its area near the top than a noise field does — a lump's whole rim and every seam are at
    /// full heat — so the mean is well above a half and the charred share is the minority.
    #[test]
    fn a_char_field_has_a_middle_and_both_ends_of_it() {
        for seed in [0, 7, 1234] {
            let values = samples(seed);
            let count = values.len() as f32;
            let mean = values.iter().sum::<f32>() / count;
            assert!(
                (FIELD_MEAN.0..=FIELD_MEAN.1).contains(&mean),
                "seed {seed}: mean {mean} is outside the documented {FIELD_MEAN:?}"
            );
            let charred = values.iter().filter(|v| **v < EMBER_AT).count() as f32 / count;
            let hot = values.iter().filter(|v| **v > 0.75).count() as f32 / count;
            assert!((0.04..=0.22).contains(&charred), "seed {seed}: {charred:.3} of the bed is charred");
            assert!(hot > 0.20, "seed {seed}: only {hot:.3} of the bed is glowing fuel");
            // Both ends are reached, so the bed's brightest coals can sit at the top of the curve. The
            // top is a *seam*, a measure-zero ridge, so a 64×64 grid lands next to it rather than on it:
            // the assertion is that the field gets within a pixel of full glow, not exactly there.
            let peak = values.iter().cloned().fold(f32::MIN, f32::max);
            assert!(peak > 0.98, "seed {seed}: the brightest sample is only {peak}");
            assert!(
                values.iter().cloned().fold(f32::MAX, f32::min) < EMBER_AT,
                "seed {seed}: no burnt-through coal"
            );
        }
    }

    /// Structure, not noise per texel: neighbouring samples must be mostly correlated, or the bed reads
    /// as static rather than as coals. A per-texel random field would have a neighbour difference of about
    /// a third of the range. The field is *allowed* a sharp step — a seam is a crease, and the whole point
    /// of a lump is that its edge is straight — so this bounds the mean step rather than the maximum.
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
        assert!(mean_step < 0.20, "mean step between neighbours is {mean_step}, too rough to be coals");
        assert!(mean_step > 0.02, "the field is constant across neighbours: {mean_step}");
    }

    #[test]
    fn the_field_is_reproducible_from_its_seed() {
        assert_eq!(char_field(0.37, 0.81, 7), char_field(0.37, 0.81, 7));
        assert_ne!(char_field(0.37, 0.81, 7), char_field(0.37, 0.81, 8));
    }

    /// The lumps are *lumps*: along a line across the bed the field has several separate peaks — a lump's
    /// rim is one, and a seam between two lumps is another — with charred troughs between them. A wash,
    /// or a single large blob, has one peak; the previous four-octave noise field read as camouflage at
    /// 4× precisely because its features had no separate edges to be peaks.
    #[test]
    fn the_field_has_lumps_with_hot_seams_and_charred_hearts() {
        for seed in [0, 7, 1234] {
            // A horizontal line across the bed, at a row well inside it.
            let row: Vec<f32> = (0..256).map(|i| char_field(i as f32 / 255.0, 0.37, seed)).collect();
            let peaks = (1..row.len() - 1)
                .filter(|i| row[*i] > row[i - 1] && row[*i] >= row[i + 1] && row[*i] > 0.60)
                .count();
            assert!(peaks >= 4, "seed {seed}: only {peaks} hot peaks along a row, so not a heap of lumps");
            // A seam reaches the top of the field and a coal's heart falls well under it, in the same row.
            assert!(row.iter().cloned().fold(f32::MIN, f32::max) > 0.90, "seed {seed}: no hot seam");
            assert!(row.iter().cloned().fold(f32::MAX, f32::min) < 0.25, "seed {seed}: no charred heart");
        }
    }

    /// The stops, and the two properties that make the ramp a ramp rather than a lerp with a crease in
    /// it: the ends are the ends exactly, and the middle is reached from both sides.
    #[test]
    fn a_radiance_ramp_passes_through_its_three_stops() {
        let char = [0.1, 0.05, 0.02];
        let ember = [2.2, 0.32, 0.05];
        let fuel = [9.0, 1.7, 0.25];
        assert_eq!(char_radiance(0.0, char, ember, fuel), char);
        assert_eq!(char_radiance(1.0, char, ember, fuel), fuel);
        assert_eq!(char_radiance(EMBER_AT, char, ember, fuel), ember);
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
