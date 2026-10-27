//! A fire light that moves, and a light that takes hold when it is lit.
//!
//! [`flicker_lights`] covers both [`PointLight`] and [`SpotLight`], so a wall sconce flickers the
//! same way a brazier does: it is the same fire.

use bevy::light::{PointLight, SpotLight};
use bevy::prelude::*;
use bevy::time::Time;

use crate::LightsSystems;
use crate::chain_sets;
use crate::model::{FLICKER_DEPTH, KINDLE_SECONDS, flicker_swing, kindle_gain};

/// [`FlickerPlugin`]: modulates every [`Flicker`] light and advances every [`Kindle`].
pub struct FlickerPlugin;

impl Plugin for FlickerPlugin {
    fn build(&self, app: &mut App) {
        chain_sets(app);
        app.add_message::<Kindled>()
            .register_type::<Flicker>()
            .register_type::<Kindle>()
            .add_systems(Update, flicker_lights.in_set(LightsSystems::Flicker));
    }
}

/// A light's mean intensity and the clock phase its flicker runs at.
///
/// The light's intensity is written every frame (while it is visible) as
/// `base · flicker_swing(diameter, depth, t, phase) · kindle_gain(…)`: see [`flicker_swing`]. Put it on a
/// [`PointLight`] or a [`SpotLight`]. Two fires given different phases never fall into step, and the
/// `diameter` here is the same number the flame drawn on the fire should be given, so the two breathe
/// together.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[reflect(Component)]
pub struct Flicker {
    /// The light's mean intensity, in the same units as the light's own `intensity` (lumens).
    ///
    /// This is the number to write when a fire is dimmed or brightened: the flicker and the kindle
    /// are multipliers on it, so they never accumulate.
    pub base: f32,
    /// The flicker's clock phase, in radians. Any two fires with different phases are out of step;
    /// a common recipe is a multiple of the index (`slot as f32 * 1.3`).
    pub phase: f32,
    /// The fire's fuel-bed diameter, in the host's world units, when the host knows it: this sizes the
    /// flicker through the puffing law (`f = 1.5/√D` Hz), so a brazier moves faster than a pyre.
    ///
    /// `None` (the default) uses the crate's un-sized rate, which is a host's choice for a fire whose
    /// size is not modelled. `Some(0.34)` is a brazier's coal bed; `Some(4.0)` a bonfire.
    pub diameter: Option<f32>,
    /// How far the light dips below its peak: the multiplier runs `1 - 2·depth ..= 1` with mean
    /// `1 - depth`, so [`base`](Self::base) is always the fire's **brightest** moment and a deeper
    /// flicker is a darker trough.
    ///
    /// [`FLICKER_DEPTH`] (`0.15`, the default) is the crate's inherited flicker, `0.70..=1.00`. A calm
    /// lantern is `0.03`; a gusting torch `0.3`. Clamped to `0..=`[`MAX_FLICKER_DEPTH`](crate::MAX_FLICKER_DEPTH). **Set the same
    /// value on the [`FlameSite`](crate::FlameSite) drawn on the fire**, so the flame and the ground
    /// under it dip together.
    pub depth: f32,
}

impl Default for Flicker {
    fn default() -> Self {
        Self { base: 0.0, phase: 0.0, diameter: None, depth: FLICKER_DEPTH }
    }
}

/// A light that is taking hold: its intensity flares up over [`KINDLE_SECONDS`] instead of switching
/// on.
///
/// Insert [`Kindle::default()`] to light a fire, and insert it again to re-light one that has gone
/// out. The crate advances [`Kindle::elapsed`] and writes [`Kindled`] on the first frame it is
/// present; a host never advances it itself.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[reflect(Component)]
pub struct Kindle {
    /// Seconds since the fire was lit. The crate stops advancing it at [`KINDLE_SECONDS`]; it keeps
    /// counting only while the light is not yet fully kindled.
    pub elapsed: f32,
}

/// A fire took hold this frame: its [`Kindle`] began, so the host can play the sound of it.
///
/// Written once per [`Kindle`], on the frame the component appears (including a re-inserted one).
/// A host that wants the sound's position reads the entity's `GlobalTransform` this frame.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Kindled {
    /// The light that began to kindle.
    pub entity: Entity,
}

/// A light with a single intensity: what [`flicker_lights`] needs of the types it modulates.
///
/// Implemented for [`PointLight`] and [`SpotLight`] only. The trait is private, so adding a third
/// light type is a change here and nowhere else.
trait Emits: Component {
    fn intensity(&mut self) -> &mut f32;
}

impl Emits for PointLight {
    fn intensity(&mut self) -> &mut f32 {
        &mut self.intensity
    }
}

