//! A fire light that moves, and a light that takes hold when it is lit.
//!
//! [`flicker_lights`] covers both [`PointLight`] and [`SpotLight`], so a wall sconce flickers the
//! same way a brazier does: it is the same fire.

use bevy::light::{PointLight, SpotLight};
use bevy::prelude::*;
use bevy::time::Time;

use crate::LightsSystems;
use crate::chain_sets;
use crate::model::{KINDLE_SECONDS, flicker_factor, kindle_gain};

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
/// `base · flicker_factor(t, phase) · kindle_gain(…)`: see [`flicker_factor`]. Put it on a
/// [`PointLight`] or a [`SpotLight`]. Two fires given different phases never fall into step.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Reflect)]
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
type PointLightFrame<'w, 's> = (
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
        modulate(entity, &mut *light, *flicker, kindle, seen, visibility, watched, t, dt, &mut kindled);
    }
    for (entity, mut light, flicker, kindle, seen, visibility) in &mut spots {
        modulate(entity, &mut *light, *flicker, kindle, seen, visibility, watched, t, dt, &mut kindled);
    }
}

/// One light's frame: hear it if it has just been lit, advance its kindle, and write its intensity.
///
/// The one place either light type's intensity is written, so a point light and a spot light cannot
/// drift apart.
#[allow(clippy::too_many_arguments)]
fn modulate<L: Emits>(
    entity: Entity,
    light: &mut L,
    flicker: Flicker,
    kindle: Option<Mut<Kindle>>,
    seen: Option<&ViewVisibility>,
    visibility: Option<&Visibility>,
    watched: bool,
    t: f32,
    dt: f32,
    kindled: &mut MessageWriter<Kindled>,
) {
    let gain = match kindle {
        Some(mut kindle) => {
            if kindle.elapsed == 0.0 {
                kindled.write(Kindled { entity });
            }
            if kindle.elapsed < KINDLE_SECONDS {
                kindle.elapsed += dt;
            }
            kindle_gain(kindle.elapsed)
        }
        None => 1.0,
    };
    // A light the host put out is not written; one that is merely culled by a camera is not written
    // either, because nothing is extracted for it. Both checks are skipped entirely with no camera,
    // where `ViewVisibility` says false for everything and culling does not apply.
    if visibility.is_some_and(|visibility| *visibility == Visibility::Hidden) {
        return;
    }
    if watched && seen.is_some_and(|seen| !seen.get()) {
        return;
    }
    *light.intensity() = flicker.base * flicker_factor(t, flicker.phase) * gain;
}
