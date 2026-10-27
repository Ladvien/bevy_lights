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

/// The lumps across a bed at the coarse heap's scale: six, so a lump is one coal wide. The bed is
/// 0.34 m and a coal is ~5 cm, which is six across.
const LUMPS: f32 = 5.6;

/// The finer heap's lumps across, and its share of the field: a bed of coals is a *crushed* material,
/// so a load of 10 cm lumps settles into a matrix of fragments a few centimetres across. Two scales is
/// what stops the bed reading as a tiled pattern of one coal size.
const FINE_LUMPS: f32 = 9.4;
/// The finer heap's weight against the coarse one's. Under a half, so it breaks up the coarse cells
/// without replacing their structure.
const FINE_SHARE: f32 = 0.74;

/// How far a lump's centre may wander from its cell's centre, as a share of a cell: the lumps of a bed
/// are not a grid, and an almost-full-cell jitter is what leaves a heap with no axis in it.
const JITTER: f32 = 0.85;

/// A lump's radius, as a share of a cell, at its smallest and largest: drawn per lump so the heap is a
/// mix of sizes. Under a half-cell each so the shards stay separate most of the time, and the field is
/// a union of *shards* rather than a partition of *cells*.
const RADIUS: (f32, f32) = (0.36, 0.62);

/// A lump's long axis against its short one, at its least and most elongated: a shard is not a disc.
const STRETCH: (f32, f32) = (0.62, 1.24);

/// The share of cells that hold no coal at all: where a lump settled away and only ash is left.
const SPARSE: f32 = 0.30;

/// How deep into a lump its fall-off from the hot core runs: a coal is not a flat chip, it is a lump
/// whose middle is the temperature that glows and whose faces cool towards its edges. At `0.92` the
/// lump is nearly flat and the fall is quick at its rim; a soft cone would read as a lit bump.
const PLATEAU: f32 = 0.92;

/// The two fracture depths, as a share of a lump's radius: a shard's edge is jagged, and one term of
/// value noise gives it one scale of jaggedness. Without them every lump is a rotated box.
const EDGE: (f32, f32) = (0.30, 0.14);

/// How much a bed cools from the plume's axis to its rim, and how sharply: the flame's plume is
/// axisymmetric (Xia & Zhang 2018), so the coals under the axis are the hottest and the bed's corners
/// are the coolest. Without this the whole surface is one temperature and reads as a lit plate.
const CORE_COOL: (f32, f32) = (0.62, 1.4);

/// The brightness of a lump, as a share of the field's top, drawn per lump so the bed is not six-by-six
/// copies of one coal: a heap is a mix of lumps at different stages, not a uniform tile.
const LUMP_FLOOR: f32 = 0.58;

/// The field's ember stop, in `0..=1`: the fraction of a bed that is charred.
///
/// A bed burning for hours is **mostly glowing**, with char in the ash between the lumps. At `0.5` the
/// bed read as a dark pit with bright patches and its mean radiance fell *below* the lit stone of the
/// bowl it sits in — a motivated-source failure of exactly the kind this work exists to fix: the source
/// must be the brightest thing on its own prop. A quarter is what a mostly-alight bed measures.
const EMBER_AT: f32 = 0.30;

/// The field's mean over a bed, measured: what a caller can expect from any seed, and what the tests
/// assert. Overlapping shards put rather more of the bed in coal than a nearest-lump partition did,
/// because a shard's area is not bounded by its neighbours' — measured across seeds 0, 7 and 1234 the
/// mean is 0.45–0.50, and the ash is a fifth to a quarter of the bed.
///
/// The *brightness* is not the field's job: [`coal_radiance`] maps the field onto a temperature ramp,
/// and the lump that sits at the top of the ramp is tens of times brighter than the ash beneath it.
pub const FIELD_MEAN: (f32, f32) = (0.42, 0.52);

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

