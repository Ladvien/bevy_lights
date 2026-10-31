//! A bed of coals that lives with its fire: how hot it is, what colour that heat glows, and how it is packed for the GPU.
//!
//! [`char`](super::char) says what a bed of coals *is* (a field of lumps and ash, and three temperatures to colour it with). This says
//! what it **does**: it warms as its fire catches, breathes with the flame's flicker and, when the fire is put out, goes on glowing and
//! cools. A bed has a *heat* `h` in `0..=1`: `1` is a fire burning, `0` is cold. Heat moves every temperature of the ramp by the same
//! number of kelvin, [`BED_COOL_KELVIN`] times `1 - h`, and the ramp's radiance follows the blackbody law down, so a cooling bed goes through
//! deepening red to nothing instead of dimming through grey. `docs/research/fuel-bed-heat.md` has the sources and the numbers.
//!
//! Everything here is plain `f32` maths with no engine in it, so the dynamics and the ramp are testable without an app and a host can drive
//! its own bed from them.

use super::char::char_radiance;
use super::cinematography::blackbody_radiance;

/// The width of one repeat of the bed's coal field, in metres: six coarse lumps (`TILE_LUMPS` in [`char`](super::char)) of 6 cm, the size
/// of a coal. A bed wider than this repeats the tile.
pub const BED_TILE: f32 = 0.36;

/// How many kelvin a bed that has cooled all the way (heat `0`) is under a burning one, at every stop of the ramp. At 450 K the hottest
/// coal (1300 K burning) is at 850 K, ten thousand times under its burning radiance: dark on any display. The glow of a body is steep in
/// temperature (it roughly doubles for every 25 K near 1200 K), so this is what makes the bed's glow *die* rather than dim. A design
/// default (`docs/research/fuel-bed-heat.md`).
pub const BED_COOL_KELVIN: f32 = 450.0;

/// The time constant, in seconds, with which a bed warms towards a burning fire's heat: a fire that has just caught lights its bed over
/// seconds, behind the flame's own 0.6 s catch. A design default.
pub const BED_WARM_SECONDS: f32 = 4.0;

/// The time constant, in seconds, of a bed's cooling after its fire is put out when the host does not say: Newton's law makes the heat
/// fall as `exp(-t / cooling)`, and the glow is gone in about a minute (the hottest coal is under [`BED_VISIBLE_SHARE`] of its burning
/// brightness at [`bed_visible_heat`]). A real bed smoulders for tens of minutes; this is the length a game scene can wait. A design default.
pub const DEFAULT_BED_COOLING: f32 = 60.0;

/// How much of the flame's flicker dip the bed's heat follows: the heat it is drawn at is `h (1 - BED_BREATH (1 - swing))`. Small, and it
/// has to be: the glow is steep in temperature, so a 6 % dip in heat (about 8 K at the hottest stop) is already a 25 % swing in brightness,
/// the depth the flame's own flicker has. A design default.
pub const BED_BREATH: f32 = 0.06;

/// The columns (the char field, `0..=1`) and rows (the heat, `0..=1`) of [`bed_ramp`]'s table.
pub const BED_RAMP_FIELD: usize = 32;
/// See [`BED_RAMP_FIELD`]. A cooling bed's glow falls by a factor of about ten thousand across the heat axis, so the rows are close: each
/// is about half a stop apart where the glow is steepest, and a linear blend between two of them is within a few per cent.
pub const BED_RAMP_HEAT: usize = 24;

/// The hottest coal's brightest channel when the bed is burning (`h = 1`) in the table [`bed_ramp`] returns, in linear light: well past
/// the display's white, because a coal at 1300 K is: its red is thousands of times the display's white at a night exposure, and its green
/// a tenth of that, which is what turns the hottest lumps from red to orange and yellow once the tonemapper rolls them off. The dull ember
/// stop is about a hundredth of it. The material multiplies by the scene's fire strength. A design default, set by capture.
pub const BED_PEAK: f32 = 3.5;

