//! A budgeted pool of real lights for thousands of virtual ones.
//!
//! A town at night has a torch on every passer-by and a fire on every gate, and no GPU renders a
//! thousand shadow-casting point lights at 60 Hz. The trick every shipping game uses is to keep a
//! small number of real lights and hand them to whichever props are nearest the camera: the eye
//! cannot tell that the torch two streets away is only a flame, and Bevy only clusters the lights
//! that exist.
//!
//! [`LightPoolPlugin`] does exactly that, generically over the host's focus marker (a camera-focus
//! entity, a player, a squad) — the same shape `bevy_speech_bubbles` uses for its camera marker.
//! The budget is 16 lights against thousands of candidates, and the assignment is a sort, so the
//! work is `O(candidates log candidates)` a frame and nothing else.

use std::marker::PhantomData;

use bevy::light::PointLight;
use bevy::prelude::*;

use crate::flicker::{Flicker, FlickerPlugin, Kindle, Kindled, Snuff, Snuffed, advance_candidate_fires};
use crate::model::pool::{
    fade_factor, fade_out_factor, focus_quota, preferred, quota_cutoff, shadow_set, shared_value,
};
use crate::model::{
    DEFAULT_FIRE_DIAMETER, FLICKER_DEPTH, SNUFF_SECONDS, kindle_gain, lean_light_shift, snuff_gain,
};
use crate::{FADE_SECONDS, HYSTERESIS, LightsSystems, chain_sets};
use crate::{FlameSite, FlameWind};

/// [`LightPoolPlugin`]: keeps [`LightPool::size`] real lights and hands them to the nearest
/// [`PoolCandidate`]s.
///
/// Generic over `F`, the host's focus marker: the entity carrying `F` is the point the pool measures
/// distance from. Exactly one by default; with [`FocusRule::Shared`], one per viewer (a split screen's
/// players, a map camera beside the player's), and the pool's lights are shared between them. Add it
/// after [`FlickerPlugin`] — the pool's lights are [`Flicker`]
/// lights, and [`Plugin::finish`] reports a missing flicker plugin with an `error!` rather than
/// panicking or adding one itself.
pub struct LightPoolPlugin<F: Component> {
    _focus: PhantomData<fn() -> F>,
}

impl<F: Component> Default for LightPoolPlugin<F> {
    fn default() -> Self {
        Self { _focus: PhantomData }
    }
}

impl<F: Component> LightPoolPlugin<F> {
    /// A pool measuring from the host's `F`.
    pub fn new() -> Self {
        Self::default()
    }
}

impl<F: Component> Plugin for LightPoolPlugin<F> {
    fn build(&self, app: &mut App) {
        chain_sets(app);
        app.init_resource::<LightPool>()
            .register_type::<LightPool>()
            .register_type::<PoolCandidate>()
            .register_type::<PoolLight>()
            .init_resource::<PoolBudget>()
            .register_type::<PoolBudget>()
            // The messages are registered by `FlickerPlugin` too; `add_message` twice is harmless, and the
            // candidate fires write them even when that plugin is missing (which `finish` reports).
            .add_message::<Kindled>()
            .add_message::<Snuffed>()
            .add_systems(
                Update,
                (advance_candidate_fires, assign_pool::<F>).chain().in_set(LightsSystems::Pool),
            );
        crate::report::watch::<PoolCandidate>(app);
        crate::report::watch_resource::<LightPool>(app);
    }

    fn finish(&self, app: &mut App) {
        if !app.is_plugin_added::<FlickerPlugin>() {
            // Not added from here: a host that adds `FlickerPlugin` itself afterwards would panic on
            // the duplicate, and Bevy has no "add if missing". The pool's lights exist either way;
            // without the flicker they sit at intensity 0 and the pool stays dark.
            error!("bevy_lights: LightPoolPlugin needs FlickerPlugin; pool lights will stay dark");
        }
    }
}

/// The pool's budget and the light it spawns: how many real lights exist, how far a candidate may be
/// from the focus to get one, how brightly the pool burns, and the light every slot starts from.
///
/// [`LightPoolPlugin`] `init_resource`s this, so a host that inserts its own first overrides every
/// number. The defaults are a metre world's: 16 lights, a 60 m reach and the intensity of a small
/// fire.
///
/// With the `serde` feature it is `Serialize` and `Deserialize`, so an editor can save it with a
/// project and edit it live. `PointLight` itself has no serde derive, so [`LightPool::light`] is
/// written as the fields the pool copies onto its lights: all of them but `intensity`, which the pool
/// ignores and which reads back as 0. Bevy's `soft_shadows_enabled` is among them where Bevy's
/// `experimental_pbr_pcss` feature gives the light one; in a build without it, it is saved as `false`.
/// Every field is optional on the way in: a missing one takes its [`Default`].
#[derive(Resource, Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
#[reflect(Resource)]
pub struct LightPool {
    /// How many real lights exist. Shrinking it despawns the extra lights on the next frame; growing
    /// it spawns more.
    pub size: usize,
    /// How far from the focus a candidate may be, in world units, to be given a light: from the nearest
    /// focus, under [`FocusRule::Shared`].
    pub reach: f32,
    /// A dimmer shared by the whole pool, usually how lit the night is (0 puts every pool light
    /// out). Applied on top of each candidate's own intensity.
    pub gain: f32,
    /// The light every slot is made from. Every field of it but two is every pool light's, copied onto
    /// each light when the template **changes** — its `color`, `range`, `radius`, the shadow's quality
    /// (`shadow_depth_bias`, `shadow_normal_bias`, `shadow_map_near_z`), `contact_shadows_enabled`,
    /// `affects_lightmapped_mesh_diffuse`, and `soft_shadows_enabled` where Bevy has it — so a light the
    /// pool spawns later matches the ones already burning. The two that are not: `shadow_maps_enabled`
    /// turns the pool's shadows on (on as many lights as [`LightPool::max_shadowed`] allows), and
    /// `intensity` is ignored (each slot's is its candidate's, times [`LightPool::gain`]).
    ///
    /// Contact shadows need the host's camera to carry Bevy's `ContactShadows` (and, measured on the
    /// courtyard, TAA, or they dot a lit wall); soft shadows need Bevy's `experimental_pbr_pcss` feature
    /// and soften visibly only at a `radius` of metres, which also widens every highlight the light makes.
    /// Both are off in the default template.
    ///
    /// Changing `gain`, `size` or `reach` does not touch the slots' looks, so a host that tints its own
    /// fire lights (a palette recolouring every light that carries its fire marker) keeps its tint
    /// through the night's glide, which writes `gain` every frame. A change to the template does reset
    /// every light to it, colour included.
    #[cfg_attr(feature = "serde", serde(with = "template_serde"))]
    pub light: PointLight,
    /// How much better a challenger must be than the light a slot already holds before it takes it:
    /// `1.0` is "any improvement wins", the default [`HYSTERESIS`] 1.15 needs a challenger about 13 %
    /// better. This is the overlap rule that stops two props trading a light every frame.
    ///
    /// "Better" is [`LightPool::rank`]'s own quantity, so a pool ranking by illuminance needs a
    /// challenger to arrive 13 % brighter (at its distance) rather than merely be nearer — the rule and
    /// the rank cannot disagree about what a better light is.
    pub hysteresis: f32,
    /// How long, in seconds, a light takes to come back to full brightness after it changes owner.
    /// [`FADE_SECONDS`] by default; `0.0` snaps it back at once (the pre-fade behaviour).
    pub fade_seconds: f32,
    /// How the pool ranks its candidates: by the illuminance each would put on the subject (the default)
    /// or by distance alone.
    pub rank: Rank,
    /// Whether the pool serves one viewer or several: [`FocusRule::One`] (the default) needs exactly one
    /// entity carrying the host's focus marker, [`FocusRule::Shared`] takes any number and shares
    /// [`LightPool::size`] between them. See [`FocusRule`].
    pub focus: FocusRule,
    /// How many extra lights the pool keeps beyond the ones it assigns, so a hand-off can be a
    /// **cross-fade**: the departing owner's light fades out while the arriving one fades in, and the
    /// subject never brightens or darkens.
    ///
    /// The departing light keeps moving with the candidate it fades out — its position, its
    /// [`PoolCandidate::offset`] and the flame's lean in the wind, as the arriving light works them out —
    /// for as long as that candidate is in the pool, so both ends of a cross-fade of one light stand on one
    /// point; once the candidate has left it (hidden, out of reach, despawned) the light stays where it
    /// last stood.
    ///
    /// It is also the pool's **hand-off budget**: a reserve light is busy for the whole fade it carries,
    /// so the pool changes at most `reserve` lights' owners in a frame and the rest wait for the next
    /// frame. One by default, which cross-fades one hand-off per frame; raise it in a scene where many
    /// props move at once. `0` removes the cross-fade and the throttle together: a hand-off then dips
    /// rather than cross-fading, and every light that needs moving moves at once (iteration 2's
    /// behaviour).
    pub reserve: usize,
    /// At most how many of the pool's lights cast shadows, when [`LightPool::light`] has
    /// `shadow_maps_enabled`: the best-ranked holders by [`LightPool::rank`] — the closest and brightest
    /// fires, where a missing shadow shows — while the rest light without one. `None` (the default) is
    /// no cap: every pool light casts the template's shadow. With the template's flag off, none does.
    ///
    /// Every shadowed point light renders a six-sided shadow map every frame (about 0.6 ms a light on an
    /// RTX 3090 in the courtyard example), so this is the pool's **shadow budget**, kept apart from its
    /// light budget: sixteen lights can afford three shadows.
    ///
    /// A light that earns or loses its shadow because the ranking moved gains or loses it the way it
    /// changes owner: a reserve light takes the light as it was and fades it out while the slot fades
    /// back in with the new setting, so a shadow deepens over [`LightPool::fade_seconds`] instead of
    /// appearing. That shares the hand-off budget ([`LightPool::reserve`]): a shadow change waits for a
    /// free reserve, after the frame's owner changes, and for the light's own fade to finish; with
    /// `reserve: 0` it dips and returns, as a hand-off does. A shadow keeps its light until a shadowless
    /// one is better by [`LightPool::hysteresis`] ([`shadow_set`](crate::shadow_set)). During such a fade
    /// up to `max_shadowed + reserve` lights cast shadows. A pool with shadows wants `reserve: 2`: on the
    /// courtyard, with one reserve shared by hand-offs and shadow changes, three shadows were all placed
    /// in 39 % of frames; with two, in 99 %. A change to this field or to the template's flag is a
    /// settings change, and applies to every light at once, the reserves' included: with the flag off, or a
    /// cap of `Some(0)`, a reserve fading a light out does so without its shadow, on that frame. A cap
    /// lowered to a number above zero reaches the slots at once, and a reserve already fading a shadowed
    /// light out keeps that shadow for the rest of its fade, inside the `max_shadowed + reserve` above.
    pub max_shadowed: Option<usize>,
}

