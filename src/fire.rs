//! [`Fire`]: a whole fire as one component.
//!
//! A fire is a flame, the light the pool hands it, and (for the kinds that have them) sparks, smoke, tongues and a bed of coals, and the
//! numbers they share (diameter, depth, height, the light's offset, the phase, the colour) must agree. [`Fire`] holds the few choices that
//! are a host's (a [`FireKind`], how bright, a colour) and [`FirePlugin`] writes the six components from them, so a fire is placed, saved,
//! loaded and edited as one value. The design and its sources are in `docs/research/fire-component.md`.

use bevy::prelude::*;

use crate::model::{fire_phase, flame_light_offset, flame_mid};
use crate::{
    FireKind, FlameEmbers, FlamePlugin, FlameSite, FlameSmoke, FlameTongues, FuelBed, LightPool,
    LightsSystems, PoolCandidate,
};

/// The illuminance, in lux, a [`Fire`] with no `lumens` of its own is sized to put on the ground at the edge of its kind's
/// [`reach`](FireKind::reach), with its light at the flame's middle and the fire's foot on the ground: the courtyard's own figure for a
/// carried fire, the shoulder two stops under a pool's centre (`docs/research/lighting-look.md`), in the crate's reference exposure
/// (EV100 6). A design default, and only a starting point: the lumens that read right depend on the host's exposure, which nothing here
/// converts, so a host sets [`Fire::lumens`] (or builds the fire with [`Fire::lit`]).
pub const FIRE_EDGE_LUX: f32 = 250.0;

/// A whole fire: its flame, the light the pool hands it, and its sparks, smoke, tongues and bed of coals, from a kind, a brightness and a colour.
///
/// Spawn it on the entity at the flame's foot (with a `Transform`) and [`FirePlugin`] gives that entity a [`FlameSite`] (a flame the height
/// and diameter of the kind), a [`PoolCandidate`] (the light [`LightPool`] hands the fire when it is near enough, standing in the flame and
/// breathing at its rate and depth), and the kind's own [`FlameEmbers`], [`FlameSmoke`], [`FlameTongues`] and [`FuelBed`] where it has them
/// ([`FireKind::embers`] and its siblings say which). A fire is put out and lit again by [`Snuff`](crate::Snuff) and
/// [`Kindle`](crate::Kindle) on the same entity, as a pooled torch always was.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_lights::*;
/// # fn spawn(mut commands: Commands) {
/// // A campfire whose light puts 250 lx on the ground 5 m out, 0.4 m up; every fire a different phase.
/// commands.spawn((Fire::lit(FireKind::Campfire, 250.0, 0.4), Transform::from_xyz(4.0, 0.0, 2.0)));
/// // A blue gas torch with no sparks.
/// commands.spawn((
///     Fire { color: Some(Color::srgb(0.1, 0.3, 1.0)), embers: false, ..Fire::new(FireKind::Torch) },
///     Transform::from_xyz(-3.0, 1.5, 0.0),
/// ));
/// # }
/// ```
///
/// # What the fire owns
///
/// When a `Fire` arrives, each part is written only if the entity does not already carry one, so what a host spawns beside it wins, as a
/// required component's explicit value does: `(Fire::new(kind), FlameSmoke { density: 0.2 })` keeps its thin smoke. When a `Fire` is
/// **changed** afterwards (replaced, or edited through `get_mut`, reflection or BRP) every part is written again, and a part its fields no
/// longer name is removed, so a panel that turns a campfire into a lantern leaves no tongues behind. What a host edits on a part after
/// that stands until the `Fire` next changes. Removing the `Fire` leaves the parts: they are ordinary components the host may keep.
///
/// The fire needs [`FlamePlugin`] to be drawn, [`LightPoolPlugin`](crate::LightPoolPlugin) to be lit and `FlickerPlugin` to breathe;
/// [`FirePlugin`] reports one of them missing rather than adding it.
///
/// With the `serde` feature it is `Serialize` and `Deserialize`, and every field is optional on the way in, so `(kind: Campfire)` is a
/// campfire.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
#[reflect(Component, Default)]
pub struct Fire {
    /// What burns: the flame's height, the bed's diameter (which sizes the flicker), how far it dips, and which of the extras it has.
    pub kind: FireKind,
    /// The mean lumens of the light the pool hands this fire, in the host's own units and exposure. `None` is sized from the kind: the
    /// lumens that put [`FIRE_EDGE_LUX`] on the ground at the edge of its [`reach`](FireKind::reach).
    pub lumens: Option<f32>,
    /// This fire's own clock start, in radians: the flame and the light it is handed breathe from it. `None` is derived from the entity
    /// ([`fire_phase`](crate::fire_phase)), so fires placed alike do not breathe together and a run that spawns in the same order is
    /// reproducible.
    pub phase: Option<f32>,
    /// The fire's own colour, when it is not the scene's orange: the colour of the light, and the middle of the flame's ramp as the same hue
    /// at the shipped flame's brightness ([`flame_mid`](crate::flame_mid)), so one value is a fire of that colour, flame, sparks and light
    /// (`docs/research/fire-colour.md`). `None` is the scene's.
    pub color: Option<Color>,
    /// Whether the kind's sparks are drawn, if it sheds any ([`FireKind::embers`]).
    pub embers: bool,
    /// Whether the kind's smoke is drawn, if it gives any ([`FireKind::smoke`]).
    pub smoke: bool,
    /// Whether a wide kind burns as several tongues ([`FireKind::tongues`]).
    pub tongues: bool,
    /// Whether a kind with a bed of coals draws one ([`FireKind::bed`]).
    pub bed: bool,
}

