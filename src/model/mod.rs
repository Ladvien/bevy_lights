//! The engine-free half of the crate: plain `f32` maths with no `bevy` in it.
//!
//! `tests/leaf.rs` enforces that. The point is that the shapes of these curves — how a flame
//! flickers, how a light takes hold, how far a raised light reaches, how a flame's height and phase
//! are packed — are testable without an app, a window or a GPU, and are the part of the crate a
//! host is most likely to want to reuse in its own systems.

pub mod char;
pub mod cinematography;
pub mod fire;
pub mod pool;
pub mod problem;
pub mod reach;
pub mod tag;
pub mod wind;

pub use char::{EMBER_AT, char_field, char_radiance, coal_radiance};
pub use cinematography::{
    HIGH_KEY, LOW_KEY, MOON_KEY, blackbody_colour, blackbody_luminance, fill_for, fire_lumens, fire_lux,
    fire_reach, ground_lux, ground_ramp, ground_reach, key_to_fill, stops, white_balanced,
};
pub use fire::{
    CALM_FLICKER_DEPTH, DEFAULT_FIRE_DIAMETER, FLAME_TILT_LIMIT, FLICKER_DEPTH, KINDLE_SECONDS,
    MAX_FLICKER_DEPTH, PUFFING_CONSTANT, SNUFF_SECONDS, WHIPPING_FLICKER_DEPTH, flame_tilt, flicker_depth,
    flicker_factor, flicker_factor_of, flicker_mean, flicker_rate, flicker_swing, kindle_gain, snuff_gain,
};
pub use pool::{FADE_SECONDS, HYSTERESIS, fade_factor, fade_out_factor, hand_off, preferred, shadow_set};
pub use problem::{Problem, ProblemKind};
pub use reach::{RANGE_MARGIN, flame_light_offset, lean_light_shift, raised_light};
pub use tag::{
    FLAME_REACH, ember_cull_extents, ember_reach, flame_cull_extents, pack_flame_tag, set_flame_brightness,
    unpack_flame_tag,
};
pub use wind::{GUST_INTENSITY, GUST_TERMS, GUST_TIMESCALE, gust_unit, gust_velocity};
