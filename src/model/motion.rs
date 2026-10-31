//! A moving fire's own wind: the air a flame meets is the air it moves through.
//!
//! A flame leans downwind of the air *relative to the fire* (`docs/research/flame-motion.md`: the tilt law's `U` is the speed of the
//! air past the fuel bed, so a bed carried through still air meets what a stationary one meets in a wind). [`relative_air`] is that
//! air, the wind less the fire's own velocity; [`smooth_velocity`] turns a fire's frame-to-frame steps into the velocity its flame
//! feels, a low pass that averages a fixed-step character's uneven frames and a swinging arm's reversals.

/// The time constant, in seconds, of the filter that turns a fire's steps into the velocity its flame feels.
///
/// Design default (`docs/research/flame-motion.md`): a little under one puff of a torch-sized fire (a 0.1 m bed puffs at 4.7 Hz, a
/// period of 0.21 s), slow enough to average a fixed step against the display's rate and a physics jitter, quick enough that a start
/// and a stop read at once. The flame is at 95 % of its lean after three of these, about 0.45 s.
pub const MOTION_RESPONSE: f32 = 0.15;

/// The fastest step, in metres per second, that is a motion rather than a teleport.
///
/// Design default: above a galloping horse (about 20 m/s) and a cart. A step that implies more in one frame is a respawn or an editor
/// drag across the map, and [`smooth_velocity`] ignores it.
pub const MAX_TRACKED_SPEED: f32 = 40.0;

/// The smallest velocity, in metres per second, a fire is kept moving at: under it, with the fire no longer stepping, the velocity is
/// exactly zero, so a fire that has stopped is a still fire again.
///
/// Design default: 0.01 m/s leans a 0.1 m torch about 3 degrees (`flame_tilt`), under a pixel at the courtyard's distances.
pub const MIN_TRACKED_SPEED: f32 = 0.01;

/// The velocity a fire's flame feels after a frame in which the fire stepped by `step` (metres, on the ground plane) in `dt` seconds,
/// given the velocity it felt before: a first-order low pass, `previous + (step / dt - previous) · (1 - e^(-dt / response))`.
///
/// It is frame-rate independent (the same `response` gives the same curve at 30 and 120 frames a second), holds at `dt = 0` (a paused
/// world), ignores a step over [`MAX_TRACKED_SPEED`] (a teleport carries no velocity) and snaps a velocity under
/// [`MIN_TRACKED_SPEED`] to exactly zero once the fire is no longer stepping, so a stopped fire stops costing anything. A
/// non-finite input returns `previous`. `response` is clamped at a millisecond from below, which is the raw velocity.
///
/// ```
/// # use bevy_lights::model::motion::*;
/// // A walker at 1.4 m/s, sampled at 60 Hz: the flame feels it in about half a second.
/// let mut v = [0.0, 0.0];
/// for _ in 0..27 {
///     v = smooth_velocity(v, [1.4 / 60.0, 0.0], 1.0 / 60.0, MOTION_RESPONSE);
/// }
/// assert!((v[0] - 1.4).abs() < 0.08 && v[1] == 0.0);
/// // Stopped, it settles to nothing.
/// for _ in 0..240 {
///     v = smooth_velocity(v, [0.0, 0.0], 1.0 / 60.0, MOTION_RESPONSE);
/// }
/// assert_eq!(v, [0.0, 0.0]);
/// ```
pub fn smooth_velocity(previous: [f32; 2], step: [f32; 2], dt: f32, response: f32) -> [f32; 2] {
    let finite = previous.iter().chain(&step).chain([&dt, &response]).all(|x| x.is_finite());
    if !finite || dt <= 0.0 {
        return previous;
    }
    let raw = [step[0] / dt, step[1] / dt];
    let raw_speed = raw[0].hypot(raw[1]);
    if raw_speed > MAX_TRACKED_SPEED {
        return previous;
    }
    let k = 1.0 - (-dt / response.max(1.0e-3)).exp();
    let v = [previous[0] + (raw[0] - previous[0]) * k, previous[1] + (raw[1] - previous[1]) * k];
    if v[0].hypot(v[1]) < MIN_TRACKED_SPEED && raw_speed < MIN_TRACKED_SPEED { [0.0, 0.0] } else { v }
}

/// The air a fire's flame meets, on the ground plane: the `wind` less `gain` times the fire's own `velocity`, in metres per second.
///
/// A fire carried through still air meets a wind of its velocity reversed, so its flame streams behind it; carried into a wind it
/// meets both, and with the wind behind it, the difference. A fire at rest meets the wind exactly, and `gain` 0 (a negative or `NaN`
/// gain is 0) is the wind whatever the fire does.
///
/// ```
/// # use bevy_lights::model::motion::*;
/// assert_eq!(relative_air([0.0, 0.0], [1.5, 0.0], 1.0), [-1.5, 0.0], "a walker's flame streams behind them");
/// assert_eq!(relative_air([2.0, 0.0], [-1.5, 0.0], 1.0), [3.5, 0.0], "walking into the wind adds");
/// assert_eq!(relative_air([2.0, 0.0], [2.0, 0.0], 1.0), [0.0, 0.0], "moving with the wind: no lean");
/// assert_eq!(relative_air([2.0, 0.0], [1.5, 0.0], 0.0), [2.0, 0.0], "gain 0 is the old behaviour");
/// ```
pub fn relative_air(wind: [f32; 2], velocity: [f32; 2], gain: f32) -> [f32; 2] {
    let gain = gain.max(0.0);
    [wind[0] - gain * velocity[0], wind[1] - gain * velocity[1]]
}