impl Default for Fire {
    /// A torch, sized from its kind, with everything the kind has.
    fn default() -> Self {
        Self::new(FireKind::Torch)
    }
}

impl Fire {
    /// A fire of `kind` with its own light, phase and extras and the scene's colour.
    pub const fn new(kind: FireKind) -> Self {
        Self {
            kind,
            lumens: None,
            phase: None,
            color: None,
            embers: true,
            smoke: true,
            tongues: true,
            bed: true,
        }
    }

    /// A fire of `kind` whose light puts `lux` on the ground at the edge of its [`reach`](FireKind::reach), with the light `over` metres above
    /// the ground ([`FireKind::lumens`]): `over` is the height of the flame's middle, so a fire standing on the ground is half its flame's
    /// height. `lux` is the host's own look number in its own exposure.
    pub fn lit(kind: FireKind, lux: f32, over: f32) -> Self {
        Self { lumens: Some(kind.lumens(lux, over)), ..Self::new(kind) }
    }

    /// What is unusable in these settings, field by field; empty when they are fine. See
    /// [`Flicker::problems`](crate::Flicker::problems). `lumens` and `phase` are reported only when set; the colour's channels are
    /// reported as the flame's are (`"color.red"`).
    pub fn problems(&self) -> Vec<crate::Problem> {
        use crate::model::problem::{finite, non_negative, positive};
        let mut out = Vec::new();
        if let Some(lumens) = self.lumens {
            positive(&mut out, "lumens", lumens);
        }
        if let Some(phase) = self.phase {
            finite(&mut out, "phase", phase);
        }
        if let Some(color) = self.color {
            let color = LinearRgba::from(color);
            non_negative(&mut out, "color.red", color.red);
            non_negative(&mut out, "color.green", color.green);
            non_negative(&mut out, "color.blue", color.blue);
        }
        out
    }