impl Default for LightPool {
    fn default() -> Self {
        Self {
            size: 16,
            reach: 60.0,
            gain: 1.0,
            light: PointLight { intensity: 0.0, range: 10.0, shadow_maps_enabled: false, ..default() },
            rank: Rank::Illuminance,
            focus: FocusRule::One,
            hysteresis: HYSTERESIS,
            fade_seconds: FADE_SECONDS,
            reserve: 1,
            max_shadowed: None,
        }
    }
}

/// Serde for [`LightPool::light`]: every field the pool's lights take from it (all but `intensity`).
#[cfg(feature = "serde")]
mod template_serde {
    use bevy::light::PointLight;
    use bevy::prelude::*;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    /// Bevy's `PointLight::soft_shadows_enabled` exists only with its `experimental_pbr_pcss` feature,
    /// which this crate does not turn on, so it is read and written by name: saved as `false` and
    /// ignored on load in a build without it.
    const SOFT_SHADOWS: &str = "soft_shadows_enabled";

    #[derive(Serialize, Deserialize)]
    #[serde(default)]
    struct Template {
        color: Color,
        range: f32,
        radius: f32,
        shadow_maps_enabled: bool,
        contact_shadows_enabled: bool,
        soft_shadows_enabled: bool,
        affects_lightmapped_mesh_diffuse: bool,
        shadow_depth_bias: f32,
        shadow_normal_bias: f32,
        shadow_map_near_z: f32,
    }

    impl From<&PointLight> for Template {
        fn from(light: &PointLight) -> Self {
            Self {
                color: light.color,
                range: light.range,
                radius: light.radius,
                shadow_maps_enabled: light.shadow_maps_enabled,
                contact_shadows_enabled: light.contact_shadows_enabled,
                soft_shadows_enabled: light
                    .field(SOFT_SHADOWS)
                    .and_then(|field| field.try_downcast_ref::<bool>())
                    .copied()
                    .unwrap_or(false),
                affects_lightmapped_mesh_diffuse: light.affects_lightmapped_mesh_diffuse,
                shadow_depth_bias: light.shadow_depth_bias,
                shadow_normal_bias: light.shadow_normal_bias,
                shadow_map_near_z: light.shadow_map_near_z,
            }
        }
    }

    impl Default for Template {
        fn default() -> Self {
            Self::from(&super::LightPool::default().light)
        }
    }

    impl Template {
        /// The template light, with the intensity the pool ignores at 0.
        fn into_light(self) -> PointLight {
            let mut light = super::LightPool::default().light;
            light.intensity = 0.0;
            light.color = self.color;
            light.range = self.range;
            light.radius = self.radius;
            light.shadow_maps_enabled = self.shadow_maps_enabled;
            light.contact_shadows_enabled = self.contact_shadows_enabled;
            light.affects_lightmapped_mesh_diffuse = self.affects_lightmapped_mesh_diffuse;
            light.shadow_depth_bias = self.shadow_depth_bias;
            light.shadow_normal_bias = self.shadow_normal_bias;
            light.shadow_map_near_z = self.shadow_map_near_z;
            if let Some(soft) =
                light.field_mut(SOFT_SHADOWS).and_then(|field| field.try_downcast_mut::<bool>())
            {
                *soft = self.soft_shadows_enabled;
            }
            light
        }
    }

    pub fn serialize<S: Serializer>(light: &PointLight, serializer: S) -> Result<S::Ok, S::Error> {
        Template::from(light).serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<PointLight, D::Error> {
        Ok(Template::deserialize(deserializer)?.into_light())
    }
}

/// How [`LightPool`] ranks its candidates — and, since [`LightPool::hysteresis`] compares the same
/// quantity, how it decides whether a challenger is worth a hand-off.
///
/// The craft's rule is that the eye reads the light that matters, and what arrives at the subject is a
/// light's output over the square of its distance: [`Rank::Illuminance`] is that sum, and
/// [`Rank::Distance`] is the old rule (the nearest prop wins however dim it is).
///
/// [`Rank::Distance`] is kept for a host whose candidates all burn alike, and because a behaviour change
/// in a released crate should be something a host can switch off.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Rank {
    /// Highest illuminance on the subject first: `intensity / distance²`, so a prop four times as bright
    /// wins at twice the distance.
    #[default]
    Illuminance,
    /// Nearest first, whatever it burns: the rule before [`Rank::Illuminance`] existed.
    Distance,
}