/// The share of a burning bed's hottest coal under which a bed is dark: a thousandth, a hundredth of the display's white at
/// [`BED_PEAK`], where a glow is lost against a night sky. A design default.
pub const BED_VISIBLE_SHARE: f32 = 1e-3;

/// The three stops' temperatures when the bed is burning, in kelvin: the same as [`coal_radiance`](super::char::coal_radiance)'s
/// (charred, dull ember, hottest fuel).
const STOPS_KELVIN: [f32; 3] = [950.0, 1020.0, 1300.0];

/// The bed's radiance at char-field sample `field` and heat `heat`, in candelas per square metre as linear RGB.
///
/// At heat `1` this is [`coal_radiance`](super::char::coal_radiance)'s ramp (the ember and the fuel stop to a few per cent: the same
/// temperatures through the same blackbody law; the charred stop differs, because `coal_radiance` clamps it to 1000 K and floors it at a
/// reflected grey, where a glow that is *drawn additively* wants it to be the little it emits). At lower heat every stop is
/// [`BED_COOL_KELVIN`]` * (1 - heat)` kelvin cooler. `heat` is clamped to `0..=1` and a `NaN` is a burning bed; `field` as
/// [`char_radiance`] takes it.
///
/// ```
/// # use bevy_lights::model::bed::*;
/// let burning = bed_radiance(1.0, 1.0);
/// let cooling = bed_radiance(1.0, 0.5);
/// let cold = bed_radiance(1.0, 0.0);
/// // Half the heat is about 175 K cooler: orders of magnitude dimmer, and redder.
/// assert!(burning[0] > cooling[0] * 20.0);
/// assert!(cooling[1] / cooling[0] < burning[1] / burning[0]);
/// // Cold is dark.
/// assert!(cold[0] < burning[0] * 1e-2);
/// ```
pub fn bed_radiance(field: f32, heat: f32) -> [f32; 3] {
    let stops = stops_at(heat);
    char_radiance(field, stops[0], stops[1], stops[2])
}

/// The ramp's three stops (charred, ember, fuel) at `heat`.
fn stops_at(heat: f32) -> [[f32; 3]; 3] {
    let heat = if heat.is_nan() { 1.0 } else { heat.clamp(0.0, 1.0) };
    let drop = BED_COOL_KELVIN * (1.0 - heat);
    STOPS_KELVIN.map(|kelvin| blackbody_radiance(kelvin - drop))
}

/// The brightest channel of the hottest stop of a burning bed: what [`bed_ramp`] divides by.
fn peak() -> f32 {
    let [red, green, blue] = blackbody_radiance(STOPS_KELVIN[2]);
    red.max(green).max(blue).max(f32::MIN_POSITIVE)
}

/// The bed's colour as a table the shader reads: [`BED_RAMP_HEAT`] rows of [`BED_RAMP_FIELD`] columns, row-major (`row * BED_RAMP_FIELD +
/// column`), column `c` the char field at `c / (BED_RAMP_FIELD - 1)` and row `r` the heat at `r / (BED_RAMP_HEAT - 1)`. Each entry is
/// [`bed_radiance`] in the crate's display-referred units, as `[r, g, b, 0]`: divided by the hottest stop's brightest channel at heat 1 and
/// multiplied by [`BED_PEAK`], so a burning bed's hottest coal is [`BED_PEAK`] and the table needs no exposure.
///
/// ```
/// # use bevy_lights::model::bed::*;
/// let table = bed_ramp();
/// assert_eq!(table.len(), BED_RAMP_FIELD * BED_RAMP_HEAT);
/// let hottest = table[(BED_RAMP_HEAT - 1) * BED_RAMP_FIELD + BED_RAMP_FIELD - 1];
/// assert!((hottest[0].max(hottest[1]).max(hottest[2]) - BED_PEAK).abs() < 1e-3);
/// ```
pub fn bed_ramp() -> Vec<[f32; 4]> {
    let scale = BED_PEAK / peak();
    let mut table = Vec::with_capacity(BED_RAMP_FIELD * BED_RAMP_HEAT);
    for row in 0..BED_RAMP_HEAT {
        let stops = stops_at(row as f32 / (BED_RAMP_HEAT - 1) as f32);
        for column in 0..BED_RAMP_FIELD {
            let field = column as f32 / (BED_RAMP_FIELD - 1) as f32;
            let [red, green, blue] = char_radiance(field, stops[0], stops[1], stops[2]);
            table.push([red * scale, green * scale, blue * scale, 0.0]);
        }
    }
    table
}

