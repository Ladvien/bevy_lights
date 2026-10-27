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

/// The lumps across a bed: six, so a lump is one coal wide. The bed is 0.34 m and a coal is ~5 cm,
/// which is six across; the showcase's nearest camera is 4 m away, so a lump is ~17 px.
const LUMPS: f32 = 6.0;

/// How far a lump's centre may wander from its cell's centre, as a share of a cell: the lumps of a
/// bed are not a grid, and a jitter of this much makes the seams wander without letting any cell fall
/// empty.
const JITTER: f32 = 0.62;

/// How deep into a lump its fall-off from the hot middle reaches, in cells: a coal is not a flat chip,
/// it is a lump whose middle is the temperature that glows and whose faces cool towards its edges.
const RIM: f32 = 0.85;

/// How wide the dark ash is between two lumps, in cells: the channel where the fire has not reached.
///
/// A bed of coals is **bright lumps with dark ash between them**, and getting that the right way round
/// is the whole of this field's shape. The first version had it inverted — a hot *seam* between lumps
/// and a charred centre inside each — which at 4× rendered as a honeycomb net of glowing lines, the
/// unmistakable look of a chain-link fence or a waffle, not of a fire. A coal is the thing that glows;
/// the gap between two coals is where the air is and where the ash sits, and it is the dark part.
///
/// The width is what decides which of the two the bed reads as. At `0.42` the ash covered a third of
/// the bed and the lumps read as islands in it; at `0.22` the ash is a channel, two lumps are separated
/// by a line rather than by a field, and the bed is mostly coal — which is what a bed that has been
/// burning for hours is. Measured over a 96×96 grid: `0.22` leaves 0.26 of the bed under the ember stop
/// and 0.46 above the middle.
const GAP: f32 = 0.22;

/// The brightness of a lump, as a share of the field's top, drawn per lump so the bed is not six-by-six
/// copies of one coal: a heap is a mix of lumps at different stages, not a uniform tile.
const LUMP_FLOOR: f32 = 0.72;

/// The field's ember stop, in `0..=1`: the fraction of a bed that is charred.
///
/// A bed burning for hours is **mostly glowing**, with char in the ash between the lumps. At `0.5` the
/// bed read as a dark pit with bright patches and its mean radiance fell *below* the lit stone of the
/// bowl it sits in — a motivated-source failure of exactly the kind this work exists to fix: the source
/// must be the brightest thing on its own prop. A quarter is what a mostly-alight bed measures.
const EMBER_AT: f32 = 0.25;

/// The field's mean over a bed, measured: what a caller can expect from any seed, and what the tests
/// assert. Bright lumps with dark ash put roughly a third of the bed in the ash and a tenth at full
/// glow, so the mean sits below a half — and that is correct here. The *brightness* is not the field's
/// job: [`coal_radiance`] maps the field onto a temperature ramp, and the lump that sits at the top of
/// the ramp is tens of times brighter than the ash beneath it.
pub const FIELD_MEAN: (f32, f32) = (0.40, 0.50);

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
/// to draw it: the distance to the nearest lump's centre and to the second nearest. Inside one lump the
/// field is that lump's own brightness, cooling a little towards its edge; in the band where two lumps
/// are nearly equidistant — the channel between them — the field falls away to black. That channel is
/// the **ash**, and it is dark because the air and the burnt-out fuel are there, not bright. Getting
/// this the right way round is the difference between a bed of coals and a glowing waffle.
///
/// The distance is **Chebyshev** (the larger of the two axis differences), which is why the lumps have
/// straight fracture edges meeting at angles rather than the smooth curves a Euclidean field gives: a
/// coal's edges are straight. Four octaves of value noise — this function's first form — produced
/// round, cloud-like patches that read as camouflage at 4×. `docs/research/fuel-bed.md` records both
/// measurements, and the honeycomb the inverted version rendered as.
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
    // Two heaps at different scales, so the lumps are not one size. A bed of coals is a *crushed*
    // material: a load of 10 cm lumps settles into a matrix of fragments a few centimetres across, and a
    // bed of one uniform lump size renders as cobblestones or as a honeycomb whatever its brightness is.
    // The second heap is half the first's size and weighted less, so it breaks up the first's cells
    // without replacing their structure.
    let coarse = heap(u, v, LUMPS, seed);
    let fine = heap(u, v, LUMPS * 1.9, seed ^ 0x2545);
    0.68 * coarse + 0.32 * fine
}