impl Emits for SpotLight {
    fn intensity(&mut self) -> &mut f32 {
        &mut self.intensity
    }
}

/// A point light's frame of work: the entity, the light, and the components that shape its frame.
type PointLightFrame<'w> = (
    Entity,
    &'w mut PointLight,
    &'w Flicker,
    Option<&'w mut Kindle>,
    Option<&'w ViewVisibility>,
    Option<&'w Visibility>,
);

/// A spot light's frame of work: the same shape as [`PointLightFrame`].
type SpotLightFrame<'w> = (
    Entity,
    &'w mut SpotLight,
    &'w Flicker,
    Option<&'w mut Kindle>,
    Option<&'w ViewVisibility>,
    Option<&'w Visibility>,
);

/// Writes every [`Flicker`] light's intensity, and advances every [`Kindle`].
///
/// Runs on virtual time (`Time`), so a paused world's fires hold still while the flames drawn on
/// them keep moving — the same split the crate inherits from the game it was extracted from.
///
/// A light the camera does not see (`ViewVisibility` false) keeps its last intensity: nothing is
/// re-extracted for it, so writing it would cost a change-detection edge for no pixels. Its
/// [`Kindle`] still advances, so it has already flared by the time it comes into view.
fn flicker_lights(
    time: Res<Time>,
    // `Without` on each other's light type, so the two queries are provably disjoint: both ask for
    // `&mut Kindle`, and Bevy cannot know an entity is not both a point light and a spot light.
    mut points: Query<PointLightFrame, Without<SpotLight>>,
    mut spots: Query<SpotLightFrame, Without<PointLight>>,
    cameras: Query<(), With<Camera>>,
    mut kindled: MessageWriter<Kindled>,
) {
    let t = time.elapsed_secs();
    let dt = time.delta_secs();
    // Whether anything is watching. `ViewVisibility` exists on every light (it is required by
    // `Visibility`) and is **false with no camera**, so without this an app with no window would
    // never write an intensity at all: a headless bench, a test, a server-side simulation. A light is
    // only culled when there is a camera to cull it from.
    let watched = !cameras.is_empty();
    for (entity, mut light, flicker, kindle, seen, visibility) in &mut points {
        let hidden = is_hidden(visibility, seen, watched);
        if modulate(&mut *light, *flicker, kindle, hidden, t, dt) {
            kindled.write(Kindled { entity });
        }
    }
    for (entity, mut light, flicker, kindle, seen, visibility) in &mut spots {
        let hidden = is_hidden(visibility, seen, watched);
        if modulate(&mut *light, *flicker, kindle, hidden, t, dt) {
            kindled.write(Kindled { entity });
        }
    }
}

/// Whether a light's intensity is left alone this frame.
///
/// A light the host put out (`Visibility::Hidden`) is not written, and neither is one a camera culled,
/// because nothing is extracted for it. The culling test is skipped with no camera, where
/// `ViewVisibility` says false for everything and there is nothing to cull from.
fn is_hidden(visibility: Option<&Visibility>, seen: Option<&ViewVisibility>, watched: bool) -> bool {
    visibility.is_some_and(|visibility| *visibility == Visibility::Hidden)
        || (watched && seen.is_some_and(|seen| !seen.get()))
}

/// One light's frame: advance its kindle and write its intensity. Answers whether the fire was lit
/// this frame, so the caller can announce it.
///
/// The one place either light type's intensity is written, so a point light and a spot light cannot
/// drift apart.
///
/// **A kindle that has been heard never reads exactly 0 again.** `Kindle::default()` (elapsed 0) is
/// how a host lights a fire, and `elapsed == 0.0` is how this system knows the fire has not been
/// announced. A frame whose delta is zero — the first frame of a fixed-step run — would otherwise
/// leave it at 0 and announce the same fire twice, so the advance is floored at the smallest positive
/// float. (Bevy's `Added<Kindle>` is no substitute: replacing a component leaves its added tick
/// alone, so a re-lit fire would never be heard again.)
fn modulate<L: Emits>(
    light: &mut L,
    flicker: Flicker,
    kindle: Option<Mut<Kindle>>,
    hidden: bool,
    t: f32,
    dt: f32,
) -> bool {
    let mut lit = false;
    let gain = match kindle {
        Some(mut kindle) => {
            lit = kindle.elapsed == 0.0;
            if kindle.elapsed < KINDLE_SECONDS {
                kindle.elapsed = (kindle.elapsed + dt).max(f32::MIN_POSITIVE);
            }
            kindle_gain(kindle.elapsed)
        }
        None => 1.0,
    };
    if !hidden {
        let swing = flicker_swing(flicker.diameter, flicker.depth, t, flicker.phase);
        *light.intensity() = flicker.base * swing * gain;
    }
    lit
}
