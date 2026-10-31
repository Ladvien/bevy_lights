//! A fire light that moves, and a light that takes hold when it is lit.
//!
//! [`flicker_lights`] covers both [`PointLight`] and [`SpotLight`], so a wall sconce flickers the
//! same way a brazier does: it is the same fire.

use bevy::camera::visibility::VisibilitySystems;
use bevy::light::{PointLight, SpotLight};
use bevy::prelude::*;
use bevy::time::Time;

use crate::LightsSystems;
use crate::chain_sets;
use crate::model::{
    FLICKER_DEPTH, KINDLE_SECONDS, SNUFF_SECONDS, flicker_depth, flicker_mean, flicker_swing, kindle_gain,
    snuff_gain,
};

/// [`FlickerPlugin`]: modulates every [`Flicker`] light and advances every [`Kindle`].
pub struct FlickerPlugin;

impl Plugin for FlickerPlugin {
    fn build(&self, app: &mut App) {
        chain_sets(app);
        app.add_message::<Kindled>()
            .add_message::<Snuffed>()
            .register_type::<Flicker>()
            .register_type::<Kindle>()
            .register_type::<Snuff>()
            .register_type::<crate::FireKind>()
            .add_systems(Update, flicker_lights.in_set(LightsSystems::Flicker))
            // After every system that decides a light's `ViewVisibility` for the frame, and before the
            // renderer extracts the lights (which is after `PostUpdate`).
            .add_systems(
                PostUpdate,
                write_lights_coming_into_view
                    .after(VisibilitySystems::CheckVisibility)
                    .after(VisibilitySystems::MarkNewlyHiddenEntitiesInvisible),
            );
        crate::report::watch::<Flicker>(app);
        crate::report::watch::<Snuff>(app);
        crate::report::watch::<Kindle>(app);
    }
}

/// A light's mean intensity and the clock phase its flicker runs at.
///
/// **Spell it `Flicker { base, phase, ..default() }`.** A field added later (as `diameter` and `depth`
/// were) is then absorbed by the default instead of breaking the host. The struct is deliberately not
/// `#[non_exhaustive]`: that would forbid a literal outside this crate even with `..default()`.
///
/// The light's intensity is written every frame (while it is visible) as
/// `base · flicker_swing(diameter, depth, t, phase) · kindle_gain(…)`: see [`flicker_swing`]. Put it on a
/// [`PointLight`] or a [`SpotLight`]. Two fires given different phases never fall into step, and the
/// `diameter` here is the same number the flame drawn on the fire should be given, so the two breathe
/// together.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
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
    /// lantern is [`CALM_FLICKER_DEPTH`](crate::CALM_FLICKER_DEPTH) (`0.05`); a gusting torch
    /// [`WHIPPING_FLICKER_DEPTH`](crate::WHIPPING_FLICKER_DEPTH) (`0.3`). `base` is the *peak*: to hit a
    /// budget on average, set it to `budget / `[`flicker_mean`](crate::flicker_mean)`(depth)`, or build the fire with [`Flicker::at_mean`], and change it later with [`Flicker::set_depth`] (keeps the mean). Clamped to
    /// `0..=`[`MAX_FLICKER_DEPTH`](crate::MAX_FLICKER_DEPTH). **Set the same
    /// value on the [`FlameSite`](crate::FlameSite) drawn on the fire**, so the flame and the ground
    /// under it dip together.
    pub depth: f32,
}

impl Default for Flicker {
    fn default() -> Self {
        Self { base: 0.0, phase: 0.0, diameter: None, depth: FLICKER_DEPTH }
    }
}

impl Flicker {
    /// A fire whose budget is written at its **mean**: `base` is solved as
    /// `mean / `[`flicker_mean`]`(depth)`, so the light averages `mean` at any `depth`. Set the other
    /// fields with struct update: `Flicker { phase: 1.3, ..Flicker::at_mean(lumens, 0.3) }`.
    ///
    /// `depth` is stored clamped, as [`flicker_depth`] reads it.
    ///
    /// ```
    /// # use bevy_lights::*;
    /// let fire = Flicker::at_mean(1000.0, WHIPPING_FLICKER_DEPTH);
    /// assert!((fire.mean() - 1000.0).abs() < 1e-3);
    /// assert!(fire.base > 1000.0); // the peak sits above the mean
    /// ```
    pub fn at_mean(mean: f32, depth: f32) -> Self {
        let depth = flicker_depth(depth);
        Self { base: mean / flicker_mean(depth), depth, ..Self::default() }
    }

