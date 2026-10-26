//! Fire light: how a fire's intensity moves from frame to frame, and how it takes hold when it is
//! first lit.
//!
//! The **rate** is measured: a buoyant diffusion flame puffs at `f = 1.5 / √D` Hz for a fuel bed `D`
//! metres across (Cetegen & Ahmed 1993; Xia & Zhang 2018, whose fit to six experimental studies is
//! `f = 0.48 √(g/D)`). [`flicker_rate`] is that law and [`flicker_factor_of`] is the flicker sized by
//! it. The **shape** — two sines, `0.85 ± 0.15`, the second at 1.7 times the first — and the kindle
//! flare are design defaults inherited from Sanctuary (decision 0127), kept so a migrated world's
//! fires look the same on the day the crate is adopted. `flicker_factor` is the shape at Sanctuary's
//! fixed rates, for a host with no size to give.
//!
//! `docs/research/fire-light.md` records the law, its constants and the two papers it comes from.

use core::f32::consts::{PI, TAU};

/// The constant in the puffing-frequency law: `f = 1.5 / √D` Hz.
///
/// From Cetegen & Ahmed 1993 (`f ∝ D^-1/2` with a factor of about 1.5), restated by Xia & Zhang 2018
/// as `f = 0.48 √(g/D)` with `g = 9.8 m/s²`; the two are the same law, since `0.48·√9.8 = 1.503`.
pub const PUFFING_CONSTANT: f32 = 1.5;

/// The narrowest a fuel bed may be before the law is asked for a rate it does not describe, in the
/// host's world units (metres for a metre world). At 1 cm the rate is 15 Hz, which is a candle flame
/// rather larger than a wick; below this the rate is clamped and stays finite.
const MIN_BED: f32 = 0.01;

/// How fast a fire of fuel-bed diameter `diameter` flickers, in hertz: `1.5 / √diameter`.
///
/// The law is the literature's: a buoyant diffusion flame sheds a toroidal vortex at a frequency set
/// by the fuel bed's size, faster for a small fire and slower for a large one (Cetegen & Ahmed 1993;
/// Xia & Zhang 2018). A 0.34 m coal bed is 2.6 Hz; a 1 m brazier 1.5 Hz; a 4 m pyre 0.75 Hz.
///
/// `diameter` is in the host's world units (metres for a metre world) and is clamped at 1 cm from
/// below, so a degenerate bed gives a fast finite rate rather than an infinity. A `NaN` is 1 Hz.
///
/// ```
/// # use bevy_lights::model::fire::*;
/// assert!((flicker_rate(1.0) - 1.5).abs() < 1e-5);
/// assert!((flicker_rate(0.34) - 2.573).abs() < 1e-2, "a brazier's coals");
/// assert!(flicker_rate(0.1) > flicker_rate(1.0), "a small fire is faster");
/// ```
pub fn flicker_rate(diameter: f32) -> f32 {
    if diameter.is_nan() {
        return 1.0;
    }
    PUFFING_CONSTANT / diameter.clamp(MIN_BED, 1.0e6).sqrt()
}

/// The flicker multiplier of a fire whose fuel bed is `diameter` across, at time `t` seconds and clock
/// `phase` radians: the same two-sine shape as [`flicker_factor`], run at the rate the law gives.
///
/// The first sine runs at `f = `[`flicker_rate`]`(diameter)` hertz (`ω = 2πf` rad/s) and the second at
/// 1.7 times it, so two fires of different sizes do not flicker in step *and* a larger fire's light
/// moves more slowly than a smaller one's — which is the whole point of sizing it. The window and the
/// mean are the same as [`flicker_factor`]'s (`0.85 ± 0.15`), because a lumen budget is written
/// against that mean.
///
/// ```
/// # use bevy_lights::model::fire::*;
/// for diameter in [0.1, 0.34, 1.0, 4.0] {
///     let mut t = 0.0;
///     while t <= 5.0 {
///         let f = flicker_factor_of(diameter, t, 0.0);
///         assert!((0.70..=1.00).contains(&f));
///         t += 0.01;
///     }
/// }
/// ```
pub fn flicker_factor_of(diameter: f32, t: f32, phase: f32) -> f32 {
    let omega = TAU * flicker_rate(diameter);
    let wobble = (omega * t + phase).sin() * 0.5 + (1.7 * omega * t + 1.7 * phase).sin() * 0.5;
    0.85 + 0.15 * wobble
}