/// How many viewers [`LightPool`] serves: one focus, or several that share its lights.
///
/// A game with one camera has one focus, and the pool measures "near" from it. A game with several
/// (local split-screen co-op, a map camera beside the player's, a cutscene camera while the player stands
/// elsewhere) cannot name one point that is near for all of them, and the craft is the multi-camera
/// cinematographer's: the lighting is a compromise, and the question is who pays for it
/// (`docs/research/pool-foci.md`).
///
/// The pool is **shared, not multiplied**: [`LightPool::size`] is the total for every viewer, so a host
/// whose budget is per view raises it itself (two views at 16 lights each is `size: 32`).
///
/// It is a setting and not a behaviour change because a host that spawns two foci by mistake should be
/// told, which [`FocusRule::One`] still does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FocusRule {
    /// Exactly one entity carries the focus marker. With none the pool is dark; with more than one it is
    /// dark and a warning names the setting that serves several. The rule before [`FocusRule::Shared`]
    /// existed.
    #[default]
    One,
    /// Any number of entities carry the focus marker, one for each viewer, and the pool's lights are
    /// shared between them.
    ///
    /// Each focus is entitled to [`focus_quota`]`(size, foci)` lights, `ceil(size / foci)`, and ranks the
    /// candidates within [`LightPool::reach`] of it by [`LightPool::rank`]; a candidate's place in the
    /// pool's one ordering is its best place among the viewers it is near, measured against each viewer's
    /// own share ([`shared_value`]). So a viewer standing among twenty fires cannot take the lights of one
    /// standing by three: both keep their nearest, and what a viewer does not use goes to the other. A
    /// fixture near two viewers holds one light. The hand-off, the reserve cross-fade, the shadow budget
    /// and [`PoolBudget`] work as for one focus, on that one ordering; a focus that appears or goes (a
    /// player joining or dropping out) re-divides the lights by the same cross-fades.
    ///
    /// With exactly one focus this is [`FocusRule::One`], to the bit.
    Shared,
}

/// A prop that would like one of the pool's real lights: how bright it burns, and where its light
/// sits relative to the prop's origin.
///
/// A candidate may carry a [`Kindle`] or a [`Snuff`] (and no [`Flicker`], which would make it a light of its
/// own): the crate advances them, writes [`Kindled`] and [`Snuffed`] for the candidate, and the light the pool
/// lends is `intensity` times [`kindle_gain`](crate::kindle_gain) times [`snuff_gain`](crate::snuff_gain), the
/// product [`Flicker`] lights use. A [`FlameSite`](crate::FlameSite) whose `fire` names the candidate follows the
/// same state, so a torch that is put out dims its light and shrinks its flame together. A candidate whose
/// `Snuff` has finished no longer wants a light: its slot is freed. Ranking uses the plain `intensity`, so a
/// fire that is going out keeps its slot until it is out.
///
/// Remove the component to take the prop out of the pool at once (a torch that has been taken away, a lamp
/// that has a real light of its own). Re-insert it to put it back.
///
/// A candidate whose own [`Visibility`] is [`Visibility::Hidden`] is skipped — a torch put away for
/// the day is not a light — and so is one further from the focus than [`LightPool::reach`]. A
/// candidate with no `Visibility` at all is not hidden: a host may pool a light on an entity that has
/// no mesh.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[reflect(Component, Default)]
pub struct PoolCandidate {
    /// Where the light goes, in **world** axes, relative to the candidate's own translation. It is
    /// not rotated with the candidate: a torch swung by a walking arm still lights upward from its
    /// head.
    ///
    /// A fire's light belongs at the middle of the flame drawn on it —
    /// [`flame_light_offset`](crate::model::reach::flame_light_offset)`(height)` is that point, relative
    /// to the `FlameSite`'s own origin, which is the flame's foot — so a candidate spawned **on the
    /// `FlameSite` entity** takes that point as its `offset` ([`FireKind::candidate`](crate::FireKind::candidate)
    /// does). With `offset` zero the light would stand at the foot, and the wind's lean, which moves the flame's
    /// *middle*, would then sink it below the coal bed. A candidate on a *parent* of the flame adds only the
    /// difference between the two positions.
    ///
    /// Adding the flame's height to a candidate that already sits at the flame puts the light above the
    /// fire, where nothing visible emits it, and loses the sizing: a torch sized for 250 lx at 2 m
    /// delivers 72 lx when its light ends up 1.8 m higher, because the illuminance is inverse-square in
    /// the distance from where the light actually is. `docs/research/carried-fire.md` measures it.
    pub offset: Vec3,
    /// The light's intensity at [`LightPool::gain`] 1, in lumens.
    pub intensity: f32,
    /// The phase of this prop's own fire, in radians, when the host knows it: the same number on the
    /// prop's `Flicker` (or `FlameSite`) the fire burns at.
    ///
    /// A slot keeps its own phase while it is between owners, so without this a light handed to a torch
    /// flickers at the *slot's* phase while the flame on that torch flickers at the torch's — two clocks
    /// on one fire, which is the defect `docs/research/flame-flicker.md` measures for the braziers.
    /// `None` (the default) leaves the slot's own phase alone, which is what a host that does not
    /// model per-prop fire phases wants.
    pub phase: Option<f32>,
    /// The fire's fuel-bed diameter, in the host's world units, when the host knows it: the same value
    /// the flame on this prop is given, so the handed-out light breathes at the flame's own rate.
    pub diameter: Option<f32>,
    /// How far the fire dips below its peak, when the host chose a depth: the same value as the
    /// [`Flicker::depth`] and [`FlameSite::depth`](crate::FlameSite::depth) of the fire on this prop.
    /// `None` (the default) is [`FLICKER_DEPTH`](crate::FLICKER_DEPTH). A slot adopts it with the
    /// candidate and takes the default again when the next candidate has none.
    pub depth: Option<f32>,
}

impl PoolCandidate {
    /// A candidate whose budget is written at its **mean**: `intensity` is solved as
    /// `mean / `[`flicker_mean`](crate::flicker_mean)`(depth)` and `depth` is set, so the light the
    /// pool hands out averages `mean` (at gain 1) at any depth. Set the other fields with struct
    /// update: `PoolCandidate { offset, ..PoolCandidate::at_mean(lumens, 0.3) }`.
    pub fn at_mean(mean: f32, depth: f32) -> Self {
        let depth = crate::model::flicker_depth(depth);
        Self { intensity: mean / crate::model::flicker_mean(depth), depth: Some(depth), ..Self::default() }
    }

    /// What is unusable in this candidate, field by field; empty when it is fine. See
    /// [`Flicker::problems`](crate::Flicker::problems). `offset` is reported per axis
    /// (`"offset.x"`), `phase`, `diameter` and `depth` only when set, with the ranges the matching
    /// [`Flicker`](crate::Flicker) fields use.
    pub fn problems(&self) -> Vec<crate::Problem> {
        use crate::model::problem::{finite, non_negative, positive, within};
        let mut out = Vec::new();
        finite(&mut out, "offset.x", self.offset.x);
        finite(&mut out, "offset.y", self.offset.y);
        finite(&mut out, "offset.z", self.offset.z);
        non_negative(&mut out, "intensity", self.intensity);
        if let Some(p) = self.phase {
            finite(&mut out, "phase", p);
        }
        if let Some(d) = self.diameter {
            positive(&mut out, "diameter", d);
        }
        if let Some(d) = self.depth {
            within(&mut out, "depth", d, 0.0, crate::MAX_FLICKER_DEPTH);
        }
        out
    }
}

/// One of the crate's spawned real lights. Marks an entity the pool owns and drives.
///
/// A host styles these rather than fighting them: recolour them for the palette, give them a marker
/// so a host system can find them (and so no flame is drawn on them: a pool light is a light in
/// mid-air, the flame belongs on the torch). Its [`Flicker`] is the crate's: the plugin writes the
/// base every frame from the assigned candidate and leaves the phase alone.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[reflect(Component)]
pub struct PoolLight {
    /// Which slot this light is, in `0..`[`LightPool::size`]: stable for the life of the pool, and
    /// the reason two slots never flicker in step (`phase = 1.3 · slot`).
    ///
    /// A **reserve** light (a cross-fade's departing owner) has a slot at or past [`LightPool::size`]:
    /// it is never assigned a candidate, and it is on only while a hand-off is fading out.
    pub slot: usize,
}

