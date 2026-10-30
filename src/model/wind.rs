//! Gusts: the turbulent part of a wind, as a pure function of time.
//!
//! Outdoor wind is a mean plus a stationary Gaussian fluctuation whose size is the *turbulence
//! intensity* `I = σ / mean` (Hannesdóttir, Kelly & Dimitrov 2019, `10.5194/wes-4-325-2019`:
//! "Gaussian turbulent velocity component fluctuations … assumed to be stationary and homogeneous").
//! [`gust_unit`] is that fluctuation at unit variance, built from a sum of sinusoids so it needs no
//! state: the same `(t, seed)` gives the same value on every machine, on every frame, and a host can
//! sample it at any time. [`gust_velocity`] applies it to a mean wind.
//!
//! The spectrum's shape (one-pole, with a correlation time) and the defaults are design defaults;
//! `docs/research/flame-gust.md` has the reasoning.

use core::f64::consts::TAU;

/// How many sinusoids make one gust process.
pub const GUST_TERMS: usize = 12;

/// The turbulence intensity a gust defaults to: `σ_U / mean`. A design default, above the IEC's
/// class C of 0.12 because ground level among walls is rougher than hub height over open water.
pub const GUST_INTENSITY: f32 = 0.25;

/// The correlation time a gust defaults to, in seconds. A design default: slow against a flame's own
/// 1–3 Hz puff, quick enough that a gust builds and drops within a few seconds.
pub const GUST_TIMESCALE: f32 = 2.0;

/// The shortest correlation time accepted; below it the process would be white noise at frame rate.
const MIN_TIMESCALE: f32 = 0.05;

/// The lowest tone, as a fraction of the spectrum's corner frequency.
const LOWEST: f64 = 0.125;

/// Frequency ratio between neighbouring tones.
const RATIO: f64 = core::f64::consts::SQRT_2;

/// A well-mixed 32-bit hash (the `lowbias32` finaliser), for phases.
fn mix(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^ (x >> 16)
}

/// A zero-mean, unit-variance gust process at time `t` seconds, with correlation time `timescale`
/// seconds, for the stream `seed`.
///
/// A sum of [`GUST_TERMS`] sinusoids, log-spaced from 0.125 to about 5.7 times the corner frequency
/// `1 / (2π · timescale)`, with random phases from `seed` and amplitudes from the one-pole spectrum
/// `S(f) ∝ 1 / (1 + (f / f_c)²)`, normalised so the variance is exactly 1. It is smooth (no
/// frequency above 5.7 corners) and quasi-periodic, never repeating inside a session. `timescale` is
/// clamped at 0.05 s from below; a `NaN` in `t` or `timescale` gives 0.
///
/// ```
/// # use bevy_lights::model::wind::*;
/// assert_eq!(gust_unit(3.5, 7, 2.0), gust_unit(3.5, 7, 2.0), "deterministic");
/// assert_ne!(gust_unit(3.5, 7, 2.0), gust_unit(3.5, 8, 2.0), "a seed is a stream");
/// ```
pub fn gust_unit(t: f64, seed: u32, timescale: f32) -> f32 {
    if t.is_nan() || timescale.is_nan() {
        return 0.0;
    }
    let corner = 1.0 / (TAU * f64::from(timescale.max(MIN_TIMESCALE)));
        let mut total = 0.0;
    let mut sum = 0.0;
    let mut freq = LOWEST;
    let mut tones = [(0.0, 0.0); GUST_TERMS];
    for (k, tone) in tones.iter_mut().enumerate() {
        // log-spaced tones weigh f · S(f): equal weight per octave under the spectrum.
        let weight = freq / (1.0 + freq * freq);
        let phase = f64::from(mix(seed ^ mix(k as u32 + 1))) / f64::from(u32::MAX) * TAU;
        *tone = (weight, phase);
        sum += weight;
        freq *= RATIO;
    }
    let mut freq = LOWEST;
    for (weight, phase) in tones {
        let amplitude = (2.0 * weight / sum).sqrt();
        total += amplitude * (TAU * freq * corner * t + phase).sin();
        freq *= RATIO;
    }
    total as f32
}