    /// The light's mean intensity before the kindle: `base · `[`flicker_mean`]`(depth)`.
    pub fn mean(&self) -> f32 {
        self.base * flicker_mean(self.depth)
    }

    /// Writes `base` so the light averages `mean` at the current `depth`. A system that rewrites a
    /// light's lumens every frame calls this and cannot forget the depth.
    pub fn set_mean(&mut self, mean: f32) {
        self.base = mean / flicker_mean(self.depth);
    }

    /// Changes `depth` and **keeps the mean**: `base` is re-solved so the fire breathes differently
    /// and gives the same light on average. A `base` of `0.0` (a put-out fire) stays `0.0`. `depth`
    /// is stored clamped, as [`flicker_depth`] reads it.
    pub fn set_depth(&mut self, depth: f32) {
        let mean = self.mean();
        self.depth = flicker_depth(depth);
        self.set_mean(mean);
    }

    /// The lumens this fire gives at virtual time `t` seconds, `kindle` seconds after it was lit
    /// (`None`: already burning). This is what `flicker_lights` writes each frame before any [`Snuff`]
    /// multiplies it by [`snuff_gain`], so an
    /// editor's timeline scrubber or a settings preview can draw a fire's curve with no app running.
    pub fn intensity_at(&self, t: f32, kindle: Option<f32>) -> f32 {
        let gain = kindle.map_or(1.0, kindle_gain);
        self.base * flicker_swing(self.diameter, self.depth, t, self.phase) * gain
    }
}

impl Flicker {
    /// What is unusable in these settings, field by field; empty when they are fine.
    ///
    /// The crate never refuses a value, so a `NaN` phase or a negative `base` still runs (and writes a
    /// bad intensity). This is for a host that reads a `Flicker` from a file or a panel. A `depth`
    /// outside `0..=`[`MAX_FLICKER_DEPTH`](crate::MAX_FLICKER_DEPTH) is reported although it is
    /// clamped, because the flame drawn on the fire reads the same number.
    pub fn problems(&self) -> Vec<crate::Problem> {
        use crate::model::problem::{finite, non_negative, positive, within};
        let mut out = Vec::new();
        non_negative(&mut out, "base", self.base);
        finite(&mut out, "phase", self.phase);
        if let Some(d) = self.diameter {
            positive(&mut out, "diameter", d);
        }
        within(&mut out, "depth", self.depth, 0.0, crate::MAX_FLICKER_DEPTH);
        out
    }
}

/// A light that is taking hold: its intensity flares up over [`KINDLE_SECONDS`] instead of switching
/// on.
///
/// Insert [`Kindle::default()`] to light a fire, and insert it again to re-light one that has gone
/// out. The crate advances [`Kindle::elapsed`] and writes [`Kindled`] on the first frame it is
/// present; a host never advances it itself.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
#[reflect(Component)]
pub struct Kindle {
    /// Seconds since the fire was lit. The crate stops advancing it at [`KINDLE_SECONDS`]; it keeps
    /// counting only while the light is not yet fully kindled.
    pub elapsed: f32,
}