impl LightPool {
    /// What is unusable in these settings, field by field; empty when they are fine. See
    /// [`Flicker::problems`](crate::Flicker::problems). A `size` of 0 is a dark pool, not a problem; the
    /// template light's `intensity` is ignored by the pool and not checked.
    pub fn problems(&self) -> Vec<crate::Problem> {
        use crate::model::problem::{non_negative, positive, within};
        let mut out = Vec::new();
        positive(&mut out, "reach", self.reach);
        non_negative(&mut out, "gain", self.gain);
        within(&mut out, "hysteresis", self.hysteresis, 1.0, f32::MAX);
        non_negative(&mut out, "fade_seconds", self.fade_seconds);
        positive(&mut out, "light.range", self.light.range);
        non_negative(&mut out, "light.radius", self.light.radius);
        positive(&mut out, "light.shadow_map_near_z", self.light.shadow_map_near_z);
        out
    }
}

/// Which candidates hold a real light right now, which of those lights cast shadows, and how many
/// wanted one: the pool's lamp schedule.
///
/// [`LightPoolPlugin`] `init_resource`s it and rewrites it every frame, after the assignment; it is
/// read-only for a host (a write is overwritten next frame). A level editor shows it to say which
/// torches are real lights at the viewer's position, and a game reads [`PoolBudget::starved`] to see
/// that its [`LightPool::size`] is too small for the scene.
///
/// A candidate **wants** a light when it is visible, within [`LightPool::reach`] of the focus, and the
/// pool is lit (a focus exists and [`LightPool::gain`] is above 0); a dark pool wants and holds nothing.
/// Reserve lights, which only fade a departed owner out, are not part of it.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Resource, Default)]
pub struct PoolBudget {
    /// The candidate each slot holds, indexed by [`PoolLight::slot`], `0..`[`LightPool::size`].
    pub holders: Vec<Option<Entity>>,
    /// How many candidates asked for a light this frame, held or not.
    pub wanting: usize,
    /// The candidates whose light casts a shadow this frame, in slot order: at most
    /// [`LightPool::max_shadowed`] of the [`holders`](Self::holders), and none when the template's
    /// `shadow_maps_enabled` is off. A reserve light fading a shadowed light out is not listed.
    pub shadowed: Vec<Entity>,
}

impl PoolBudget {
    /// How many real lights are in use.
    pub fn granted(&self) -> usize {
        self.holders.iter().flatten().count()
    }

    /// How many candidates wanted a light and got none: virtual this frame.
    pub fn starved(&self) -> usize {
        self.wanting.saturating_sub(self.granted())
    }

    /// The candidate slot `slot` holds, if any.
    pub fn holder_of(&self, slot: usize) -> Option<Entity> {
        self.holders.get(slot).copied().flatten()
    }

    /// Whether `candidate` holds a real light this frame.
    pub fn holds(&self, candidate: Entity) -> bool {
        self.holders.contains(&Some(candidate))
    }

    /// Whether `candidate`'s light casts a shadow this frame.
    pub fn casts_shadow(&self, candidate: Entity) -> bool {
        self.shadowed.contains(&candidate)
    }
}

/// The part of [`LightPool::light`] every pool light shares: the whole template but the two fields each
/// light sets for itself — its `intensity` (its candidate's, through its [`Flicker`]) and its
/// `shadow_maps_enabled` (the shadow budget's) — zeroed here so a comparison ignores them.
fn shared(template: &PointLight) -> PointLight {
    PointLight { intensity: 0.0, shadow_maps_enabled: false, ..*template }
}

/// Whether two lights agree field for field, by the light's own `Reflect` equality, so a field this
/// crate does not name is compared too: Bevy's `soft_shadows_enabled`, which exists only with its
/// `experimental_pbr_pcss` feature.
fn same_light(a: &PointLight, b: &PointLight) -> bool {
    a.reflect_partial_eq(b) == Some(true)
}

/// Writes the template's shared part onto a pool light, keeping the light's own intensity and shadow
/// flag, and touching it (and so its change detection) only if a setting differs.
fn apply_shared(look: &PointLight, light: &mut Mut<PointLight>) {
    let wanted =
        PointLight { intensity: light.intensity, shadow_maps_enabled: light.shadow_maps_enabled, ..*look };
    if !same_light(light, &wanted) {
        **light = wanted;
    }
}

/// One candidate's light assignment: its rank key (ascending is best), its distance from the focus
/// (squared) for the reach test, its **value** — the quantity the overlap rule compares, a "smaller is
/// better" scalar: the squared distance under [`Rank::Distance`](crate::Rank::Distance) and the squared
/// distance over the intensity (the reciprocal of the illuminance) under
/// [`Rank::Illuminance`](crate::Rank::Illuminance) — its entity, its world position and its intensity.
///
/// The key and the distance are separate because they are different questions: *which* lights should be
/// real is about what arrives at the subject, while the overlap rule is about how far it has to travel.
///
/// A candidate's own fire clock ([`PoolCandidate::phase`], [`PoolCandidate::diameter`]) is deliberately
/// **not** in the tuple: it would be carried through every comparison and every sort swap of the whole
/// candidate list — 5000 entries a frame — to be read for the `pool.size` winners. It is resolved for
/// those instead, in `winners` below. Measured: the two `Option<f32>` in the tuple cost 3.4 % of the
/// pool's own frame (`pool_p50_us` 493 → 511) for a number only the winners use.
type Assignment = (f32, f32, f32, Entity, Vec3, f32);

/// What `assign_pool` reads of a candidate: its entity, its propagated position, its own `Transform`
/// when it has one (the freshest truth in the frame the host moved it), whether it is hidden, whether its
/// fire is out, and how it wants its light.
///
/// The fire's [`Snuff`] is read here, with the row, and not looked up by entity: a fire that is out keeps
/// its `Snuff` for as long as the host leaves it out, so a town at dawn has thousands of them, and a
/// search through the ones that are out for each candidate was quadratic in them (issue #11). An optional
/// column is matched once per table, so a world that never snuffs a fire pays a boolean a candidate.
type CandidateRead<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static GlobalTransform,
        Option<&'static Transform>,
        Option<&'static ChildOf>,
        Option<&'static Visibility>,
        Option<&'static Snuff>,
        &'static PoolCandidate,
    ),
    Without<PoolLight>,
>;

/// A pool light as it last stood: whose light it was, where (the point its `Transform` was given: the
/// candidate's position plus offset plus the flame's lean), how bright (its candidate's intensity, before
/// the pool's gain and fade), its flicker, and whether it cast a shadow. A reserve light fading a light out
/// takes all of it, so the light that leaves keeps moving, and shadowing, exactly as the one that left did —
/// otherwise the departing and arriving lights swing out of step and their sum wobbles. `at` is where it
/// last stood, not where it stands: while `owner` is still in the pool the reserve re-reads that candidate's
/// light point every frame, and `at` only holds the last point it followed it to.
#[derive(Clone, Copy)]
struct LastLight {
    owner: Entity,
    at: Vec3,
    intensity: f32,
    phase: f32,
    diameter: Option<f32>,
    depth: f32,
    shadow: bool,
}

/// The pool's memory between frames: what each slot holds, how long since each slot last changed
/// owner, and the look last copied onto the slots.
///
/// `old_look` is pruned every frame to the candidates that still exist, so it does not grow with the
/// number of candidates ever pooled.
///
/// In one `Local` rather than three so `assign_pool` stays inside clippy's argument budget, and so a
/// reader sees the whole of the pool's state in one place.
#[derive(Default)]
struct HandOffState {
    /// The candidate each slot holds, by slot index.
    held: std::collections::HashMap<usize, Entity>,
    /// Seconds since each slot last changed owner (or shadow). A slot that has never changed owner holds
    /// [`f32::MAX`], so its fade is finished.
    fade_at: std::collections::HashMap<usize, f32>,
    /// The template's shared part ([`shared`]) as it was last written onto the slots.
    applied: Option<PointLight>,
    /// The shadow settings (the template's flag and [`LightPool::max_shadowed`]) the slots were last
    /// shadowed under: a change of settings applies at once, a change of ranking cross-fades.
    shadow_settings: Option<(bool, Option<usize>)>,
    /// Each candidate's light as it last stood: a departing owner's, for a cross-fade, which by the
    /// frame of the hand-off is no longer in the candidate list.
    old_look: std::collections::HashMap<Entity, LastLight>,
    /// Which reserve light is fading out which light, by slot index.
    departure_release: std::collections::HashMap<usize, LastLight>,
    /// This frame's foci: none, one, or (under [`FocusRule::Shared`]) one per viewer. Scratch, kept for
    /// its capacity.
    foci: Vec<Vec3>,
    /// Under [`FocusRule::Shared`] with several foci: each candidate's rank value from each focus,
    /// `foci.len()` numbers a candidate in the order of the list, [`f32::INFINITY`] where the candidate is
    /// beyond that focus's reach. Scratch, kept for its capacity.
    shares: Vec<f32>,
    /// One focus's column of `shares`, and every focus's cutoff ([`quota_cutoff`]). Scratch.
    column: Vec<f32>,
    cutoffs: Vec<f32>,
}

