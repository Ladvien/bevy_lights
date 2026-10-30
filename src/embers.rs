//! Embers: the sparks a fire throws up.
//!
//! Add [`FlameEmbers`] to a [`FlameSite`](crate::FlameSite) and its [`Flame`] gets one more child, a camera-facing additive quad
//! (`src/embers.wesl`) on which sparks leave the flame, climb the plume under drag, weave and cool from yellow through red to
//! nothing. There is no particle state: every spark is a pure function of the shared clock and an integer hash of its slot and
//! cycle, so the host pays one draw per fire and the sparks of a fire are the same on every machine.
//!
//! The quad is the flame's child, so it takes the fire's catch and the wind's lean from the flame's own instance scale: a fire
//! being lit sheds nothing until it has caught, a fire put out stops, and a wind blows the sparks further than it leans the flame.
//! The design and its sources are in `docs/research/flame-embers.md`.

use bevy::asset::embedded_asset;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::light::NotShadowCaster;
use bevy::mesh::{MeshTag, MeshVertexBufferLayoutRef};
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey, MeshMaterial3d};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError};
use bevy::shader::ShaderRef;
use bevy::time::Real;

use crate::flame::{Flame, FlameAssets, FlameLook, FlameSite};
use crate::model::{pack_flame_tag, set_flame_brightness, unpack_flame_tag};

/// The embers shader, embedded in the binary by [`FlamePlugin`](crate::FlamePlugin).
const EMBER_SHADER_PATH: &str = "embedded://bevy_lights/embers.wesl";

/// The most sparks a second one fire sheds: the shader draws sixteen slots and a spark lives up to 1.8 s, so eight a second is the
/// rate at which every slot is always busy. [`FlameEmbers::rate`] above it draws at it.
pub const MAX_EMBER_RATE: f32 = 8.0;

/// Sparks leaving a fire: add it to a [`FlameSite`](crate::FlameSite) to give its flame embers, remove it to take them away.
///
/// A site without one is drawn exactly as it always was: no extra entity, no extra draw. The sparks' colour is the flame's
/// ([`FlameLook::color`]) pulled towards the hot end of its ramp when they are new and towards a dull red as they cool, and their
/// strength is [`FlameLook::strength`]; both follow a changed look.
///
/// The sparks respect the site's [`depth_fade`](crate::FlameSite::depth_fade) (a fire whose source cannot be seen sheds none that
/// can) and its catch (a flame that has not caught sheds none). The default is a hearth's few sparks a second, a design default
/// (`docs/research/flame-embers.md`).
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
#[reflect(Component)]
pub struct FlameEmbers {
    /// How many sparks the fire sheds a second, `0.0..=`[`MAX_EMBER_RATE`]. `0.0` sheds none.
    pub rate: f32,
}

impl Default for FlameEmbers {
    fn default() -> Self {
        Self { rate: 3.0 }
    }
}

impl FlameEmbers {
    /// What is unusable in these settings, field by field; empty when they are fine. See
    /// [`Flicker::problems`](crate::Flicker::problems).
    pub fn problems(&self) -> Vec<crate::Problem> {
        let mut out = Vec::new();
        crate::model::problem::within(&mut out, "rate", self.rate, 0.0, MAX_EMBER_RATE);
        out
    }

    /// The rate as the share of [`MAX_EMBER_RATE`] the mesh tag's low byte carries.
    fn share(&self) -> f32 {
        if self.rate.is_finite() { (self.rate / MAX_EMBER_RATE).clamp(0.0, 1.0) } else { 0.0 }
    }
}

/// The embers material: the same uniforms as [`FlameMaterial`](crate::FlameMaterial), because the sparks are the fire's colour and
/// clock. Private: a host never builds one.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct EmberMaterial {
    #[uniform(0)]
    base: LinearRgba,
    /// `x` seconds (wrapped hourly), `y` the tag's height scale, `z` 1 on the material of the sites that asked for the depth
    /// fade, `w` the strength.
    #[uniform(0)]
    params: Vec4,
}

impl Material for EmberMaterial {
    fn vertex_shader() -> ShaderRef {
        EMBER_SHADER_PATH.into()
    }