impl Kindle {
    /// What is unusable in this state, field by field; empty when it is fine. See
    /// [`Flicker::problems`](crate::Flicker::problems). A `Kindle` read from a saved scene with a
    /// negative or `NaN` `elapsed` would write a bad gain.
    pub fn problems(&self) -> Vec<crate::Problem> {
        let mut out = Vec::new();
        crate::model::problem::non_negative(&mut out, "elapsed", self.elapsed);
        out
    }
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

/// A light going out: its intensity falls to nothing over [`SNUFF_SECONDS`] instead of switching off.
///
/// Insert [`Snuff::default()`] to put a fire out (a dousing, a torch burnt down, rain on a lantern). The
/// crate advances [`Snuff::elapsed`] on virtual time and multiplies the light's intensity by
/// [`snuff_gain`]; a [`FlameSite`](crate::FlameSite) whose [`fire`](crate::FlameSite::fire) names this
/// light is drawn at the same factor, so light and flame go out together. At the end the intensity is
/// exactly 0 and [`Snuffed`] is written once; the fire then stays out until the host removes `Snuff`
/// (and inserts [`Kindle::default()`] to light it again). With both present, `Snuff` wins. A linked flame that
/// has a [`FlameSmoke`](crate::FlameSmoke) leaves a trail of smoke that lifts off and thins for
/// [`SMOKE_TAIL_SECONDS`](crate::SMOKE_TAIL_SECONDS) after the flame is gone.
///
/// It acts on a [`Flicker`] light, or on a [`PoolCandidate`](crate::PoolCandidate) (see there): the pool lends the
/// candidate's light at the same factor.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
#[reflect(Component)]
pub struct Snuff {
    /// Seconds since the fire began to go out. The crate stops advancing it at [`SNUFF_SECONDS`]. A
    /// value already at [`SNUFF_SECONDS`] is a fire that is out: it is not announced again.
    pub elapsed: f32,
}

impl Snuff {
    /// What is unusable in this state, field by field; empty when it is fine. See
    /// [`Flicker::problems`](crate::Flicker::problems).
    pub fn problems(&self) -> Vec<crate::Problem> {
        let mut out = Vec::new();
        crate::model::problem::non_negative(&mut out, "elapsed", self.elapsed);
        out
    }
}

/// A fire went out this frame: its [`Snuff`] ran its course, so the host can play the hiss, spawn the
/// smoke and despawn or hide the prop.
///
/// Written once per [`Snuff`], on the frame its `elapsed` reaches [`SNUFF_SECONDS`].
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Snuffed {
    /// The light that went out.
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
    Option<&'w mut Snuff>,
    Option<&'w ViewVisibility>,
    Option<Ref<'w, Visibility>>,
);

/// A spot light's frame of work: the same shape as [`PointLightFrame`].
type SpotLightFrame<'w> = (
    Entity,
    &'w mut SpotLight,
    &'w Flicker,
    Option<&'w mut Kindle>,
    Option<&'w mut Snuff>,
    Option<&'w ViewVisibility>,
    Option<Ref<'w, Visibility>>,
);

/// Writes every [`Flicker`] light's intensity, and advances every [`Kindle`].
///
/// Runs on virtual time (`Time`), so a paused world's fires hold still while the flames drawn on
/// them keep moving — the same split the crate inherits from the game it was extracted from.
///
/// A light the camera does not see (`ViewVisibility` false) keeps its last intensity: nothing is
/// re-extracted for it, so writing it would cost a change-detection edge for no pixels. Its
/// [`Kindle`] still advances, so it has already flared by the time it comes into view. The frame it
/// comes into view it is written by [`write_lights_coming_into_view`].
fn flicker_lights(
    time: Res<Time>,
    // `Without` on each other's light type, so the two queries are provably disjoint: both ask for
    // `&mut Kindle`, and Bevy cannot know an entity is not both a point light and a spot light.
    mut points: Query<PointLightFrame, Without<SpotLight>>,
    mut spots: Query<SpotLightFrame, Without<PointLight>>,
    cameras: Query<(), With<Camera>>,
    mut kindled: MessageWriter<Kindled>,
    mut snuffed: MessageWriter<Snuffed>,
) {
    let t = time.elapsed_secs();
    let dt = time.delta_secs();
    // Whether anything is watching. `ViewVisibility` exists on every light (it is required by
    // `Visibility`) and is **false with no camera**, so without this an app with no window would
    // never write an intensity at all: a headless bench, a test, a server-side simulation. A light is
    // only culled when there is a camera to cull it from.
    let watched = !cameras.is_empty();
    for (entity, mut light, flicker, kindle, snuff, seen, visibility) in &mut points {
        let hidden = is_hidden(visibility, seen, watched);
        let heard = modulate(&mut *light, *flicker, kindle, snuff, hidden, t, dt);
        announce(entity, heard, &mut kindled, &mut snuffed);
    }
    for (entity, mut light, flicker, kindle, snuff, seen, visibility) in &mut spots {
        let hidden = is_hidden(visibility, seen, watched);
        let heard = modulate(&mut *light, *flicker, kindle, snuff, hidden, t, dt);
        announce(entity, heard, &mut kindled, &mut snuffed);
    }
}

