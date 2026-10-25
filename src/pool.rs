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

use crate::flicker::{Flicker, FlickerPlugin};
use crate::{LightsSystems, chain_sets};

/// [`LightPoolPlugin`]: keeps [`LightPool::size`] real lights and hands them to the nearest
/// [`PoolCandidate`]s.
///
/// Generic over `F`, the host's focus marker: exactly one entity carrying `F` is the point the pool
/// measures distance from. Add it after [`FlickerPlugin`] — the pool's lights are [`Flicker`]
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
            .add_systems(Update, assign_pool::<F>.in_set(LightsSystems::Pool));
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
/// Not `Serialize`: `PointLight` itself has no serde derive (Bevy ships `serialize` for `Color`, and
/// a light's intensity is a runtime number this crate writes every frame anyway). The template's
/// fields are what a save would need, and a host that wants them saved saves its own budget.
#[derive(Resource, Clone, Debug, Reflect)]
#[reflect(Resource)]
pub struct LightPool {
    /// How many real lights exist. Shrinking it despawns the extra lights on the next frame; growing
    /// it spawns more.
    pub size: usize,
    /// How far from the focus a candidate may be, in world units, to be given a light.
    pub reach: f32,
    /// A dimmer shared by the whole pool, usually how lit the night is (0 puts every pool light
    /// out). Applied on top of each candidate's own intensity.
    pub gain: f32,
    /// The light every slot is made from: its `color`, `range`, `radius` and `shadow_maps_enabled`
    /// are copied onto every pool light, and its `intensity` is ignored (each slot's is its
    /// candidate's, times [`LightPool::gain`]).
    pub light: PointLight,
}

impl Default for LightPool {
    fn default() -> Self {
        Self {
            size: 16,
            reach: 60.0,
            gain: 1.0,
            light: PointLight { intensity: 0.0, range: 10.0, shadow_maps_enabled: false, ..default() },
        }
    }
}

/// A prop that would like one of the pool's real lights: how bright it burns, and where its light
/// sits relative to the prop's origin.
///
/// Remove the component to take the prop out of the pool (a torch that has been doused, a lamp that
/// has a real light of its own). Re-insert it to put it back.
///
/// A candidate whose own [`Visibility`] is [`Visibility::Hidden`] is skipped — a torch put away for
/// the day is not a light — and so is one further from the focus than [`LightPool::reach`]. A
/// candidate with no `Visibility` at all is not hidden: a host may pool a light on an entity that has
/// no mesh.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[reflect(Component)]
pub struct PoolCandidate {
    /// Where the light goes, in **world** axes, relative to the candidate's own translation. It is
    /// not rotated with the candidate: a torch swung by a walking arm still lights upward from its
    /// head.
    pub offset: Vec3,
    /// The light's intensity at [`LightPool::gain`] 1, in lumens.
    pub intensity: f32,
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
    pub slot: usize,
}

/// One candidate's light assignment: its distance from the focus (squared), its entity, its world
/// position and its intensity.
type Assignment = (f32, Entity, Vec3, f32);

/// Spawns, resizes and assigns the pool's real lights.
///
/// One frame, in order:
///
/// 1. **Slots.** Spawns the missing entities so slots `0..size` exist, and despawns those past
///    `size`. When [`LightPool`] changed, the shared fields of its `light` are copied onto every
///    slot; each slot's intensity is left to the assignment.
/// 2. **Focus.** The one entity carrying `F`. None, or more than one, means no assignment this frame
///    (and a warning for the second case): a pool with two cameras has no single near.
/// 3. **Candidates.** Every [`PoolCandidate`] whose own [`Visibility`] is not `Hidden`, within
///    [`LightPool::reach`], sorted by distance and then by [`Entity`], so the assignment is the same
///    on every machine and two props at one distance never swap between frames.
/// 4. **Assign.** Slot `k` takes candidate `k` at the candidate's position plus its
///    [`PoolCandidate::offset`], with `Flicker::base = intensity · gain`. Every slot with no
///    candidate is hidden, and the whole pool is hidden when there is no focus or the gain is zero.
fn assign_pool<F: Component>(
    mut commands: Commands,
    pool: Res<LightPool>,
    focus: Query<&GlobalTransform, With<F>>,
    candidates: Query<(Entity, &GlobalTransform, Option<&Visibility>, &PoolCandidate), Without<PoolLight>>,
    mut slots: Query<(Entity, &PoolLight, &mut Transform, &mut Visibility, &mut Flicker, &mut PointLight)>,
) {
    let existing: Vec<(Entity, usize)> =
        slots.iter().map(|(entity, light, ..)| (entity, light.slot)).collect();

    // 1. Slots: one entity per slot, no more and no fewer.
    for slot in 0..pool.size {
        if !existing.iter().any(|(_, at)| *at == slot) {
            commands.spawn((
                Name::new(format!("Pool light {slot}")),
                PoolLight { slot },
                PointLight { intensity: 0.0, ..pool.light },
                Flicker { base: 0.0, phase: 1.3 * slot as f32 },
                Transform::default(),
                Visibility::Hidden,
            ));
        }
    }
    for (entity, slot) in &existing {
        if *slot >= pool.size {
            commands.entity(*entity).despawn();
        }
    }

    // The shared light settings follow the resource, so a host can recolour or re-range the whole
    // pool in one write. A slot's own intensity is the assignment's, never the resource's.
    if pool.is_changed() {
        for (_, _, _, _, _, mut light) in &mut slots {
            let wanted = &pool.light;
            if light.color != wanted.color
                || light.range != wanted.range
                || light.radius != wanted.radius
                || light.shadow_maps_enabled != wanted.shadow_maps_enabled
            {
                light.color = wanted.color;
                light.range = wanted.range;
                light.radius = wanted.radius;
                light.shadow_maps_enabled = wanted.shadow_maps_enabled;
            }
        }
    }

    // 2. Focus: exactly one entity, or none.
    let focus_at = {
        let mut found = focus.iter();
        match (found.next(), found.next()) {
            (Some(at), None) => Some(at.translation()),
            (Some(_), Some(_)) => {
                let count = 2 + found.count();
                warn_once!(
                    "bevy_lights: LightPoolPlugin found {count} focus entities; the pool needs exactly one"
                );
                None
            }
            _ => None,
        }
    };

    // 3. Candidates: visible, in reach, nearest first, ties by entity.
    let mut lit: Vec<Assignment> = match focus_at {
        Some(focus_at) if pool.gain > 0.0 => candidates
            .iter()
            // Not `Visibility::Hidden`: a candidate with no `Visibility` at all is not hidden.
            .filter(|(_, _, visibility, _)| !visibility.is_some_and(|v| *v == Visibility::Hidden))
            .map(|(entity, at, _, candidate)| {
                let position = at.translation();
                (
                    position.distance_squared(focus_at),
                    entity,
                    position + candidate.offset,
                    candidate.intensity,
                )
            })
            .filter(|(distance_squared, ..)| *distance_squared <= pool.reach * pool.reach)
            .collect(),
        _ => Vec::new(),
    };
    lit.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.to_bits().cmp(&b.1.to_bits())));

    // 4. Assign slot k to candidate k; hide the rest.
    for (_, slot, mut transform, mut visibility, mut flicker, _) in &mut slots {
        match lit.get(slot.slot) {
            Some(&(_, _, at, intensity)) => {
                transform.translation = at;
                flicker.base = intensity * pool.gain;
                visibility.set_if_neq(Visibility::Inherited);
            }
            None => {
                flicker.base = 0.0;
                visibility.set_if_neq(Visibility::Hidden);
            }
        }
    }
}