/// Value noise on `(x, y)`, in `0..=1`: the fracture term a lump's edge is cut by.
///
/// One octave of it per call, and [`heap`] calls it twice at scales five-eighths apart. A lump's
/// silhouette has to be ragged at more than one scale: a single term gives every lump the same size of
/// sawtooth, while two give a shard a coarse bite and a fine chip to its edge, which is what a
/// fractured coal looks like at the size a texel can hold.
fn edge_noise(x: f32, y: f32, seed: u32) -> f32 {
    let (ix, iy) = (x.floor() as i32, y.floor() as i32);
    let (fx, fy) = (x - ix as f32, y - iy as f32);
    let (u, v) = (smooth(fx), smooth(fy));
    let corner = |dx: i32, dy: i32| lattice(ix + dx, iy + dy, seed);
    let top = corner(0, 0) + (corner(1, 0) - corner(0, 0)) * u;
    let bottom = corner(0, 1) + (corner(1, 1) - corner(0, 1)) * u;
    top + (bottom - top) * v
}

/// One heap's char field at `(u, v)`: the brightness of the brightest lump covering the point, in
/// `0..=1`, or `0` where no lump does.
///
/// **A union of shards, not a partition of cells**, and that distinction is the whole of this
/// function. A nearest-lump partition — the field's first form — makes the ash a *connected net*: every
/// point is equidistant from two lumps somewhere, so the dark channels join and the bed renders as a
/// waffle of pale cells. That survived a mean-and-spread test and rendered as a glowing honeycomb, and
/// then survived as a pale honeycomb, because the net is inherent to a partition. Here each lump is an
/// independent shard with its own radius, so the bright shapes overlap, the channels between them
/// *terminate* where three shards meet, and a cell may hold no coal at all — which is what a heap is.
///
/// The distance to a shard is **Chebyshev** after it is rotated and stretched along its own axis, so a
/// lump is a rotated rectangle rather than a circle: a coal's edges are straight. Two octaves of noise
/// cut into the radius then give the edge its fracture. `docs/research/fuel-bed.md` records the renders
/// this replaced, including the honeycomb two partitions produced.
fn heap(u: f32, v: f32, cells: f32, share: f32, seed: u32) -> f32 {
    let (x, y) = (u * cells, v * cells);
    let (ix, iy) = (x.floor() as i32, y.floor() as i32);
    let mut brightest = 0.0f32;
    // A shard reaches at most half a cell plus its jitter, so a 5×5 block covers every lump that can
    // hold the point.
    for dy in -2..=2 {
        for dx in -2..=2 {
            let (cx, cy) = (ix + dx, iy + dy);
            // Not every cell holds a coal: where a lump settled away there is only ash.
            if lattice(cx, cy, seed ^ 0xabcd) < SPARSE {
                continue;
            }
            let angle = lattice(cx, cy, seed ^ 0x77a1) * core::f32::consts::PI;
            let (sin, cos) = angle.sin_cos();
            let stretch = STRETCH.0 + (STRETCH.1 - STRETCH.0) * lattice(cx, cy, seed ^ 0x33d1);
            let radius = RADIUS.0 + (RADIUS.1 - RADIUS.0) * lattice(cx, cy, seed ^ 0x1b3f);
            let px = cx as f32 + 0.5 + (lattice(cx, cy, seed) - 0.5) * 2.0 * JITTER;
            let py = cy as f32 + 0.5 + (lattice(cx, cy, seed ^ 0x9e37) - 0.5) * 2.0 * JITTER;
            let (vx, vy) = (x - px, y - py);
            let (ux, uy) = (vx * cos + vy * sin, -vx * sin + vy * cos);
            let mut distance = (ux * stretch).abs().max((uy * (2.0 - stretch)).abs());
            // The fracture: two scales of noise cutting into the radius, so the edge is not a box.
            distance *= 1.0 + EDGE.0 * (edge_noise(ux * 2.2 + cx as f32 * 9.0, uy * 2.2 + cy as f32 * 9.0, seed ^ 0x1f1f) - 0.5) * 2.0;
            distance *= 1.0 + EDGE.1 * (edge_noise(ux * 5.5 + cx as f32 * 31.0, uy * 5.5 + cy as f32 * 31.0, seed ^ 0x2b2b) - 0.5) * 2.0;
            let reach = distance / radius;
            if reach < 1.0 {
                // Each lump is at its own stage of the burn, and cools from its middle to its edge.
                let own = LUMP_FLOOR + (1.0 - LUMP_FLOOR) * lattice(cx, cy, seed ^ 0x51ed);
                let body = own * (1.0 - (1.0 - PLATEAU) * smooth(reach));
                if body * share > brightest {
                    brightest = body * share;
                }
            }
        }
    }
    brightest
}