/// What one light's frame told the host.
#[derive(Clone, Copy, Default)]
struct Heard {
    /// The fire was lit this frame.
    lit: bool,
    /// The fire went out this frame.
    out: bool,
}

fn announce(
    entity: Entity,
    heard: Heard,
    kindled: &mut MessageWriter<Kindled>,
    snuffed: &mut MessageWriter<Snuffed>,
) {
    if heard.lit {
        kindled.write(Kindled { entity });
    }
    if heard.out {
        snuffed.write(Snuffed { entity });
    }
}

/// Whether a light's intensity is left alone this frame.
///
/// A light the host put out (`Visibility::Hidden`) is not written, and neither is one a camera culled,
/// because nothing is extracted for it. The culling test is skipped with no camera, where
/// `ViewVisibility` says false for everything and there is nothing to cull from.
///
/// A light **shown this frame** (its `Visibility` changed, not just added, to something other than
/// `Hidden`) is written even though its `ViewVisibility` is still last frame's false: `PostUpdate`
/// recomputes it and the renderer extracts the light this same frame, so a skipped write drew it for
/// one frame at whatever intensity it was left at. Measured on the pool: a reserve light taking a
/// cross-fade came on at 112 lm, the tail of its last fade, instead of 70 000.
fn is_hidden(visibility: Option<Ref<Visibility>>, seen: Option<&ViewVisibility>, watched: bool) -> bool {
    let Some(visibility) = visibility else {
        return watched && seen.is_some_and(|seen| !seen.get());
    };
    if *visibility == Visibility::Hidden {
        return true;
    }
    let shown_now = visibility.is_changed() && !visibility.is_added();
    watched && !shown_now && seen.is_some_and(|seen| !seen.get())
}

/// One light's frame: advance its kindle and snuff and write its intensity. Answers what the host should
/// hear of it.
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
///
/// A snuff is announced on the frame it *reaches* [`SNUFF_SECONDS`], so a `Snuff` already there (a fire
/// loaded from a scene as out) says nothing, and a zero-delta frame cannot announce it twice.
fn modulate<L: Emits>(
    light: &mut L,
    flicker: Flicker,
    kindle: Option<Mut<Kindle>>,
    snuff: Option<Mut<Snuff>>,
    hidden: bool,
    t: f32,
    dt: f32,
) -> Heard {
    let (elapsed, out, heard) = advance_fire(kindle, snuff, dt);
    if !hidden {
        *light.intensity() = level(flicker, elapsed, out, t);
    }
    heard
}

/// What a fire gives at virtual time `t`: its flicker, its kindle's flare (`elapsed`, `None` when it has none)
/// and its snuff's gain (`out`, `None` when it has none). The one formula [`modulate`] writes every frame and
/// [`write_lights_coming_into_view`] writes the frame a light is first drawn.
fn level(flicker: Flicker, elapsed: Option<f32>, out: Option<f32>, t: f32) -> f32 {
    flicker.intensity_at(t, elapsed) * out.unwrap_or(1.0)
}