#[cfg(test)]
mod tests {
    use super::*;

    const STEP: f32 = 1.0 / 60.0;

    fn run(v: [f32; 2], speed: f32, frames: usize, dt: f32) -> [f32; 2] {
        (0..frames).fold(v, |v, _| smooth_velocity(v, [speed * dt, 0.0], dt, MOTION_RESPONSE))
    }

    #[test]
    fn a_constant_step_reaches_the_true_velocity_in_about_three_time_constants() {
        let frames = (3.0 * MOTION_RESPONSE / STEP) as usize;
        let v = run([0.0, 0.0], 2.0, frames, STEP);
        assert!((v[0] / 2.0 - 0.95).abs() < 0.02, "{v:?} after {frames} frames");
        let v = run([0.0, 0.0], 2.0, 600, STEP);
        assert!((v[0] - 2.0).abs() < 1e-3);
    }

    #[test]
    fn the_curve_is_the_same_at_any_frame_rate() {
        let at = |dt: f32| run([0.0, 0.0], 1.5, (0.3 / dt).round() as usize, dt)[0];
        let (slow, mid, fast) = (at(1.0 / 30.0), at(1.0 / 60.0), at(1.0 / 120.0));
        assert!((slow - mid).abs() < 0.02 && (mid - fast).abs() < 0.02, "{slow} {mid} {fast}");
    }

    #[test]
    fn a_paused_world_holds_the_velocity() {
        let v = [0.7, -0.2];
        assert_eq!(smooth_velocity(v, [3.0, 3.0], 0.0, MOTION_RESPONSE), v);
        assert_eq!(smooth_velocity(v, [3.0, 3.0], -1.0, MOTION_RESPONSE), v);
    }

    #[test]
    fn a_teleport_carries_no_velocity() {
        let v = [0.7, 0.0];
        // 3 m in a frame is 180 m/s.
        assert_eq!(smooth_velocity(v, [3.0, 0.0], STEP, MOTION_RESPONSE), v);
        // A sprinting cart is not: 30 m/s is tracked.
        assert!(smooth_velocity(v, [30.0 * STEP, 0.0], STEP, MOTION_RESPONSE)[0] > v[0]);
    }

    #[test]
    fn a_fire_that_stops_settles_to_exactly_zero_and_a_creeping_one_is_not_rounded_away() {
        let v = run([0.0, 0.0], 1.4, 120, STEP);
        let settled = (0..300).fold(v, |v, _| smooth_velocity(v, [0.0, 0.0], STEP, MOTION_RESPONSE));
        assert_eq!(settled, [0.0, 0.0], "a stopped fire is a still fire");
        // 5 cm/s builds up: the first frame's share (0.005) is under the floor but the fire is stepping.
        let creep = run([0.0, 0.0], 0.05, 300, STEP);
        assert!((creep[0] - 0.05).abs() < 1e-3, "{creep:?}");
        // 5 mm/s is under the floor and stays nothing.
        assert_eq!(run([0.0, 0.0], 0.005, 300, STEP), [0.0, 0.0]);
    }

    #[test]
    fn a_swinging_arm_reverses_the_velocity_and_the_filter_follows_it_with_a_lag() {
        // A hand swinging at 1 Hz with a 1 m/s peak.
        let mut v = [0.0, 0.0];
        let mut peak: f32 = 0.0;
        for i in 0..600 {
            let t = i as f32 * STEP;
            let raw = (core::f32::consts::TAU * t).sin();
            v = smooth_velocity(v, [raw * STEP, 0.0], STEP, MOTION_RESPONSE);
            if i > 300 {
                peak = peak.max(v[0].abs());
            }
        }
        // First-order low pass: gain 1 / sqrt(1 + (2 pi f tau)^2) = 0.73 at 1 Hz, so the flame sways, a little under the arm.
        assert!((0.6..0.85).contains(&peak), "{peak}");
    }

    #[test]
    fn a_non_finite_input_returns_the_previous_velocity() {
        let v = [0.3, 0.1];
        for bad in [f32::NAN, f32::INFINITY] {
            assert_eq!(smooth_velocity(v, [bad, 0.0], STEP, MOTION_RESPONSE), v);
            assert_eq!(smooth_velocity(v, [0.0, 0.0], bad, MOTION_RESPONSE), v);
            assert_eq!(smooth_velocity(v, [0.0, 0.0], STEP, bad), v);
        }
    }

    #[test]
    fn relative_air_is_the_wind_at_rest_and_for_gain_zero_and_a_bad_gain() {
        assert_eq!(relative_air([1.0, -2.0], [0.0, 0.0], 1.0), [1.0, -2.0]);
        assert_eq!(relative_air([1.0, -2.0], [5.0, 5.0], 0.0), [1.0, -2.0]);
        assert_eq!(relative_air([1.0, -2.0], [5.0, 5.0], -3.0), [1.0, -2.0]);
        assert_eq!(relative_air([1.0, -2.0], [5.0, 5.0], f32::NAN), [1.0, -2.0]);
        assert_eq!(relative_air([0.0, 0.0], [2.0, 0.0], 0.5), [-1.0, 0.0]);
    }
}