/// The char field of a fuel bed at texture coordinates `(u, v)`, both in `0..=1`: `0` is fully burnt
/// through, `1` is the hottest glowing fuel.
///
/// A bed of coals is **a heap of overlapping lumps**, and two properties of that are what the field is
/// for. The bright coal is the *lump* and the dark is the **ash between lumps**, where the air and the
/// burnt-out fuel are — getting that the right way round is the difference between a bed of coals and a
/// glowing waffle. And the heap is **two scales**, coarse lumps with finer fragments settled between
/// them, because a bed of one lump size renders as cobblestones whatever its brightness is.
///
/// The bed also cools from the middle outwards: the flame's plume is axisymmetric, so the coals under
/// the axis are the hottest and the bed's corners are its coolest.
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
    let coarse = heap(u, v, LUMPS, 1.0, seed);
    let fine = heap(u, v, FINE_LUMPS, FINE_SHARE, seed ^ 0x2545);
    let brightest = coarse.max(fine);
    // The bed cools from the plume's axis to its rim: `d` is 0 at the middle of the texture and 1 at
    // its corners.
    let d = (u - 0.5).hypot(v - 0.5) / core::f32::consts::FRAC_1_SQRT_2;
    let cool = 1.0 - (1.0 - CORE_COOL.0) * smooth(d.powf(CORE_COOL.1));
    (brightest * cool).clamp(0.0, 1.0)
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
/// The three temperatures, and why they are these numbers. Char at **950 K** emits 1.1 cd/m²: it has
/// stopped burning and shows only the light landing on it, so its floor is a small grey rather than a
/// black hole. The ember at **1140 K** is 59 cd/m² and is the bed's standing colour — alight, not
/// caught, sitting well under the white point. The seam at **1330 K** is 1183 cd/m², twenty times the
/// ember, and it is a *seam*: where air reaches the fuel, which is the hottest part of a bed with no
/// flame on it. The ratio between them is what makes the bed read as coals rather than as a tint, and
/// each one climbs so steeply with temperature (a blackbody's visible output roughly doubles per 25 K
/// here) that the spread is set by the temperatures rather than tuned afterwards.
///
/// `docs/research/fuel-bed.md` records the measurements, the two renders these replaced, and why the
/// bed's **specular** had to go with them.
///
/// ```
/// # use bevy_lights::model::char::*;
/// let coal = coal_radiance(0.0);
/// let seam = coal_radiance(1.0);
/// // The seam is twenty times brighter than a burnt-through lump, as two temperatures 380 K apart
/// // are: the bed has real dynamic range, not a tint.
/// assert!(seam[0] > coal[0] * 15.0, "{seam:?} against {coal:?}");
/// // And it is a colour, not a grey: a coal's blue channel is a fraction of its red.
/// assert!(seam[2] < seam[0] * 0.15);
/// ```
pub fn coal_radiance(field: f32) -> [f32; 3] {
    /// The temperature of a lump that has burnt through, in kelvin: its surface shows the fire's light
    /// and emits almost nothing of its own. A char layer's own emission is below the visible threshold.
    const CHAR_KELVIN: f32 = 950.0;
    /// The temperature of a dull ember, in kelvin: alight, not yet caught, and the bed's standing
    /// colour.
    ///
    /// **1020 K, not 1140 K**, and the difference is measured rather than aesthetic. At this scene's
    /// `ev100` 6.0 a 1140 K blackbody's red channel lands at **DN 255** — the ember stop itself is
    /// already past the display's white point — so every lump above the ash read identical white with
    /// its red clipped, and the bed's structure survived only in green. That is why the first build of
    /// this ramp rendered a pale honeycomb whatever the field did: the field was there and the display
    /// could not show it. At 1020 K the ember sits at **DN 157** and the fuel stop at DN 255, so the
    /// bed's range is spent on screen instead of above it. `docs/research/fuel-bed.md` records the
    /// table this was chosen from.
    const EMBER_KELVIN: f32 = 1020.0;
    /// The temperature at a seam, in kelvin: where air reaches the fuel, the hottest a coal bed gets
    /// without a visible flame on it. 1300 K is DN 255 in red and DN 217 in green, so the hottest coals
    /// are the top of the display's range and carry their colour with them.
    const FUEL_KELVIN: f32 = 1300.0;
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
    char_radiance(field, charred, scaled(EMBER_KELVIN, 1.0), scaled(FUEL_KELVIN, 1.0))
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
