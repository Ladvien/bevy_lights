//! The engine-free half of the crate: plain `f32` maths with no `bevy` in it.
//!
//! `tests/leaf.rs` enforces that. The point is that the shapes of these curves — how a flame
//! flickers, how a light takes hold, how far a raised light reaches, how a flame's height and phase
//! are packed — are testable without an app, a window or a GPU, and are the part of the crate a
//! host is most likely to want to reuse in its own systems.

pub mod fire;
pub mod reach;
pub mod tag;

pub use fire::{KINDLE_SECONDS, flicker_factor, kindle_gain};
pub use reach::{RANGE_MARGIN, raised_light};
pub use tag::{pack_flame_tag, unpack_flame_tag};