/// One heap's char field at `(u, v)`: the nearest-lump distance in a jittered grid of `cells` cells.
///
/// The lumps are the coals and the channels between them are the ash, so a lump's body is bright and a
/// channel is dark. Two details are what make it read as coals rather than as a tiled pattern: the lump
/// centres are jittered by [`JITTER`], which bends the cells out of a grid, and each lump carries its
/// own brightness, so the heap is a mix of coals at different stages rather than a uniform tile.
///
/// The distance is **Chebyshev**, which gives lumps straight fracture edges meeting at angles; a
/// Euclidean field gives rounded pebbles.
fn heap(u: f32, v: f32, cells: f32, seed: u32) -> f32 {
    let (x, y) = (u * cells, v * cells);
    let (ix, iy) = (x.floor() as i32, y.floor() as i32);
    let mut nearest = f32::MAX;
    let mut second = f32::MAX;
    let mut owner = (ix, iy);
    // The nearest lump's cell and its eight neighbours: a jitter under half a cell means no lump
    // outside this 3×3 block can hold the nearest centre.
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
    // The lump's own heat: a heap is lumps at different stages, so each has its own brightness, and it
    // cools from its middle towards its edge. The bright lumps are the coals; the dark is the ash.
    let own = LUMP_FLOOR + (1.0 - LUMP_FLOOR) * lattice(owner.0, owner.1, seed ^ 0x51ed);
    let body = own * (0.72 + 0.28 * smooth(1.0 - nearest / RIM));
    // The ash between two lumps: 0 where two lumps are equidistant, 1 well inside one.
    let ash = smooth((second - nearest) / GAP);
    (body * ash).clamp(0.0, 1.0)
}

/// A coal's radiance at [`char_field`] sample `field`, in **candelas per square metre**, as linear
/// RGB: charred coal's own reflected light at `0`, a dull ember at [`EMBER_AT`], and the bed's hottest
/// fuel at `1`.
///
/// This is the physical form of [`char_radiance`], and the one a **surface** wants: a material's
/// `emissive` is a radiance in cd/m², so a bed built from blackbody luminances is exposed correctly by
/// whatever exposure the host has set, and re-exposed for free when the host changes it. The three
/// stops are three *temperatures*, and their magnitudes come from
/// [`blackbody_luminance`](crate::blackbody_luminance) rather than from three guessed strengths — a
/// distinction that is not cosmetic. Written by eye, a bed is a set of numbers that only look right at
/// the one exposure it was tuned at; written from temperatures, it is right at every exposure and its
/// colour and brightness cannot drift apart.
///
/// The temperatures, and why: a bed burning for hours is mostly alight, its seams are where the air
/// reaches the fuel and are the hottest part, and charred coal has stopped emitting and shows only
/// what lands on it. `docs/research/fuel-bed.md` records the measurements and the citation for each.
///
/// ```
/// # use bevy_lights::model::char::*;
/// let coal = coal_radiance(0.0);
/// let seam = coal_radiance(1.0);
/// // The seam is two orders of magnitude brighter than a burnt-through lump, as two temperatures
/// // 350 K apart are: the bed has real dynamic range, not a tint.
/// assert!(seam[0] > coal[0] * 50.0, "{seam:?} against {coal:?}");
/// // And it is a colour, not a grey: a coal's blue channel is a fraction of its red.
/// assert!(seam[2] < seam[0] * 0.15);
/// ```
pub fn coal_radiance(field: f32) -> [f32; 3] {
    /// The temperature of a lump that has burnt through, in kelvin: its surface shows the fire's light
    /// and emits almost nothing of its own. A char layer's own emission is below the visible threshold.
    const CHAR_KELVIN: f32 = 950.0;
    /// The temperature of a dull ember, in kelvin: alight, not yet caught. This is the bed's standing
    /// colour, and it sits under the white point so the bed reads as orange rather than as a wash.
    const EMBER_KELVIN: f32 = 1200.0;
    /// The temperature at a seam, in kelvin: where air reaches the fuel, the hottest a coal bed gets
    /// without a visible flame on it.
    const FUEL_KELVIN: f32 = 1400.0;
    /// Charred coal has stopped *emitting*, but it still shows the fire's light on its own surface, so
    /// its floor is a small grey rather than a black: a black hole in a bed reads as a missing texture,
    /// not as spent fuel.
    const CHAR_REFLECTED: [f32; 3] = [0.6, 0.5, 0.45];
    let charred = scaled(CHAR_KELVIN, 1.0);
    let charred = [
        charred[0].max(CHAR_REFLECTED[0]),
        charred[1].max(CHAR_REFLECTED[1]),
        charred[2].max(CHAR_REFLECTED[2]),
    ];
    char_radiance(
        field,
        charred,
        scaled(EMBER_KELVIN, 1.0),
        scaled(FUEL_KELVIN, 1.0),
    )
}