/// One step of a bed's heat over `dt` seconds: it warms towards `1` with time constant [`BED_WARM_SECONDS`] while its fire `burning`, and
/// cools towards `0` with time constant `cooling` seconds (Newton's law: the excess temperature decays exponentially) once the fire is out.
/// A `cooling` that is not a positive number is [`DEFAULT_BED_COOLING`], a `dt` that is not a non-negative number is no time, and the
/// heat is kept in `0..=1`.
///
/// ```
/// # use bevy_lights::model::bed::*;
/// // Put out, a bed keeps most of its heat for a while and then goes: after one time constant it holds 1/e of it.
/// let mut heat = 1.0;
/// for _ in 0..4500 {
///     heat = bed_heat_step(heat, false, 0.01, 45.0);
/// }
/// assert!((heat - (-1.0f32).exp()).abs() < 1e-3);
/// ```
pub fn bed_heat_step(heat: f32, burning: bool, dt: f32, cooling: f32) -> f32 {
    let heat = if heat.is_nan() { 1.0 } else { heat.clamp(0.0, 1.0) };
    let dt = if dt.is_finite() && dt > 0.0 { dt } else { 0.0 };
    let (target, time) = if burning {
        (1.0, BED_WARM_SECONDS)
    } else {
        (0.0, if cooling.is_finite() && cooling > 0.0 { cooling } else { DEFAULT_BED_COOLING })
    };
    (heat + (target - heat) * (1.0 - (-dt / time).exp())).clamp(0.0, 1.0)
}

/// The heat a bed is *drawn* at: its thermal `heat` less a share ([`BED_BREATH`]) of the fire's flicker dip, `swing` being
/// [`flicker_swing`](super::fire::flicker_swing) (`1` at the flame's peak). The coals brighten and dim with the flame, a little, and a bed
/// that is cold stays cold.
///
/// ```
/// # use bevy_lights::model::bed::*;
/// assert_eq!(bed_drawn_heat(1.0, 1.0), 1.0);
/// assert!(bed_drawn_heat(1.0, 0.6) < 1.0);
/// assert_eq!(bed_drawn_heat(0.0, 0.6), 0.0);
/// ```
pub fn bed_drawn_heat(heat: f32, swing: f32) -> f32 {
    let heat = if heat.is_nan() { 1.0 } else { heat.clamp(0.0, 1.0) };
    let dip = if swing.is_nan() { 0.0 } else { (1.0 - swing).clamp(0.0, 1.0) };
    heat * (1.0 - BED_BREATH * dip)
}

/// The field of the tile a bed of `seed` shows from, as the fraction of the tile its `u` and `v` start at: two integer-hashed numbers in
/// `0..1`, so beds with different seeds show different coals and every bed is the same on every machine.
pub fn bed_offset(seed: u32) -> [f32; 2] {
    let mix = |mut v: u32| {
        v = (v ^ (v >> 16)).wrapping_mul(0x7feb_352d);
        v = (v ^ (v >> 15)).wrapping_mul(0x846c_a68b);
        v ^ (v >> 16)
    };
    let a = mix(seed.wrapping_add(0x9e37_79b9));
    let b = mix(a ^ 0x85eb_ca6b);
    [(a & 0xff) as f32 / 256.0, (b & 0xff) as f32 / 256.0]
}