    fn fragment_shader() -> ShaderRef {
        EMBER_SHADER_PATH.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Add
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if let Some(depth) = descriptor.depth_stencil.as_mut() {
            depth.depth_write_enabled = Some(false);
        }
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

/// Registers the embers' shader, material, assets and systems; [`FlamePlugin`](crate::FlamePlugin) calls it.
pub(crate) fn build(app: &mut App) {
    embedded_asset!(app, "embers.wesl");
    app.add_plugins(bevy::pbr::MaterialPlugin::<EmberMaterial>::default())
        .init_resource::<EmberAssets>()
        .register_type::<FlameEmbers>()
        .add_systems(Startup, build_ember_assets);
}

/// The quad and the two materials (depth fade off and on) every ember quad shares.
#[derive(Resource, Default)]
struct EmberAssets {
    mesh: Handle<Mesh>,
    material: Handle<EmberMaterial>,
    faded_material: Handle<EmberMaterial>,
    built: bool,
}

impl EmberAssets {
    fn material_for(&self, depth_fade: bool) -> &Handle<EmberMaterial> {
        if depth_fade { &self.faded_material } else { &self.material }
    }
}

/// The ember quad of a flame: a child of the [`Flame`], so it inherits the flame's catch and lean.
#[derive(Component)]
struct Ember;

/// The marker that a [`Flame`] already has its ember quad.
#[derive(Component)]
struct Embered;

fn material_for(look: FlameLook, depth_fade: bool) -> EmberMaterial {
    EmberMaterial {
        base: LinearRgba {
            red: look.color.red,
            green: look.color.green.min(1.0),
            blue: look.color.blue,
            alpha: look.strength,
        },
        params: Vec4::new(0.0, look.max_height, if depth_fade { 1.0 } else { 0.0 }, look.strength),
    }
}

fn build_ember_assets(
    mut assets: ResMut<EmberAssets>,
    look: Res<FlameLook>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<EmberMaterial>>,
) {
    assets.mesh = meshes.add(Rectangle::new(1.0, 1.0));
    assets.material = materials.add(material_for(*look, false));
    assets.faded_material = materials.add(material_for(*look, true));
    assets.built = true;
}

/// Gives the flame of every [`FlameEmbers`] site one ember quad, carrying the site's phase and height and, in the tag's brightness
/// byte, the rate.
pub(crate) fn dress_embers(
    mut commands: Commands,
    flames: Query<(Entity, &ChildOf), (With<Flame>, Without<Embered>)>,
    sites: Query<(&FlameSite, &FlameEmbers)>,
    ember_assets: Res<EmberAssets>,
    flame_assets: Res<FlameAssets>,
) {
    if !ember_assets.built || !flame_assets.built() {
        return;
    }
    for (flame, parent) in &flames {
        let Ok((site, embers)) = sites.get(parent.parent()) else { continue };
        let tag =
            MeshTag::new(pack_flame_tag(site.phase, site.height, embers.share(), flame_assets.max_height()));
        let mesh = ember_assets.mesh.clone();
        let material = ember_assets.material_for(site.depth_fade).clone();
        commands.entity(flame).queue_silenced(move |mut flame: EntityWorldMut| {
            flame.insert(Embered).with_children(|parent| {
                parent.spawn((
                    Name::new("Embers"),
                    Ember,
                    tag,
                    Mesh3d(mesh),
                    MeshMaterial3d(material),
                    Transform::IDENTITY,
                    NotShadowCaster,
                    NoFrustumCulling,
                ));
            });
        });
    }
}

/// Follows each site's [`FlameEmbers`] and [`FlameSite::depth_fade`] with its quad, takes the quad away when the component goes, and
/// ticks the shared clock and a changed [`FlameLook`].
pub(crate) fn update_embers(
    mut commands: Commands,
    real: Res<Time<Real>>,
    look: Res<FlameLook>,
    ember_assets: Res<EmberAssets>,
    mut materials: ResMut<Assets<EmberMaterial>>,
    mut embers: Query<(Entity, &ChildOf, &mut MeshTag, &mut MeshMaterial3d<EmberMaterial>), With<Ember>>,
    flames: Query<&ChildOf, With<Flame>>,
    sites: Query<(&FlameSite, Option<&FlameEmbers>)>,
) {
    if embers.is_empty() {
        return;
    }
    for (entity, parent, mut tag, mut material) in &mut embers {
        let site = flames.get(parent.parent()).ok().and_then(|flame| sites.get(flame.parent()).ok());
        let Some((site, Some(wanted))) = site else {
            // The component is gone (or the flame is): the quad goes, and the flame may be dressed again.
            commands.entity(entity).despawn();
            commands.entity(parent.parent()).remove::<Embered>();
            continue;
        };
        let handle = ember_assets.material_for(site.depth_fade);
        if material.0 != *handle {
            material.0 = handle.clone();
        }
        let packed = set_flame_brightness(tag.value, wanted.share());
        if packed != tag.value {
            tag.value = packed;
        }
    }
    let elapsed = real.elapsed_secs_wrapped();
    for handle in [&ember_assets.material, &ember_assets.faded_material] {
        let Some(mut made) = materials.get_mut(handle) else { continue };
        made.params.x = elapsed;
        if look.is_changed() {
            made.base = LinearRgba {
                red: look.color.red,
                green: look.color.green.min(1.0),
                blue: look.color.blue,
                alpha: look.strength,
            };
            made.params.y = look.max_height;
            made.params.w = look.strength;
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::flame::{build_flame_assets, dress_flames, update_flames};
    use crate::model::FLICKER_DEPTH;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(Assets::<Mesh>::default())
            .insert_resource(Assets::<crate::FlameMaterial>::default())
            .insert_resource(Assets::<EmberMaterial>::default())
            .init_resource::<FlameLook>()
            .init_resource::<FlameAssets>()
            .init_resource::<EmberAssets>();
        app.add_systems(Startup, (build_flame_assets, build_ember_assets))
            .add_systems(Update, (dress_flames, update_flames, dress_embers, update_embers).chain());
        app.update();
        app
    }

    fn site(height: f32, phase: f32) -> FlameSite {
        FlameSite {
            height,
            phase,
            kindle: 0.0,
            diameter: None,
            depth: FLICKER_DEPTH,
            depth_fade: false,
            fire: None,
        }
    }

    fn ember_quads(app: &mut App) -> Vec<(Entity, u32)> {
        app.world_mut()
            .query_filtered::<(&ChildOf, &MeshTag), With<Ember>>()
            .iter(app.world())
            .map(|(parent, tag)| (parent.parent(), tag.value))
            .collect()
    }

    #[test]
    fn only_a_site_that_asks_gets_an_ember_quad_and_it_hangs_on_the_flame() {
        let mut app = app();
        let plain = app.world_mut().spawn((site(0.5, 0.0), Transform::default())).id();
        let lit = app.world_mut().spawn((site(0.5, 1.0), FlameEmbers::default(), Transform::default())).id();
        app.update();
        app.update();
        let quads = ember_quads(&mut app);
        assert_eq!(quads.len(), 1, "one site asked, so one quad, and a second frame does not twin it");
        let flame = quads[0].0;
        let owner = app.world().get::<ChildOf>(flame).expect("the flame hangs on its site").parent();
        assert_eq!(owner, lit, "the quad is the lit site's flame's child");
        assert_ne!(owner, plain);
    }

    #[test]
    fn the_quads_tag_carries_the_sites_height_phase_and_the_rate() {
        let mut app = app();
        app.world_mut().spawn((site(0.8, 2.0), FlameEmbers { rate: 4.0 }, Transform::default()));
        app.update();
        app.update();
        let (_, tag) = ember_quads(&mut app)[0];
        let (phase, height, share) = unpack_flame_tag(tag, 4.0);
        assert!((phase - 2.0).abs() < 0.03, "phase {phase}");
        assert!((height - 0.8).abs() < 1e-3, "height {height}");
        assert!((share - 4.0 / MAX_EMBER_RATE).abs() < 0.005, "the rate's share {share}");
    }

    #[test]
    fn a_changed_rate_rewrites_the_tag_and_a_removed_component_takes_the_quad_away_and_can_return() {
        let mut app = app();
        let id =
            app.world_mut().spawn((site(0.5, 0.0), FlameEmbers { rate: 2.0 }, Transform::default())).id();
        app.update();
        app.update();
        let before = ember_quads(&mut app)[0].1;
        app.world_mut().get_mut::<FlameEmbers>(id).unwrap().rate = 6.0;
        app.update();
        let after = ember_quads(&mut app)[0].1;
        assert_ne!(before, after, "the rate is in the tag");
        assert!((unpack_flame_tag(after, 4.0).2 - 6.0 / MAX_EMBER_RATE).abs() < 0.005);

        app.world_mut().entity_mut(id).remove::<FlameEmbers>();
        app.update();
        app.update();
        assert!(ember_quads(&mut app).is_empty(), "no component, no quad");

        app.world_mut().entity_mut(id).insert(FlameEmbers::default());
        app.update();
        app.update();
        assert_eq!(ember_quads(&mut app).len(), 1, "added again, dressed again");
    }

    #[test]
    fn a_rate_outside_its_range_is_reported_and_clamped_into_the_tag() {
        assert!(FlameEmbers { rate: 3.0 }.problems().is_empty());
        assert_eq!(FlameEmbers { rate: -1.0 }.problems().len(), 1);
        assert_eq!(FlameEmbers { rate: MAX_EMBER_RATE + 1.0 }.problems().len(), 1);
        assert_eq!(FlameEmbers { rate: f32::NAN }.problems().len(), 1);
        assert_eq!(FlameEmbers { rate: 99.0 }.share(), 1.0);
        assert_eq!(FlameEmbers { rate: f32::NAN }.share(), 0.0);
    }
}