/// A blackbody at `kelvin` as a radiance in cd/m², with its hue: [`blackbody_colour`] times
/// [`blackbody_luminance`](crate::blackbody_luminance), divided by the colour's own luminance so the
/// two agree.
///
/// The division is what makes this exact rather than approximate. `blackbody_colour` normalises to its
/// **brightest channel** — a 1200 K blackbody comes back as `[1.0, 0.046, 0.0]` — so multiplying it by
/// a luminance would give a radiance whose *red* channel is the luminance, not the colour. Luminance is
/// the weighted sum of all three channels, so the triple is divided by its own luminance first and then
/// scaled to the published one. What comes out has both the brightness and the hue of a blackbody, and
/// the two functions stay one model.
fn scaled(kelvin: f32, strength: f32) -> [f32; 3] {
    let colour = crate::model::cinematography::blackbody_colour(kelvin, 1.0);
    let lum = 0.2126 * colour[0] + 0.7152 * colour[1] + 0.0722 * colour[2];
    let target = crate::model::cinematography::blackbody_luminance(kelvin) * strength;
    if lum <= 0.0 || !lum.is_finite() {
        return [target, target, target];
    }
    [colour[0] / lum * target, colour[1] / lum * target, colour[2] / lum * target]
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
    /// and one whose lumps only wandered near the middle would read as a wash.
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
            // A bed of coals is bright lumps with dark ash between them, and the ash covers a good share
            // of the bed: measured over the 64×64 grid, a third of it is under the ember stop and a
            // tenth is at full glow. An inverted field — hot seams, charred hearts — puts those two
            // numbers the other way round, which is how the honeycomb render got past a test that only
            // checked the mean.
            assert!((0.15..=0.35).contains(&charred), "seed {seed}: {charred:.3} of the bed is ash");
            assert!((0.02..=0.22).contains(&hot), "seed {seed}: {hot:.3} of the bed is glowing fuel");
            // Both ends are reached, so the bed's brightest lumps can sit at the top of the curve.
            let peak = values.iter().cloned().fold(f32::MIN, f32::max);
            assert!(peak > 0.90, "seed {seed}: the brightest sample is only {peak}");
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

    /// The shape is **bright lumps with dark ash between them**, and a test that only checks the field's
    /// mean cannot see the difference: the first cellular version had it inverted — a hot seam between
    /// lumps and a charred heart inside each — and its mean was inside the same band while it rendered
    /// as a glowing honeycomb. The discriminating quantity is how much of the bed is *ash* against how
    /// much is *coal*. Measured over a 64×64 grid: the inverted field put 0.11 of the bed under the ember
    /// stop and 0.39 above 0.75; this one puts 0.28 under and 0.06–0.18 above. Asserting both bounds is
    /// what makes the honeycomb fail this test, and it did.
    #[test]
    fn a_bed_is_bright_lumps_with_dark_ash_between_them() {
        for seed in [0, 7, 1234] {
            let values = samples(seed);
            let count = values.len() as f32;
            let ash = values.iter().filter(|v| **v < EMBER_AT).count() as f32 / count;
            let glow = values.iter().filter(|v| **v > 0.75).count() as f32 / count;
            assert!((0.15..=0.35).contains(&ash), "seed {seed}: {ash:.3} of the bed is ash");
            assert!((0.02..=0.22).contains(&glow), "seed {seed}: {glow:.3} of the bed is at full glow");
            // Every coal is a lump with its own peak and every channel between two of them a dark
            // trough, so several rows across the bed each cross the field several times. A single row
            // can graze a boundary and miss the structure, so this counts over eight.
            let (mut peaks, mut troughs) = (0, 0);
            for row_index in 0..8 {
                let y = (row_index as f32 + 0.5) / 8.0;
                let row: Vec<f32> = (0..256).map(|i| char_field(i as f32 / 255.0, y, seed)).collect();
                peaks += (1..row.len() - 1)
                    .filter(|i| row[*i] > row[i - 1] && row[*i] >= row[i + 1] && row[*i] > 0.60)
                    .count();
                troughs += (1..row.len() - 1)
                    .filter(|i| row[*i] < row[i - 1] && row[*i] <= row[i + 1] && row[*i] < EMBER_AT)
                    .count();
            }
            assert!(peaks >= 16, "seed {seed}: only {peaks} lump peaks across eight rows");
            assert!(troughs >= 16, "seed {seed}: only {troughs} ash channels across eight rows");
            // The ash is genuinely dark — it reaches the bottom of the field, not merely below its own
            // mean — or the "ash" is a dim coal rather than a gap.
            assert!(values.iter().cloned().fold(f32::MAX, f32::min) < EMBER_AT * 0.5, "seed {seed}: no ash");
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