/// A bed's per-instance data in one `u32` (the `MeshTag`): the drawn heat in the low sixteen bits, the tile offset's `u` and `v` a byte
/// each above it. `heat` is clamped to `0..=1` and the offset to `0..1`.
pub fn pack_bed_tag(heat: f32, offset: [f32; 2]) -> u32 {
    let heat = if heat.is_nan() { 1.0 } else { heat.clamp(0.0, 1.0) };
    let byte = |v: f32| if v.is_nan() { 0 } else { (v.clamp(0.0, 1.0) * 255.0).round() as u32 };
    (heat * 65_535.0).round() as u32 | (byte(offset[0]) << 16) | (byte(offset[1]) << 24)
}

/// The inverse of [`pack_bed_tag`]: `(heat, offset)`.
pub fn unpack_bed_tag(tag: u32) -> (f32, [f32; 2]) {
    (
        (tag & 0xffff) as f32 / 65_535.0,
        [((tag >> 16) & 0xff) as f32 / 255.0, ((tag >> 24) & 0xff) as f32 / 255.0],
    )
}

/// The heat under which a bed's hottest coal has fallen to [`BED_VISIBLE_SHARE`] of its burning brightness, a hundredth of the display's
/// white at the default strength: dark enough that a bed under it may be left undrawn. Found by bisection on [`bed_radiance`], so it
/// follows the temperatures and not a number written beside them.
pub fn bed_visible_heat() -> f32 {
    let scale = BED_PEAK / peak();
    let hottest = |heat: f32| {
        let [red, green, blue] = bed_radiance(1.0, heat);
        red.max(green).max(blue) * scale
    };
    let (mut low, mut high) = (0.0f32, 1.0f32);
    for _ in 0..24 {
        let middle = 0.5 * (low + high);
        if hottest(middle) < BED_PEAK * BED_VISIBLE_SHARE {
            low = middle;
        } else {
            high = middle;
        }
    }
    high
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::model::char::{EMBER_AT, char_tile, coal_radiance};

    fn max_channel(rgb: [f32; 3]) -> f32 {
        rgb[0].max(rgb[1]).max(rgb[2])
    }

    #[test]
    fn a_burning_bed_is_the_coal_ramp_at_its_ember_and_fuel_stops() {
        // Two temperatures through one blackbody law: the bed and `coal_radiance` agree where the bed's glow is, to what two
        // integrations of the spectrum (one normalised and rescaled, one not) differ by.
        for field in [EMBER_AT, 1.0] {
            let bed = bed_radiance(field, 1.0);
            let coal = coal_radiance(field);
            for channel in 0..3 {
                let (a, b) = (bed[channel], coal[channel]);
                let tolerance = 0.05 * b.max(a) + 0.5;
                assert!((a - b).abs() <= tolerance, "field {field} channel {channel}: bed {a} vs coal {b}");
            }
        }
    }

    #[test]
    fn a_cooler_bed_is_never_brighter_in_any_channel_at_any_field() {
        for field in (0..=20).map(|i| i as f32 / 20.0) {
            let mut before = bed_radiance(field, 1.0);
            for step in 1..=20 {
                let now = bed_radiance(field, 1.0 - step as f32 / 20.0);
                for channel in 0..3 {
                    assert!(
                        now[channel] <= before[channel] * 1.0001 + 1e-9,
                        "field {field} step {step} channel {channel}: {} then {}",
                        before[channel],
                        now[channel]
                    );
                }
                before = now;
            }
        }
    }

    #[test]
    fn a_cooling_bed_gets_redder_as_it_dims_which_a_grey_dimming_would_not() {
        let red_share = |heat: f32| {
            let [r, g, b] = bed_radiance(1.0, heat);
            r / (r + g + b)
        };
        let mut before = red_share(1.0);
        for step in 1..=14 {
            let now = red_share(1.0 - step as f32 * 0.03);
            assert!(now >= before - 1e-4, "step {step}: the red share fell from {before} to {now}");
            before = now;
        }
        assert!(red_share(0.6) > red_share(1.0) + 0.02, "{} vs {}", red_share(0.6), red_share(1.0));
    }

    #[test]
    fn a_cold_bed_is_dark_on_a_display_and_a_burning_one_has_its_dynamic_range() {
        let table = bed_ramp();
        let cold_peak =
            table[..BED_RAMP_FIELD].iter().map(|c| max_channel([c[0], c[1], c[2]])).fold(0.0f32, f32::max);
        assert!(
            cold_peak < 0.005 * BED_PEAK,
            "a cold bed's brightest entry is {cold_peak} of a display's white"
        );
        let top = (BED_RAMP_HEAT - 1) * BED_RAMP_FIELD;
        let burning = &table[top..top + BED_RAMP_FIELD];
        let hottest = burning.iter().map(|c| max_channel([c[0], c[1], c[2]])).fold(0.0f32, f32::max);
        assert!((hottest - BED_PEAK).abs() < 1e-3);
        // The dull ember stop is a fraction of the hottest coal, so a burning bed is lumps on dark, not a plate.
        let ember = burning[(EMBER_AT * (BED_RAMP_FIELD - 1) as f32).round() as usize];
        let ember = max_channel([ember[0], ember[1], ember[2]]);
        assert!(ember > 0.002 * hottest && ember < 0.25 * hottest, "ember {ember} against {hottest}");
    }

    #[test]
    fn the_table_is_finite_and_never_negative() {
        for entry in bed_ramp() {
            assert!(entry.iter().all(|c| c.is_finite() && *c >= 0.0), "{entry:?}");
        }
    }

    #[test]
    fn a_fire_put_out_leaves_its_bed_most_of_its_heat_a_second_later_and_almost_none_after_four_time_constants()
     {
        let mut heat = 1.0;
        for _ in 0..60 {
            heat = bed_heat_step(heat, false, 1.0 / 60.0, DEFAULT_BED_COOLING);
        }
        assert!(heat > 0.97, "a second after the douse the bed holds {heat}");
        for _ in 0..(4.0 * DEFAULT_BED_COOLING * 60.0) as usize {
            heat = bed_heat_step(heat, false, 1.0 / 60.0, DEFAULT_BED_COOLING);
        }
        assert!(heat < 0.03, "after four time constants {heat}");
    }

    #[test]
    fn the_coals_stop_glowing_visibly_well_before_the_heat_is_zero_and_a_minute_or_so_after_the_douse() {
        let visible = bed_visible_heat();
        assert!((0.1..0.8).contains(&visible), "{visible}");
        // The time from the douse to that heat: `-cooling ln(visible)`, a minute-ish at the default.
        let seconds = -DEFAULT_BED_COOLING * visible.ln();
        assert!((15.0..120.0).contains(&seconds), "{seconds} s");
    }

    #[test]
    fn a_relight_warms_the_bed_from_where_it_had_cooled_to() {
        let mut heat = 0.2;
        for _ in 0..(3.0 * BED_WARM_SECONDS * 60.0) as usize {
            heat = bed_heat_step(heat, true, 1.0 / 60.0, DEFAULT_BED_COOLING);
        }
        assert!(heat > 0.95, "{heat}");
        assert!(bed_heat_step(0.2, true, 1.0 / 60.0, DEFAULT_BED_COOLING) > 0.2);
    }

    #[test]
    fn the_heat_step_survives_what_a_setting_can_be() {
        for (heat, dt, cooling) in [
            (f32::NAN, 0.016, 45.0),
            (0.5, f32::NAN, 45.0),
            (0.5, -1.0, 45.0),
            (0.5, f32::INFINITY, 45.0),
            (0.5, 0.016, f32::NAN),
            (0.5, 0.016, 0.0),
            (0.5, 0.016, -3.0),
            (7.0, 0.016, 45.0),
        ] {
            for burning in [true, false] {
                let now = bed_heat_step(heat, burning, dt, cooling);
                assert!((0.0..=1.0).contains(&now), "{heat} {dt} {cooling} {burning}: {now}");
            }
        }
    }

    #[test]
    fn the_breathing_follows_the_flame_a_little_and_never_warms_a_cold_bed() {
        assert_eq!(bed_drawn_heat(0.8, 1.0), 0.8);
        let dipped = bed_drawn_heat(0.8, 0.5);
        assert!(dipped < 0.8 && dipped > 0.8 * (1.0 - BED_BREATH) - 1e-6, "{dipped}");
        assert_eq!(bed_drawn_heat(0.0, 0.0), 0.0);
        assert!(bed_drawn_heat(f32::NAN, f32::NAN).is_finite());
        // The swing is a share of the heat, so it is the same dip at any heat in the bed's own terms.
        assert!(bed_drawn_heat(1.0, 0.0) >= 1.0 - BED_BREATH - 1e-6);
    }

    #[test]
    fn the_tag_round_trips_its_heat_and_its_offset_and_the_fields_do_not_bleed() {
        for heat in [0.0, 0.25, 0.7, 1.0] {
            for offset in [[0.0, 0.0], [0.5, 0.25], [0.996, 0.004]] {
                let (h, o) = unpack_bed_tag(pack_bed_tag(heat, offset));
                assert!((h - heat).abs() < 1e-4);
                assert!((o[0] - offset[0]).abs() < 1.0 / 255.0 && (o[1] - offset[1]).abs() < 1.0 / 255.0);
            }
        }
        let (h, o) = unpack_bed_tag(pack_bed_tag(1.0, [0.0, 0.0]));
        assert_eq!((h, o), (1.0, [0.0, 0.0]));
        let (h, _) = unpack_bed_tag(pack_bed_tag(f32::NAN, [f32::NAN, 2.0]));
        assert!(h.is_finite());
    }

    #[test]
    fn two_seeds_show_two_windows_of_the_same_tile_and_one_seed_is_always_the_same() {
        assert_eq!(bed_offset(3), bed_offset(3));
        assert_ne!(bed_offset(3), bed_offset(4));
        for seed in 0..64 {
            let [u, v] = bed_offset(seed);
            assert!((0.0..1.0).contains(&u) && (0.0..1.0).contains(&v), "{seed}: {u} {v}");
        }
    }

    #[test]
    fn the_tile_has_the_fields_shape_and_repeats() {
        // The tile is the courtyard's heap made periodic: ash and the hottest lump are both in it, and its mean is the field's.
        let mut values = Vec::new();
        for i in 0..64 {
            for j in 0..64 {
                values.push(char_tile(i as f32 / 64.0, j as f32 / 64.0, 7));
            }
        }
        let mean = values.iter().sum::<f32>() / values.len() as f32;
        assert!((0.35..0.65).contains(&mean), "mean {mean}");
        assert!(values.iter().any(|v| *v < 0.05) && values.iter().any(|v| *v > 0.9));
        for k in 0..32 {
            let (u, v) = (k as f32 * 0.0371, k as f32 * 0.0613);
            let a = char_tile(u, v, 7);
            assert!((a - char_tile(u + 1.0, v, 7)).abs() < 1e-3, "u repeat at {k}");
            assert!((a - char_tile(u, v + 2.0, 7)).abs() < 1e-3, "v repeat at {k}");
            assert!((a - char_tile(u - 3.0, v - 1.0, 7)).abs() < 1e-3, "negative repeat at {k}");
        }
        assert_ne!(char_tile(0.3, 0.7, 1), char_tile(0.3, 0.7, 2));
    }
}