/// The flicker multiplier of a fire, sized by its fuel bed when the host knows it: the one place the
/// "sized or un-sized" choice is made.
///
/// `Some(diameter)` is [`flicker_factor_of`] and `None` is [`flicker_factor`]. A fire's **light** and
/// the **flame drawn on it** are one fire, so both are written from this function: the light's
/// intensity by `flicker_lights` and the flame's emission by `update_flames`. Going through one
/// function is what makes it impossible for a flame to hold still while the ground under it flickers,
/// which is the defect this exists to prevent.
///
/// ```
/// # use bevy_lights::model::fire::*;
/// assert_eq!(flicker_swing(None, 2.0, 0.4), flicker_factor(2.0, 0.4));
/// assert_eq!(flicker_swing(Some(0.34), 2.0, 0.4), flicker_factor_of(0.34, 2.0, 0.4));
/// ```
pub fn flicker_swing(diameter: Option<f32>, t: f32, phase: f32) -> f32 {
    match diameter {
        Some(diameter) => flicker_factor_of(diameter, t, phase),
        None => flicker_factor(t, phase),
    }
}

/// How long a fire takes to take hold, in seconds: a light that has a [`Kindle`] flares up over this
/// long instead of switching on (Sanctuary decision 0127).
///
/// [`Kindle`]: crate::Kindle
pub const KINDLE_SECONDS: f32 = 0.6;

/// The flicker multiplier of a fire light at time `t` seconds and clock `phase` radians: a sum of two
/// sines mapped to 0.70..1.00 of the light's base intensity, mean 0.85.
///
/// The rates are 7.3 and 13.1 rad/s (about 1.2 and 2.1 Hz), and the second sine runs at 1.7 times
/// `phase`, so two fires at different phases never drift back into step. Both rates are a design
/// default inherited from Sanctuary and are **uncited**; `docs/research/fire-light.md` records the
/// puffing-frequency literature they should be replaced with.
///
/// The result is exactly `0.85 ± 0.15` in the range `[0.70, 1.00]`, which is what a light budget
/// written against the mean (Sanctuary's `WALL_LIGHT_LUMENS` divides by 0.85) assumes.
pub fn flicker_factor(t: f32, phase: f32) -> f32 {
    let wobble = (7.3 * t + phase).sin() * 0.5 + (13.1 * t + 1.7 * phase).sin() * 0.5;
    0.85 + 0.15 * wobble
}

