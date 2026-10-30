//! Practical lights for Bevy: the lights a scene's props actually carry.
//!
//! A world can have thousands of torches and only afford a dozen real lights. This crate is the
//! three pieces that make that look right:
//!
//! - [`Flicker`] and [`Kindle`] — [`FlickerPlugin`] modulates a point or spot light around its base
//!   intensity with the doubled sine a fire's light has, and flares a light up over
//!   [`KINDLE_SECONDS`] when it is first lit instead of switching it on. [`Kindled`] is the message
//!   a host hears, for the sound of a lamp catching.
//! - [`LightPool`] — [`LightPoolPlugin`] keeps a fixed number of real lights and hands them to the
//!   [`PoolCandidate`]s nearest the host's focus entity every frame; the rest of the candidates are
//!   virtual. The lights it spawns carry [`PoolLight`], so a host can style them (a fire's palette
//!   recolours them, its flames are not drawn on them).
//! - [`FlameMaterial`], [`FlameSite`] — [`FlamePlugin`] draws a billboarded, additive shader flame
//!   standing on every [`FlameSite`], out of one material and one draw per flame: `src/flame.wesl`.
//!
//! Under those three is the cinematographer's half, [`model::cinematography`]: [`key_to_fill`] and
//! [`stops`] for a scene's key against its fill, [`fill_for`] for the sky that makes a night,
//! [`fire_lumens`] for the size of a pool, and [`blackbody_colour`] and [`white_balanced`] for a
//! light's colour from the temperature it burns at. They touch no light: the same numbers a director
//! of photography sets before a lamp is hung, for a host to size and colour its practicals with.
//!
//! Every plugin is independent; adding some and not others is supported, and
//! [`LightPoolPlugin`] warns (it does not panic) when [`FlickerPlugin`] is missing.
//!
//! ```no_run
//! # use bevy::prelude::*;
//! # use bevy_lights::*;
//! # #[derive(Component)]
//! # struct MyFocus;
//! # fn main() {
//! App::new()
//!     .add_plugins((FlickerPlugin, LightPoolPlugin::<MyFocus>::default(), FlamePlugin))
//!     .add_systems(Startup, |mut commands: Commands| {
//!         // A brazier: `diameter` is its fuel bed, which sizes the flicker through the puffing law
//!         // (`f = 1.5/√D` Hz, so 0.34 m is 2.6 Hz). Leave it `None` for the un-sized rate.
//!         commands.spawn((PointLight { intensity: 60_000.0, range: 12.0, ..default() },
//!             Flicker { base: 60_000.0, phase: 0.0, diameter: Some(0.34), ..default() },
//!             Kindle::default(),
//!             // `diameter` matches the `Flicker`'s, so the drawn flame breathes with the light.
//!             FlameSite { height: 0.9, phase: 0.0, kindle: 0.0, diameter: Some(0.34),
//!                 depth: FLICKER_DEPTH, depth_fade: false, fire: None }));
//!         commands.spawn((MyFocus, Transform::default()));
//!         // A carried fire: one entity holds the `FlameSite` and the `PoolCandidate`, so the light it
//!         // is handed sits in the flame the host can see. `offset` is the flame's own centre, measured
//!         // from the site's origin, and the lumens are sized for the height the light will sit at.
//!         let height = 0.35;
//!         commands.spawn((PoolCandidate {
//!                 offset: Vec3::from(flame_light_offset(height)),
//!                 intensity: 8_000.0,
//!                 ..default()
//!             },
//!             FlameSite { height, phase: 9.0, kindle: 0.0, diameter: None, depth: FLICKER_DEPTH,
//!                 depth_fade: false, fire: None },
//!             Transform::from_xyz(4.0, 0.0, 2.0)));
//!     });
//! # }
//! ```
//!
//! The `App` above is not run by the doctest: `FlamePlugin` adds a `MaterialPlugin`, which needs a
//! render app, so running it would need a GPU. `tests/flicker.rs` and `tests/pool.rs` are the
//! behavioural proof for the other two plugins, and `src/flame.rs`'s own tests cover the flame
//! dressing headlessly.
//!
//! # What this crate does not own
//!
//! The sun, the moon, the sky, exposure and weather belong to a sky crate; a gameplay illuminance
//! grid, to a light-grid crate; and which lamp is lit and when, to the game or to a society crate.
//! This crate owns the practicals: their flicker, their kindling, the pool that decides which of
//! them are real, and the flames drawn on them. It never adds a directional light and never meters
//! exposure.
//!
//! # Units
//!
//! Intensities are Bevy's: lumens for a [`PointLight`] and a [`SpotLight`]. Distances are the
//! host's world units. Nothing in this crate converts, so a host in feet is in feet throughout.

#![warn(missing_docs)]

pub mod model;

mod embers;
mod flame;
mod flicker;
mod kind;
mod pool;
mod report;

pub use embers::{FlameEmbers, MAX_EMBER_RATE};
pub use flame::{Flame, FlameGust, FlameLook, FlameMaterial, FlamePlugin, FlameSite, FlameWind};
pub use flicker::{Flicker, FlickerPlugin, Kindle, Kindled, Snuff, Snuffed};
pub use kind::{FireKind, UnknownFireKind};
pub use model::{
    CALM_FLICKER_DEPTH, DEFAULT_FIRE_DIAMETER, EMBER_AT, FADE_SECONDS, FLAME_TILT_LIMIT, FLICKER_DEPTH,
    HIGH_KEY, HYSTERESIS, KINDLE_SECONDS, LOW_KEY, MAX_FLICKER_DEPTH, MOON_KEY, Problem, ProblemKind,
    RANGE_MARGIN, SNUFF_SECONDS, WHIPPING_FLICKER_DEPTH, blackbody_colour, blackbody_luminance, char_field,
    char_radiance, coal_radiance, fade_factor, fill_for, fire_lumens, fire_lux, fire_reach,
    flame_light_offset, flame_tilt, flicker_depth, flicker_factor, flicker_mean, flicker_swing, ground_lux,
    ground_ramp, ground_reach, hand_off, key_to_fill, kindle_gain, lean_light_shift, pack_flame_tag,
    preferred, raised_light, snuff_gain, stops, unpack_flame_tag, white_balanced,
};
pub use pool::{LightPool, LightPoolPlugin, PoolBudget, PoolCandidate, PoolLight, Rank};

use bevy::prelude::*;

/// The order the crate's systems run in `Update`.
///
/// [`LightsSystems::Pool`] hands the real lights out before [`LightsSystems::Flicker`] modulates
/// them, so a light handed to a new candidate flickers at its new base in the very same frame; and
/// [`LightsSystems::Flame`] dresses sites after both, so a flame is drawn on the frame its site
/// appears.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LightsSystems {
    /// The light pool: spawning, resizing and assigning the real lights.
    Pool,
    /// `Flicker` and `Kindle`, on every point and spot light that carries them.
    Flicker,
    /// The shader flames: dressing new sites and ticking their shared clock.
    Flame,
}

/// Chains the crate's sets, and does it idempotently, so each plugin may call it.
fn chain_sets(app: &mut App) {
    app.configure_sets(Update, (LightsSystems::Pool, LightsSystems::Flicker, LightsSystems::Flame).chain());
}