    /// The components this fire is, for the entity `entity`.
    fn parts(&self, entity: Entity) -> Parts {
        let kind = self.kind;
        let phase = self.phase.unwrap_or_else(|| fire_phase(entity.to_bits()));
        let lumens =
            self.lumens.unwrap_or_else(|| kind.lumens(FIRE_EDGE_LUX, flame_light_offset(kind.height())[1]));
        let flame_colour = self.color.map(|color| {
            let light = LinearRgba::from(color);
            let [red, green, blue] = flame_mid([light.red, light.green, light.blue]);
            Color::from(LinearRgba::new(red, green, blue, light.alpha))
        });
        Parts {
            site: FlameSite { phase, fire: Some(entity), color: flame_colour, ..kind.site() },
            candidate: PoolCandidate {
                phase: Some(phase),
                color: self.color,
                ..kind.candidate_at_mean(lumens)
            },
            embers: if self.embers { kind.embers() } else { None },
            smoke: if self.smoke { kind.smoke() } else { None },
            tongues: if self.tongues { kind.tongues() } else { None },
            bed: if self.bed { kind.bed() } else { None },
        }
    }
}

/// What a [`Fire`] is made of.
struct Parts {
    site: FlameSite,
    candidate: PoolCandidate,
    embers: Option<FlameEmbers>,
    smoke: Option<FlameSmoke>,
    tongues: Option<FlameTongues>,
    bed: Option<FuelBed>,
}

/// Writes the components of every [`Fire`] that has arrived or changed: see [`Fire`]'s "What the fire owns".
///
/// Add it with [`FlamePlugin`], [`FlickerPlugin`](crate::FlickerPlugin) and [`LightPoolPlugin`](crate::LightPoolPlugin); it writes the
/// components on the frame the `Fire` arrives, ahead of the pool, so the fire is lit on that frame. It never adds one of those plugins: a
/// host that added one afterwards would panic on the duplicate, so a missing one is reported in `finish`.
pub struct FirePlugin;

impl Plugin for FirePlugin {
    fn build(&self, app: &mut App) {
        crate::chain_sets(app);
        app.register_type::<Fire>().add_systems(Update, expand_fires.before(LightsSystems::Pool));
        crate::report::watch::<Fire>(app);
    }

    fn finish(&self, app: &mut App) {
        if !app.is_plugin_added::<FlamePlugin>() {
            error!("bevy_lights: FirePlugin needs FlamePlugin; a Fire will have a light and no flame drawn");
        }
        if !app.is_plugin_added::<crate::FlickerPlugin>() {
            error!("bevy_lights: FirePlugin needs FlickerPlugin; a Fire's light will not breathe");
        }
        if !app.world().contains_resource::<LightPool>() {
            error!("bevy_lights: FirePlugin needs LightPoolPlugin; a Fire will have a flame and no light");
        }
    }
}

fn expand_fires(mut commands: Commands, fires: Query<(Entity, Ref<Fire>), Changed<Fire>>) {
    for (entity, fire) in &fires {
        let Ok(mut entity) = commands.get_entity(entity) else { continue };
        let parts = fire.parts(entity.id());
        if fire.is_added() {
            // What the host spawned beside the fire is the host's.
            entity.insert_if_new((parts.site, parts.candidate));
            if let Some(embers) = parts.embers {
                entity.insert_if_new(embers);
            }
            if let Some(smoke) = parts.smoke {
                entity.insert_if_new(smoke);
            }
            if let Some(tongues) = parts.tongues {
                entity.insert_if_new(tongues);
            }
            if let Some(bed) = parts.bed {
                entity.insert_if_new(bed);
            }
        } else {
            entity.insert((parts.site, parts.candidate));
            match parts.embers {
                Some(embers) => entity.insert(embers),
                None => entity.remove::<FlameEmbers>(),
            };
            match parts.smoke {
                Some(smoke) => entity.insert(smoke),
                None => entity.remove::<FlameSmoke>(),
            };
            match parts.tongues {
                Some(tongues) => entity.insert(tongues),
                None => entity.remove::<FlameTongues>(),
            };
            match parts.bed {
                Some(bed) => entity.insert(bed),
                None => entity.remove::<FuelBed>(),
            };
        }
    }
}