/// A mean wind `mean` (`x`, `z` in metres per second) with gusts at time `t`: the speed is
/// `|mean| · (1 + intensity · x)`, never negative, and the heading is turned by `intensity · y`
/// radians, where `x` and `y` are independent [`gust_unit`] draws of `seed`.
///
/// A zero mean stays zero (no heading to wander about). A non-finite mean, or an `intensity` that is
/// `NaN`, gives zero; a negative `intensity` is treated as 0.
///
/// ```
/// # use bevy_lights::model::wind::*;
/// assert_eq!(gust_velocity((2.0, 0.0), 0.0, 2.0, 1, 5.0), (2.0, 0.0), "no turbulence: the mean");
/// let (x, z) = gust_velocity((2.0, 0.0), 0.25, 2.0, 1, 5.0);
/// assert!(x > 0.0 && z.abs() < x, "a gust moves the speed and a little of the heading");
/// ```
pub fn gust_velocity(mean: (f32, f32), intensity: f32, timescale: f32, seed: u32, t: f64) -> (f32, f32) {
    let speed = mean.0.hypot(mean.1);
    if !speed.is_finite() || intensity.is_nan() || speed == 0.0 {
        return (0.0, 0.0);
    }
    let intensity = intensity.max(0.0);
    let along = gust_unit(t, seed, timescale);
    let across = gust_unit(t, seed ^ 0x9e37_79b9, timescale);
    let speed = (speed * (1.0 + intensity * along)).max(0.0);
    let heading = mean.1.atan2(mean.0) + intensity * across;
    (speed * heading.cos(), speed * heading.sin())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn samples(seed: u32, timescale: f32, seconds: f32, rate: f32) -> Vec<f32> {
        let n = (seconds * rate) as usize;
        (0..n).map(|i| gust_unit(i as f64 / f64::from(rate), seed, timescale)).collect()
    }

    fn mean_var(v: &[f32]) -> (f32, f32) {
        let mean = v.iter().sum::<f32>() / v.len() as f32;
        (mean, v.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / v.len() as f32)
    }

    #[test]
    fn the_process_is_zero_mean_and_unit_variance_for_every_seed() {
        for seed in [0, 1, 7, 42, 1234, 99_999] {
            let (mean, var) = mean_var(&samples(seed, 2.0, 2000.0, 20.0));
            assert!(mean.abs() < 0.25, "seed {seed}: mean {mean}");
            assert!((0.6..1.4).contains(&var), "seed {seed}: variance {var}");
        }
    }

    #[test]
    fn it_is_smooth_at_frame_rate() {
        for seed in [3, 11] {
            let v = samples(seed, 2.0, 600.0, 60.0);
            let worst = v.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0, f32::max);
            assert!(worst < 0.1, "seed {seed}: a frame moved the gust by {worst} sigma");
        }
    }

    #[test]
    fn the_correlation_time_is_the_timescale() {
        for timescale in [1.0_f32, 2.0, 4.0] {
            let rate = 10.0;
            let v = samples(5, timescale, 4000.0, rate);
            let (mean, var) = mean_var(&v);
            let lag = (timescale * rate) as usize;
            let c = v.iter().zip(&v[lag..]).map(|(a, b)| (a - mean) * (b - mean)).sum::<f32>()
                / (v.len() - lag) as f32
                / var;
            assert!((c - (-1.0f32).exp()).abs() < 0.2, "timescale {timescale}: r(T) = {c}");
        }
    }

    #[test]
    fn no_turbulence_is_the_mean_and_a_gust_never_reverses_the_speed() {
        for i in 0..200 {
            let t = i as f64 * 0.37;
            assert_eq!(gust_velocity((2.0, 0.0), 0.0, 2.0, 9, t), (2.0, 0.0));
            // An absurd intensity can zero the speed but never flips it.
            let (x, z) = gust_velocity((1.0, 1.0), 5.0, 2.0, 9, t);
            assert!(x.is_finite() && z.is_finite());
            let v = x.hypot(z);
            assert!(v >= 0.0 && v < 1.0 + 1.5 * 2.0_f32.sqrt() * 5.0 * 10.0);
        }
    }

    #[test]
    fn the_heading_wanders_by_about_the_intensity() {
        let intensity = 0.25;
        let angles: Vec<f32> = (0..20_000)
            .map(|i| {
                let (x, z) = gust_velocity((3.0, 0.0), intensity, 2.0, 4, i as f64 * 0.1);
                z.atan2(x)
            })
            .collect();
        let (mean, var) = mean_var(&angles);
        assert!(mean.abs() < 0.1, "mean heading {mean}");
        assert!((var.sqrt() - intensity).abs() < 0.1, "heading sigma {}", var.sqrt());
    }

    #[test]
    fn degenerate_inputs_give_still_air_not_nan() {
        assert_eq!(gust_velocity((0.0, 0.0), 0.3, 2.0, 1, 1.0), (0.0, 0.0));
        assert_eq!(gust_velocity((f32::NAN, 0.0), 0.3, 2.0, 1, 1.0), (0.0, 0.0));
        assert_eq!(gust_velocity((1.0, 0.0), f32::NAN, 2.0, 1, 1.0), (0.0, 0.0));
        assert_eq!(gust_unit(f64::NAN, 1, 2.0), 0.0);
        assert!(gust_unit(1.0, 1, 0.0).is_finite(), "a zero timescale is clamped");
    }

    #[test]
    fn a_long_session_is_as_smooth_as_the_start() {
        // t in f32 loses resolution after hours; the sum runs in f64 and stays smooth.
        let t0 = 40.0 * 3600.0_f64;
        let worst = (0..600)
            .map(|i| {
                let a = gust_unit(t0 + f64::from(i) / 60.0, 3, 2.0);
                let b = gust_unit(t0 + f64::from(i + 1) / 60.0, 3, 2.0);
                (b - a).abs()
            })
            .fold(0.0, f32::max);
        assert!(worst < 0.15, "{worst}");
    }
}