/// Spawns, resizes and assigns the pool's real lights.
///
/// One frame, in order:
///
/// 1. **Slots.** Spawns the missing entities so slots `0..size` exist, and despawns those past
///    `size`. When [`LightPool::light`] changed, every field of it but `intensity` and
///    `shadow_maps_enabled` is copied onto every slot, reserves included; each slot's intensity is left
///    to the assignment and its shadow flag to the shadow budget.
/// 2. **Foci.** [`FocusRule::One`] (the default): the one entity carrying `F`. None, or more than one,
///    means no assignment this frame (and a warning for the second case): a pool with two cameras has
///    no single near unless it is told to share. [`FocusRule::Shared`]: every entity carrying `F`.
/// 3. **Candidates.** Every [`PoolCandidate`] whose own [`Visibility`] is not `Hidden` and whose fire is not
///    out (a [`Snuff`] that has run its course), within [`LightPool::reach`], ranked by [`LightPool::rank`]
///    and then by [`Entity`], so the assignment is the same on every machine and two props of equal rank
///    never swap between frames.
/// 4. **Assign.** Every slot's owner for the frame is decided **before any slot is written**: a slot keeps
///    its candidate unless a candidate no slot holds is better by [`LightPool::hysteresis`], and a
///    challenger takes the **worst-ranked** light the pool holds rather than displacing every slot that
///    happens to compare against it. At most as many lights change owner per frame as [`LightPool::reserve`]
///    allows (all of them when it is 0), so a hand-off the pool cannot cross-fade is not made. Each slot
///    then takes its candidate's position plus its [`PoolCandidate::offset`], with
///    `Flicker::base = intensity · gain · fade`. Every slot with no candidate is hidden, and the whole pool
///    is hidden when there is no focus or the gain is zero.
/// 5. **Shadow.** With the template's `shadow_maps_enabled`, the best-ranked
///    [`LightPool::max_shadowed`] holders cast shadows ([`shadow_set`]); a light that keeps its owner and
///    gains or loses its shadow because the ranking moved is cross-faded to itself through a free reserve.
#[allow(clippy::too_many_arguments)]
fn assign_pool<F: Component>(
    mut commands: Commands,
    pool: Res<LightPool>,
    mut schedule: ResMut<PoolBudget>,
    focus: Query<&GlobalTransform, With<F>>,
    candidates: CandidateRead,
    fires: Query<(Option<&Kindle>, Option<&Snuff>), With<PoolCandidate>>,
    sites: Query<&FlameSite>,
    wind: Option<Res<FlameWind>>,
    mut slots: Query<(Entity, &PoolLight, &mut Transform, &mut Visibility, &mut Flicker, &mut PointLight)>,
    mut state: Local<HandOffState>,
    time: Res<Time>,
) {
    let delta = time.delta_secs();
    let HandOffState {
        held,
        fade_at,
        applied,
        shadow_settings,
        old_look,
        departure_release,
        foci,
        shares,
        column,
        cutoffs,
    } = &mut *state;
    let existing: Vec<(Entity, usize)> =
        slots.iter().map(|(entity, light, ..)| (entity, light.slot)).collect();

    // 1. Slots: one entity per slot, plus the reserve lights, and no more and no fewer. A reserve's
    //    slot index is `size..size + reserve`, which no candidate is ever assigned to.
    for slot in 0..pool.size + pool.reserve {
        if !existing.iter().any(|(_, at)| *at == slot) {
            commands.spawn((
                Name::new(format!("Pool light {slot}")),
                PoolLight { slot },
                shared(&pool.light),
                Flicker { base: 0.0, phase: 1.3 * slot as f32, ..default() },
                Transform::default(),
                Visibility::Hidden,
            ));
        }
    }
    for (entity, slot) in &existing {
        if *slot >= pool.size + pool.reserve {
            commands.entity(*entity).despawn();
        }
    }

    // The shared light settings follow the template, so a host can recolour, re-range or re-shadow the
    // whole pool in one write: every field but the two each light sets for itself ([`shared`]), so a
    // light spawned later and a light already burning carry the same settings. They are applied when the
    // *template's* shared part changes, not whenever the resource does: a host that writes `gain` every
    // frame (the night's glide) would otherwise reset every slot to the template's colour on every frame,
    // undoing a palette that tints its own lights.
    let look = shared(&pool.light);
    if !applied.as_ref().is_some_and(|last| same_light(last, &look)) {
        for (_, _, _, _, _, mut light) in &mut slots {
            apply_shared(&look, &mut light);
        }
        *applied = Some(look);
    }

    // 2. Foci: exactly one entity under `FocusRule::One`, any number under `FocusRule::Shared`.
    foci.clear();
    match pool.focus {
        FocusRule::One => {
            let mut found = focus.iter();
            match (found.next(), found.next()) {
                (Some(at), None) => foci.push(at.translation()),
                (Some(_), Some(_)) => {
                    let count = 2 + found.count();
                    warn_once!(
                        "bevy_lights: LightPoolPlugin found {count} focus entities; the pool needs exactly one (or `LightPool::focus` set to `FocusRule::Shared`)"
                    );
                }
                _ => {}
            }
        }
        FocusRule::Shared => foci.extend(focus.iter().map(|at| at.translation())),
    }

    // 3. Candidates: visible, in reach, and ranked by the pool's rule — highest illuminance on the subject
    //    first (or nearest first), ties by entity.
    let reach_squared = pool.reach * pool.reach;
    // One focus is the plain rule. Several are the shared one: each candidate's value from each focus is
    // kept in `shares` (a row of `foci.len()` numbers a candidate, in the order of `lit`) until every
    // focus's cutoff is known.
    let single = match foci.as_slice() {
        [at] => Some(*at),
        _ => None,
    };
    shares.clear();
    let mut lit: Vec<Assignment> = if foci.is_empty() || pool.gain <= 0.0 {
        Vec::new()
    } else {
        candidates
            .iter()
            // Not `Visibility::Hidden`: a candidate with no `Visibility` at all is not hidden. And not out: a
            // candidate whose `Snuff` has run its course wants no light, and a `Snuff` in progress still does.
            .filter(|(.., visibility, snuff, _)| {
                !visibility.is_some_and(|v| *v == Visibility::Hidden)
                    && !snuff.is_some_and(|snuff| snuff.elapsed >= SNUFF_SECONDS)
            })
            .filter_map(|(entity, at, transform, parent, _, _, candidate)| {
                // The freshest position the frame has. A **root** prop with its own `Transform` has just
                // been moved by the host, and its `GlobalTransform` will not catch up until `PostUpdate`,
                // so its local translation is the freshest truth. A **child** (a torch in a hand) is
                // placed by its parent: its own `Transform` is a constant local offset, and reading it
                // would put every torch at the same point — which is the bug this arity fixes — so a
                // child always uses its `GlobalTransform`.
                let position = match (parent, transform) {
                    (None, Some(transform)) => transform.translation,
                    _ => at.translation(),
                };
                // The value is the pool's one quantity: what it **ranks** by and what the overlap rule
                // **holds** against, in the units the hysteresis margin applies to. "Smaller is better",
                // and positive — `preferred` clamps at zero, so a negated brightness would collapse every
                // comparison to `0 < 0`.
                //
                // `Illuminance` is the reciprocal of the light arriving at the subject,
                // `distance² / intensity`: the inverse square the crate's `fire_lumens` inverts, so a prop
                // four times as bright ranks exactly equal at twice the distance (which is what the
                // tie-break on entity depends on), and a challenger must arrive 15 % brighter to take a
                // light. `Distance` is the distance itself, the pair the old rule compared, so that rule is
                // unchanged.
                let value_at = |distance_squared: f32| match pool.rank {
                    Rank::Illuminance => distance_squared / candidate.intensity.max(1e-6),
                    Rank::Distance => distance_squared.sqrt(),
                };
                let (distance_squared, value) = match single {
                    Some(focus_at) => {
                        let distance_squared = position.distance_squared(focus_at);
                        (distance_squared, value_at(distance_squared))
                    }
                    None => {
                        // Several foci: the value from each one this candidate is within reach of, kept
                        // for the cutoffs; the pool's own value is settled below, once they are known.
                        let row = shares.len();
                        let mut nearest = f32::INFINITY;
                        for focus_at in foci.iter() {
                            let distance_squared = position.distance_squared(*focus_at);
                            if distance_squared <= reach_squared {
                                nearest = nearest.min(distance_squared);
                                shares.push(value_at(distance_squared));
                            } else {
                                shares.push(f32::INFINITY);
                            }
                        }
                        if nearest.is_infinite() {
                            shares.truncate(row);
                            return None;
                        }
                        (nearest, value_at(nearest))
                    }
                };
                (distance_squared <= reach_squared).then_some((
                    value,
                    distance_squared,
                    value,
                    entity,
                    position + candidate.offset,
                    candidate.intensity,
                ))
            })
            .collect()
    };
    // Shared between several foci: each focus's value is its own multiple of its cutoff (the value of its
    // quota-th best candidate), and a candidate takes its best. See `model::pool::shared_value`.
    if single.is_none() && !lit.is_empty() {
        let k = foci.len();
        let quota = focus_quota(pool.size, k);
        cutoffs.clear();
        for at in 0..k {
            column.clear();
            column.extend(shares.iter().skip(at).step_by(k).copied().filter(|value| value.is_finite()));
            cutoffs.push(quota_cutoff(column, quota).unwrap_or(1.0));
        }
        for (entry, row) in lit.iter_mut().zip(shares.chunks_exact(k)) {
            let value = row
                .iter()
                .zip(cutoffs.iter())
                .filter(|(value, _)| value.is_finite())
                .map(|(value, cutoff)| shared_value(*value, *cutoff))
                .fold(f32::INFINITY, f32::min);
            entry.0 = value;
            entry.2 = value;
        }
    }
    // Ties break on `Entity`, never on the value: four times the light at twice the distance is an exact
    // tie by design, and two equidistant props must not swap between frames.
    lit.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.3.to_bits().cmp(&b.3.to_bits())));

    // 4. Hold or hand over. Every slot's owner for this frame is decided here, in a map, **before** a
    //    single slot is written: a decision that depended on the order the slots happen to be written in
    //    is how a slot took a candidate another slot was keeping.
    //
    //    Three rules, and the first two are what stop a reshuffle:
    //
    //    - **A candidate another slot holds is not a challenger.** It belongs to its holder. Comparing
    //      every slot against one global best free candidate let a single bright newcomer displace all of
    //      them at once — measured on the courtyard, seven of eight lights went dark for one frame and
    //      faded back over the next twenty (`docs/research/pool-hand-off.md`).
    //    - **A challenger displaces the worst-ranked light the pool holds**, one light per challenger, so
    //      the pool's assignment is the best set it can hold rather than a wholesale re-sort.
    //    - **At most `reserve` lights move in a frame** (all of them when the reserve is 0), because a
    //      hand-off the pool cannot cross-fade is a hand-off that dims its subject. A scene that needs
    //      more moves gets them on the following frames: the next frame recomputes the same challengers,
    //      so the pool catches up without ever blacking out a street for one frame.
    //
    //    A slot that changes owner fades back up from nothing; a slot's first light does not fade. A slot
    //    whose owner is gone, or displaced, takes the best candidate nobody holds. The rule and the fade
    //    are in `model::pool`; the craft is in `docs/research/pool-hand-off.md`.
    let mut chosen_by_slot: std::collections::HashMap<usize, Entity> = std::collections::HashMap::new();
    // How many lights may change owner this frame, and **which** reserves carry the cross-fade. A reserve
    // is busy for the whole of the fade it is carrying, and re-using one would cut the departing light
    // off mid-fade — a pop at the end of a fade meant to hide one. So the free reserves are listed once,
    // and departure `k` goes to the `k`-th *free* one: counting them and then indexing all of them
    // would hand a busy reserve a second light. With no reserves at all there is nothing to hide a
    // hand-off behind, so the frame is not throttled.
    let free_reserves: Vec<usize> = (pool.size..pool.size + pool.reserve)
        .filter(|reserve| {
            let busy = departure_release.contains_key(reserve)
                && fade_at.get(reserve).is_some_and(|age| {
                    *age < pool.fade_seconds && fade_out_factor(*age, pool.fade_seconds) < 1.0
                });
            !busy
        })
        .collect();
    let budget = if pool.reserve == 0 { pool.size } else { free_reserves.len() };
    {
        // Slots in index order, so nothing below depends on the order the query happened to yield.
        let mut order: Vec<(usize, Option<Entity>)> =
            slots.iter().map(|(_, slot, ..)| (slot.slot, held.get(&slot.slot).copied())).collect();
        order.sort_unstable_by_key(|(slot, _)| *slot);
        order.retain(|(slot, _)| *slot < pool.size);
        let holding_of = |slot: usize| order.iter().find(|(at, _)| *at == slot).and_then(|(_, held)| *held);
        let rank_of = |entity: Entity| lit.iter().position(|(_, _, _, candidate, ..)| *candidate == entity);

        // Who keeps what it holds: a slot whose candidate is still in the list. Everything else is an
        // open slot, filled below.
        let mut keepers: Vec<usize> = Vec::new();
        let mut open: Vec<usize> = Vec::new();
        for (slot, _) in &order {
            match holding_of(*slot).map(rank_of) {
                Some(Some(_)) => keepers.push(*slot),
                _ => open.push(*slot),
            }
        }
        for slot in &keepers {
            if let Some(holding) = holding_of(*slot) {
                chosen_by_slot.insert(*slot, holding);
            }
        }

        // The challengers: the best-ranked candidates no slot holds, best first.
        let held_by_any: Vec<Entity> = order.iter().filter_map(|(_, holding)| *holding).collect();
        // Each challenger with its value, best first: the list is already in rank order.
        let mut challengers: Vec<(Entity, f32)> = lit
            .iter()
            .filter(|(_, _, _, entity, ..)| !held_by_any.contains(entity))
            .map(|(_, _, value, entity, ..)| (*entity, *value))
            .collect();

        // A challenger takes the **worst-ranked** light the pool holds, and only when the overlap rule
        // says the move is worth making: the challenger must be nearer by `hysteresis`, so two props at
        // nearly the same distance never trade a light frame after frame. How many may move, and behind
        // which reserves, is decided above.
        let mut moved = 0usize;
        while moved < budget {
            let worst = keepers
                .iter()
                .filter_map(|slot| {
                    let holding = holding_of(*slot)?;
                    Some((*slot, rank_of(holding)?, lit.iter().find(|(_, _, _, e, ..)| *e == holding)?))
                })
                .max_by_key(|(_, rank, _)| *rank);
            let Some((worst_slot, _, held)) = worst else { break };
            let Some((challenger, challenger_value)) = challengers.first().copied() else { break };
            // The overlap rule, on the rank's own quantity: a challenger has to be meaningfully nearer (or
            // meaningfully brighter) to win, so props at nearly the same value never trade a light.
            // Both values are "smaller is better" (see the rank key above): the same rule that ranks the
            // candidates decides whether a challenger has earned the light.
            if !preferred(held.2, challenger_value, pool.hysteresis) {
                break;
            }
            keepers.retain(|slot| *slot != worst_slot);
            chosen_by_slot.remove(&worst_slot);
            open.push(worst_slot);
            moved += 1;
            challengers.remove(0);
            chosen_by_slot.insert(worst_slot, challenger);
        }

        // Every open slot — an empty slot, or one whose owner was displaced — takes the best candidate
        // nobody holds, in slot order. An unfilled pool therefore reproduces "slot k takes candidate k".
        open.sort_unstable();
        for slot in open {
            if chosen_by_slot.contains_key(&slot) {
                continue;
            }
            let taken = chosen_by_slot.values().copied().collect::<Vec<Entity>>();
            if let Some((_, _, _, entity, ..)) =
                lit.iter().find(|(_, _, _, entity, ..)| !taken.contains(entity))
            {
                chosen_by_slot.insert(slot, *entity);
            }
        }
    }

    // A cross-fade's departing light: where a slot's *old* owner was, at its intensity. It is handed to
    // a free reserve light before the slots are written, so the departing light is on in the frame the
    // hand-off happens. The pool moves at most `reserve` lights a frame (above), so every departure in a
    // frame gets its own reserve light and no hand-off is left to the dip.
    //
    // **The old owner need not have left the pool.** The common hand-off is a slot changing owner while
    // the old one is still a candidate — it has simply been displaced by a nearer prop — and cross-fading
    // only a *departed* owner left that case with the dip. So a departure here is "this slot's owner is
    // about to change", which is known before the slots are written.
    //
    // The free reserves are taken in order: owner changes first, and a change of shadow (step 5) takes
    // what is left.
    let mut spare = free_reserves.iter().copied();
    {
        // A slot whose owner is about to change, whether it has a replacement or not. A slot whose owner
        // was despawned or went out of reach has no new owner, and that is still a departure — the light
        // must fade out where it stood rather than blink off.
        let mut departures: Vec<(usize, Entity)> = Vec::new();
        for (slot, holding) in slots.iter().map(|(_, slot, ..)| (slot.slot, held.get(&slot.slot).copied())) {
            if slot >= pool.size {
                continue;
            }
            match (holding, chosen_by_slot.get(&slot).copied()) {
                (Some(holding), Some(chosen)) if holding != chosen => departures.push((slot, holding)),
                (Some(holding), None) => departures.push((slot, holding)),
                _ => {}
            }
        }
        departures.sort_unstable();
        for (_, holding) in &departures {
            let Some(last) = old_look.get(holding).copied() else { continue };
            let Some(reserve) = spare.next() else { break };
            departure_release.insert(reserve, last);
            fade_at.insert(reserve, 0.0);
        }
    }

    // 5. Shadows: which held lights cast one this frame, decided before any slot is written, like the
    //    owners. `shadow_set` names the best-ranked holders; this applies its answer within the budget.
    //
    //    A light that changes owner (or is a slot's first light) takes its new owner's shadow at once: it
    //    is fading in from nothing anyway, and the reserve fading the old owner out carries the old flag.
    //    A light that **keeps** its owner and gains or loses its shadow is cross-faded to itself: a free
    //    reserve takes it (and then follows its candidate, as the slot does) and fades it out while the slot
    //    fades back in with the new flag, so the shadow deepens (or lifts) over the fade and the lit ground
    //    around it does not move. That waits for a free reserve and for the slot's own fade to finish; with
    //    no reserves at all it dips.
    //    Losses are applied before gains, so a gain that has to wait never puts the pool over budget.
    //
    //    A change of the *settings* (the template's flag, the cap) is not a hand-off: like the template's
    //    colour it applies to every light on the frame it is made, as a game's graphics menu does.
    let shadow_budget =
        if pool.light.shadow_maps_enabled { pool.max_shadowed.unwrap_or(usize::MAX) } else { 0 };
    let settings = (pool.light.shadow_maps_enabled, pool.max_shadowed);
    let snap = *shadow_settings != Some(settings);
    *shadow_settings = Some(settings);
    let mut shadow_of: std::collections::HashMap<usize, bool> = std::collections::HashMap::new();
    let mut restart: Vec<usize> = Vec::new();
    {
        let casting: std::collections::HashMap<usize, bool> =
            slots.iter().map(|(_, slot, .., light)| (slot.slot, light.shadow_maps_enabled)).collect();
        // The held lights, best-ranked first: (slot, candidate, rank, value, kept its owner, shadowed now).
        let mut holders: Vec<(usize, Entity, usize, f32, bool, bool)> = chosen_by_slot
            .iter()
            .filter(|(slot, _)| **slot < pool.size)
            .filter_map(|(slot, entity)| {
                let rank = lit.iter().position(|(_, _, _, candidate, ..)| candidate == entity)?;
                let kept = held.get(slot) == Some(entity);
                let now = kept && casting.get(slot).copied().unwrap_or(false);
                Some((*slot, *entity, rank, lit[rank].2, kept, now))
            })
            .collect();
        holders.sort_unstable_by_key(|(_, _, rank, ..)| *rank);
        let want = shadow_set(
            &holders.iter().map(|(.., value, _, now)| (*value, *now)).collect::<Vec<_>>(),
            shadow_budget,
            pool.hysteresis,
        );
        // Whether a kept light may change its shadow this frame, and if so whether its light restarts its
        // fade: at once on a settings change; through a free reserve (or a dip, with none) otherwise.
        let mut change = |slot: usize, entity: Entity| -> Option<bool> {
            if snap {
                return Some(false);
            }
            if fade_at.get(&slot).is_some_and(|age| *age < pool.fade_seconds) {
                return None;
            }
            if pool.reserve == 0 {
                return Some(true);
            }
            let last = old_look.get(&entity).copied()?;
            let reserve = spare.next()?;
            departure_release.insert(reserve, last);
            fade_at.insert(reserve, 0.0);
            Some(true)
        };
        let mut count = 0usize;
        // Shadows that stay, and losses: a loss that cannot be cross-faded yet keeps its shadow this frame.
        for ((slot, entity, .., now), want) in holders.iter().zip(&want) {
            if !*now {
                continue;
            }
            let shadow = *want
                || match change(*slot, *entity) {
                    Some(fades) => {
                        if fades {
                            restart.push(*slot);
                        }
                        false
                    }
                    None => true,
                };
            if shadow {
                count += 1;
            }
            shadow_of.insert(*slot, shadow);
        }
        // Gains, best-ranked first, while the budget has room.
        for ((slot, entity, _, _, kept, now), want) in holders.iter().zip(&want) {
            if *now {
                continue;
            }
            let gain = *want
                && count < shadow_budget
                && (!*kept
                    || match change(*slot, *entity) {
                        Some(fades) => {
                            if fades {
                                restart.push(*slot);
                            }
                            true
                        }
                        None => false,
                    });
            if gain {
                count += 1;
            }
            shadow_of.insert(*slot, gain);
        }
    }

    // Each winner's own fire clock and depth, resolved once for the `pool.size` candidates that won
    // rather than carried through the rank list's sort. A candidate that gives none of these is absent
    // from the map, so a slot it wins keeps the slot's own `1.3 · slot` phase, the un-sized rate and
    // the crate's `FLICKER_DEPTH`.
    let winners: std::collections::HashMap<_, _> = chosen_by_slot
        .values()
        .filter_map(|entity| {
            candidates
                .get(*entity)
                .ok()
                .map(|(.., candidate)| (*entity, (candidate.phase, candidate.diameter, candidate.depth)))
        })
        .collect();

    // Where a candidate's light stands relative to the point the pool puts it at: a flame leaning in the
    // wind has its middle, and so its light, off the upright axis. The site is the candidate's own, or the
    // one that names it as its `fire`. `diameter` is the candidate's own fire diameter, if it gives one.
    // One rule for the slot and for the reserve fading the same light out, so the two ends of a cross-fade
    // cannot stand apart.
    let lean_of = |entity: Entity, diameter: Option<f32>| -> Vec3 {
        match &wind {
            Some(wind) if wind.velocity != Vec2::ZERO => {
                match sites.get(entity).ok().or_else(|| sites.iter().find(|s| s.fire == Some(entity))) {
                    Some(site) => {
                        let speed = wind.velocity.length();
                        let diameter = diameter.or(site.diameter).unwrap_or(DEFAULT_FIRE_DIAMETER);
                        let toward = wind.velocity / speed;
                        Vec3::from(lean_light_shift(site.height, speed, diameter, [toward.x, toward.y]))
                    }
                    None => Vec3::ZERO,
                }
            }
            _ => Vec3::ZERO,
        }
    };

    for (_, slot, mut transform, mut visibility, mut flicker, mut light) in &mut slots {
        // A reserve light: it is not assigned a candidate, only used to fade a departure out.
        if slot.slot >= pool.size {
            let age = fade_at.entry(slot.slot).or_insert(f32::MAX);
            // Read before it ages, so the departing light runs on the arriving light's clock (which reads
            // 0 on the frame of the hand-off): the two sum to exactly one on every frame of the fade. Read
            // after, the pair dipped by up to 7 % of a light mid-fade, one frame's step of the smoothstep.
            let out = fade_out_factor(*age, pool.fade_seconds);
            *age += delta;
            let live = departure_release.get(&slot.slot).copied();
            // On while there is light left to fade; the frame the fade ends it is hidden and freed, so a
            // finished reserve is never a dark light (or a dark shadow map) the renderer still draws.
            if out > 0.0
                && let Some(mut last) = live
            {
                // The departing light is a candidate's light, and it stands where that light does now: the
                // candidate's position plus offset plus lean, as the slot works it out. A light cross-faded
                // to itself therefore has both ends on one point, and a light displaced by a nearer one
                // keeps moving with its torch. Once the candidate has left the pool (hidden, out of reach,
                // despawned) there is no "now", and the light holds the last point it followed it to.
                if let Some(at) = lit
                    .iter()
                    .find(|(_, _, _, candidate, ..)| *candidate == last.owner)
                    .map(|(.., at, _)| *at)
                {
                    last.at = at + lean_of(last.owner, last.diameter);
                    departure_release.insert(slot.slot, last);
                }
                transform.translation = last.at;
                // The departing light keeps the flicker it had (the phase, rate and depth of the light that
                // left), so the two ends of the cross-fade move together instead of beating against each
                // other, and its shadow, so a shadow leaves with its light — while the pool has a shadow
                // budget at all. The snapshot was taken under last frame's settings, so a template flag
                // turned off, or a cap of nothing, reaches the reserve here, on the frame it is made.
                flicker.phase = last.phase;
                flicker.diameter = last.diameter;
                flicker.depth = last.depth;
                flicker.base = last.intensity * pool.gain * out;
                let shadow = last.shadow && shadow_budget > 0;
                if light.shadow_maps_enabled != shadow {
                    light.shadow_maps_enabled = shadow;
                }
                visibility.set_if_neq(Visibility::Inherited);
            } else {
                flicker.base = 0.0;
                if light.shadow_maps_enabled {
                    light.shadow_maps_enabled = false;
                }
                visibility.set_if_neq(Visibility::Hidden);
                departure_release.remove(&slot.slot);
            }
            continue;
        }
        let holding = held.get(&slot.slot).copied();
        // This frame's owner for the slot, decided in one pass above: the same map for every slot, so no
        // slot's answer can depend on the order the slots are written in.
        let chosen = chosen_by_slot.get(&slot.slot).copied();
        // A slot's first light is not a hand-off; only a change from one owner to another fades, and a
        // light cross-faded to itself for a change of shadow.
        let changed = holding.is_some() && chosen != holding;

        let age = fade_at.entry(slot.slot).or_insert(f32::MAX);
        if changed || restart.contains(&slot.slot) {
            *age = 0.0;
        } else if holding.is_some() && chosen.is_some() {
            *age += delta;
        }
        let fade =
            if holding.is_none() && chosen.is_some() { 1.0 } else { fade_factor(*age, pool.fade_seconds) };

        let assigned = chosen.and_then(|entity| {
            lit.iter().find(|(_, _, _, candidate, ..)| *candidate == entity).map(|(.., at, i)| {
                // A fire taking hold or going out lends a light at its own state's factor.
                let state = fires.get(entity).map_or(1.0, |(kindle, snuff)| {
                    kindle.map_or(1.0, |k| kindle_gain(k.elapsed))
                        * snuff.map_or(1.0, |s| snuff_gain(s.elapsed))
                });
                (*at, *i * state, entity)
            })
        });
        let shadow = assigned.is_some() && shadow_of.get(&slot.slot).copied().unwrap_or(false);
        if light.shadow_maps_enabled != shadow {
            light.shadow_maps_enabled = shadow;
        }
        match assigned {
            Some((at, intensity, entity)) => {
                // A flame leaning in the wind has its middle, and so its light, off the upright axis.
                let lean = lean_of(entity, winners.get(&entity).and_then(|(_, diameter, _)| *diameter));
                transform.translation = at + lean;
                // The slot keeps its own phase until a candidate says otherwise: the light handed to a
                // prop must breathe on that prop's clock, or it beats against the flame drawn on it.
                if let Some((phase, diameter, depth)) = winners.get(&entity).copied() {
                    if let Some(phase) = phase {
                        flicker.phase = phase;
                    }
                    flicker.diameter = diameter;
                    flicker.depth = depth.unwrap_or(FLICKER_DEPTH);
                }
                flicker.base = intensity * pool.gain * fade;
                visibility.set_if_neq(Visibility::Inherited);
            }
            None => {
                flicker.base = 0.0;
                visibility.set_if_neq(Visibility::Hidden);
                *age = f32::MAX;
            }
        }
        match chosen {
            Some(entity) => {
                held.insert(slot.slot, entity);
            }
            None => {
                held.remove(&slot.slot);
            }
        }
        // Remember how this slot's light now stands, so a departure next frame fades out exactly this.
        if let Some((_, intensity, entity)) = assigned {
            old_look.insert(
                entity,
                LastLight {
                    owner: entity,
                    at: transform.translation,
                    intensity,
                    phase: flicker.phase,
                    diameter: flicker.diameter,
                    depth: flicker.depth,
                    shadow,
                },
            );
        }
    }
    // Publish the frame's answer: who holds each real slot, whose light casts a shadow, and how many
    // candidates asked. Written only when it differs, so `PoolBudget` is flagged changed on a hand-off and
    // not on every frame, and a steady pool allocates nothing here.
    let casts = |slot: usize| {
        shadow_of.get(&slot).copied().unwrap_or(false).then(|| held.get(&slot).copied()).flatten()
    };
    let unchanged = schedule.wanting == lit.len()
        && schedule.holders.len() == pool.size
        && schedule.holders.iter().enumerate().all(|(slot, holder)| *holder == held.get(&slot).copied())
        && schedule.shadowed.iter().copied().eq((0..pool.size).filter_map(casts));
    if !unchanged {
        schedule.holders.clear();
        schedule.holders.extend((0..pool.size).map(|slot| held.get(&slot).copied()));
        schedule.wanting = lit.len();
        schedule.shadowed.clear();
        schedule.shadowed.extend((0..pool.size).filter_map(casts));
    }
    // Prune the remembered positions to what the pool can still need: the candidates in this frame's
    // list and the ones a slot holds. Keyed by `Entity`, this map would otherwise grow by one entry for
    // every candidate ever pooled, which is a leak in a library that runs for hours.
    old_look.retain(|entity, _| {
        lit.iter().any(|(_, _, _, candidate, ..)| candidate == entity)
            || held.values().any(|held| held == entity)
    });
}
