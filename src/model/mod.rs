//! The engine-free half of the crate: plain `f32` maths with no `bevy` in it.
//!
//! `tests/leaf.rs` enforces that. The point is that the shapes of these curves — how a flame
//! flickers, how far apart its puffs sit, how a light takes hold, how far a raised light reaches, how
//! a flame's height and phase are packed — are testable without an app, a window or a GPU, and are
//! the part of the crate a host is most likely to want to reuse in its own systems.

pub mod cinematography;
pub mod fire;
pub mod flame;
pub mod pool;
pub mod reach;
pub mod tag;

pub use cinematography::{
    HIGH_KEY, LOW_KEY, MOON_KEY, blackbody_colour, fill_for, fire_lumens, key_to_fill, stops, white_balanced,
};
pub use fire::{
    KINDLE_SECONDS, PUFFING_CONSTANT, flicker_factor, flicker_factor_of, flicker_rate, flicker_swing,
    kindle_gain,
};
pub use flame::{GRAVITY as PUFF_GRAVITY, flame_bulges, puff_wavelength};
pub use pool::{FADE_SECONDS, HYSTERESIS, fade_factor, fade_out_factor, hand_off, preferred};
pub use reach::{RANGE_MARGIN, raised_light};
pub use tag::{pack_flame_tag, set_flame_brightness, unpack_flame_tag};