/// A point light's state, read in `PostUpdate`: the light, its fire, and whether a camera sees it.
type PointLightSeen<'w> =
    (&'w mut PointLight, &'w Flicker, Option<&'w Kindle>, Option<&'w Snuff>, &'w ViewVisibility);

/// A spot light's state, read in `PostUpdate`: the same shape as [`PointLightSeen`].
type SpotLightSeen<'w> =
    (&'w mut SpotLight, &'w Flicker, Option<&'w Kindle>, Option<&'w Snuff>, &'w ViewVisibility);

/// Writes the intensity of every light a camera sees **for the first time** this frame (its
/// `ViewVisibility` has just turned true: the camera moved over it, or it was spawned in view), before the
/// renderer extracts it.
///
/// [`flicker_lights`] leaves a culled light alone, and `ViewVisibility` is recomputed in `PostUpdate`, after
/// it: the light the camera has just turned to is still "culled" when `Update` runs and is extracted this same
/// frame, so it was drawn for one frame at whatever intensity it was last written, which for a pool light
/// handed to a new owner while out of view is the old owner's. Measured on the courtyard with two moving
/// cameras: one frame of a torch's light at 75 903 lm where the new owner's fade-in was at 104, a 5 DN jump of
/// a half-frame's mean luminance. This system is the other half of the culling: a culled light costs nothing,
/// and the frame it is seen it is as bright as its fire is.
///
/// It matches only a `ViewVisibility` that changed this frame, so a light that stays in view or out of it
/// costs a change-detection check and nothing else.
fn write_lights_coming_into_view(
    time: Res<Time>,
    mut points: Query<PointLightSeen, (Without<SpotLight>, Changed<ViewVisibility>)>,
    mut spots: Query<SpotLightSeen, (Without<PointLight>, Changed<ViewVisibility>)>,
) {
    let t = time.elapsed_secs();
    let level_of = |flicker: &Flicker, kindle: Option<&Kindle>, snuff: Option<&Snuff>| {
        level(*flicker, kindle.map(|kindle| kindle.elapsed), snuff.map(|snuff| snuff_gain(snuff.elapsed)), t)
    };
    for (mut light, flicker, kindle, snuff, seen) in &mut points {
        if seen.get() {
            light.intensity = level_of(flicker, kindle, snuff);
        }
    }
    for (mut light, flicker, kindle, snuff, seen) in &mut spots {
        if seen.get() {
            light.intensity = level_of(flicker, kindle, snuff);
        }
    }
}

/// Advances a fire's [`Kindle`] and [`Snuff`] by `dt`. Answers the kindle's `elapsed` and the snuff's
/// [`snuff_gain`] (each `None` when the fire has none), and what the host should hear.
fn advance_fire(
    kindle: Option<Mut<Kindle>>,
    snuff: Option<Mut<Snuff>>,
    dt: f32,
) -> (Option<f32>, Option<f32>, Heard) {
    let mut heard = Heard::default();
    let elapsed = kindle.map(|mut kindle| {
        heard.lit = kindle.elapsed == 0.0;
        if kindle.elapsed < KINDLE_SECONDS {
            kindle.elapsed = (kindle.elapsed + dt).max(f32::MIN_POSITIVE);
        }
        kindle.elapsed
    });
    let out = snuff.map(|mut snuff| {
        if snuff.elapsed < SNUFF_SECONDS {
            snuff.elapsed = (snuff.elapsed + dt).min(SNUFF_SECONDS);
            heard.out = snuff.elapsed >= SNUFF_SECONDS;
        }
        snuff_gain(snuff.elapsed)
    });
    (elapsed, out, heard)
}

/// A pooled fire's state: its entity and the components [`advance_candidate_fires`] steps.
type CandidateFire = (Entity, Option<&'static mut Kindle>, Option<&'static mut Snuff>);

/// Which entities [`advance_candidate_fires`] visits: a candidate with a kindle or a snuff and no light of its own.
type CandidateFireFilter = (With<crate::PoolCandidate>, Without<Flicker>, Or<(With<Kindle>, With<Snuff>)>);

/// Advances the [`Kindle`] and [`Snuff`] of every [`PoolCandidate`](crate::PoolCandidate) that is not itself a
/// [`Flicker`] light, and announces them. [`flicker_lights`] does this for a light; a pooled fire has none of its
/// own, so the pool's plugin runs this one, before it assigns, so the lent light and the flame linked to the
/// candidate read the same frame's state.
///
/// Matches only entities that carry a `Kindle` or a `Snuff`, so a pool whose candidates have neither pays
/// nothing for it.
pub(crate) fn advance_candidate_fires(
    time: Res<Time>,
    mut fires: Query<CandidateFire, CandidateFireFilter>,
    mut kindled: MessageWriter<Kindled>,
    mut snuffed: MessageWriter<Snuffed>,
) {
    let dt = time.delta_secs();
    for (entity, kindle, snuff) in &mut fires {
        let (.., heard) = advance_fire(kindle, snuff, dt);
        announce(entity, heard, &mut kindled, &mut snuffed);
    }
}