/// The share of its burn a fire has `t` seconds after it was lit: 0 at the start, then a flare that
/// runs ahead of the smoothstep rise (`s + 0.35 sin(pi s)`: 0.85 at half time, a hair over 1 at 0.85
/// of the way) and settles to exactly 1 from [`KINDLE_SECONDS`] on.
///
/// The flare is what makes a lamp being lit read as fire rather than as a switch (Sanctuary decision
/// 0127). It is a design default, not a measurement.
pub fn kindle_gain(t: f32) -> f32 {
    let s = (t / KINDLE_SECONDS).clamp(0.0, 1.0);
    let s = s * s * (3.0 - 2.0 * s);
    s + 0.35 * (PI * s).sin()
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn the_flicker_stays_inside_its_budget_window() {
        for phase in 0..7 {
            let phase = phase as f32;
            let mut t = 0.0;
            while t <= 100.0 {
                let f = flicker_factor(t, phase);
                assert!((0.70..=1.00).contains(&f), "flicker {f} at t {t} phase {phase} left 0.70..1.00");
                t += 0.01;
            }
        }
    }

    #[test]
    fn the_flicker_rate_is_the_puffing_law() {
        // f = 1.5 / sqrt(D): a 1 m bed is 1.5 Hz, a brazier's 0.34 m coals 2.57 Hz, a big fire slower.
        for (diameter, hertz) in [(1.0, 1.5), (0.34, 2.573), (0.1, 4.743), (4.0, 0.75)] {
            let rate = flicker_rate(diameter);
            assert!((rate - hertz).abs() < 1e-2, "{diameter} m gave {rate} Hz, wanted {hertz}");
        }
        // Monotone falling in the diameter, and a degenerate bed stays finite.
        let mut last = f32::INFINITY;
        for diameter in [0.05, 0.1, 0.5, 1.0, 2.0, 8.0] {
            let rate = flicker_rate(diameter);
            assert!(rate < last, "{diameter} m did not slow the rate: {rate} against {last}");
            assert!(rate.is_finite() && rate > 0.0);
            last = rate;
        }
        assert!(flicker_rate(0.0).is_finite(), "a zero bed clamps rather than dividing by zero");
        assert!(flicker_rate(-2.0).is_finite());
        assert_eq!(flicker_rate(f32::NAN), 1.0);
    }

    #[test]
    fn a_sized_flicker_keeps_the_window_and_the_mean_and_changes_the_rate() {
        for diameter in [0.05, 0.34, 1.0, 8.0] {
            let mut t = 0.0;
            let mut sum = 0.0;
            let mut n = 0.0;
            let mut last = flicker_factor_of(diameter, 0.0, 0.0);
            let mut turns = 0;
            while t <= 20.0 {
                let f = flicker_factor_of(diameter, t, 0.0);
                assert!((0.70..=1.00).contains(&f), "{diameter} m left the window at {t}: {f}");
                // Count how often the curve crosses its mean going up: a proxy for the rate.
                if last < 0.85 && f >= 0.85 {
                    turns += 1;
                }
                last = f;
                sum += f;
                n += 1.0;
                t += 0.005;
            }
            let mean = sum / n;
            assert!((mean - 0.85).abs() < 0.01, "{diameter} m's mean is {mean}, not 0.85");
            // Over 20 s the law's rate for a small fire must turn more often than a large one's.
            if diameter < 0.2 {
                assert!(turns > 10, "a {diameter} m fire turned only {turns} times in 20 s");
            }
        }
        // The sized rate is faster than the inherited one at a brazier's size, and slower at a pyre's.
        let crosses = |f: &dyn Fn(f32) -> f32| {
            let mut last = f(0.0);
            let mut turns = 0;
            let mut t = 0.0;
            while t <= 10.0 {
                let v = f(t);
                if last < 0.85 && v >= 0.85 {
                    turns += 1;
                }
                last = v;
                t += 0.005;
            }
            turns
        };
        let sized_small = crosses(&|t| flicker_factor_of(0.34, t, 0.0));
        let sized_large = crosses(&|t| flicker_factor_of(4.0, t, 0.0));
        let inherited = crosses(&|t| flicker_factor(t, 0.0));
        assert!(sized_small > inherited, "a brazier should flicker faster than the inherited rate");
        assert!(sized_large < inherited, "a pyre should flicker slower");
    }

    #[test]
    fn the_kindle_flares_then_settles_to_one() {
        assert_eq!(kindle_gain(0.0), 0.0);
        assert!((kindle_gain(0.3) - 0.85).abs() < 1e-4, "half time {:?}", kindle_gain(0.3));
        assert!((kindle_gain(0.6) - 1.0).abs() < 1e-5, "settled {:?}", kindle_gain(0.6));
        assert_eq!(
            kindle_gain(10.0),
            kindle_gain(KINDLE_SECONDS),
            "the gain is exactly 1 after KINDLE_SECONDS"
        );
    }
}
