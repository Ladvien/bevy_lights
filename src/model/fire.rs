//! Fire light: how a fire's intensity moves from frame to frame, and how it takes hold when it is
//! first lit.
//!
//! Both are **design defaults** inherited from Sanctuary (decision 0127), not measurements: the
//! two sine rates and the kindle flare were tuned by eye in that game, and are kept identical here
//! so a migrated world's fires look the same on the day the crate is adopted. The physical model
//! (a pool fire's puffing frequency against its heat release rate) is the first item on
//! `docs/loop-log.md`'s backlog, and `docs/research/fire-light.md` records what the corpus holds.

use core::f32::consts::PI;

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
