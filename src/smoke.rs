//! Smoke: what a fire gives off above its flame.
//!
//! Add [`FlameSmoke`] to a [`FlameSite`](crate::FlameSite) and its [`Flame`] gets one more child, a camera-facing additive quad
//! (`src/smoke.wesl`) holding a faint plume: a column that leaves the flame's tip, bends downwind and widens as a turbulent plume
//! does, whose wisps are lit by the fire and thin into the dark. There is no particle state: the column is a pure function of the
//! shared clock, so the host pays one draw per fire that asks and the smoke of a fire is the same on every machine.
//!
//! The quad is the flame's child, so it takes the fire's catch and the wind's lean from the flame's own instance scale: a fire being
//! lit smokes nothing until it has caught, a fire put out stops, and a wind bends the column further than it leans the flame. It has
//! the sparks' quad's geometry, so it carries the sparks' culling box ([`ember_cull_extents`](crate::ember_cull_extents)), kept in
//! step with the flame's scale.
//!
//! The smoke is **additive**: at night smoke is visible only by the light the fire puts into it, which is what an additive layer
//! draws, and an additive layer joins no sorting against the host's other transparent surfaces. It has no dark, occluding half; a
//! host that wants daylight smoke draws its own.
//!
//! The design and its sources are in `docs/research/flame-smoke.md`.

use bevy::asset::embedded_asset;
use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::NoAutoAabb;
use bevy::light::NotShadowCaster;
use bevy::math::Vec3A;
use bevy::mesh::{MeshTag, MeshVertexBufferLayoutRef};
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey, MeshMaterial3d};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError};
use bevy::shader::ShaderRef;
use bevy::time::Real;

use crate::flame::{Flame, FlameAssets, FlameLook, FlameSite, TintKey, TintedMaterials, scene_colour};
use crate::model::fire_colour::{SPARK_DULL_RED, SPARK_ORANGE};
use crate::model::{ember_cull_extents, pack_flame_tag, retint, set_flame_brightness, unpack_flame_tag};

/// The smoke shader, embedded in the binary by [`FlamePlugin`](crate::FlamePlugin).
const SMOKE_SHADER_PATH: &str = "embedded://bevy_lights/smoke.wesl";

/// The most density a fire's smoke can ask for: [`FlameSmoke::density`] above it draws at it.
pub const MAX_SMOKE_DENSITY: f32 = 1.0;

/// A fire's smoke: add it to a [`FlameSite`](crate::FlameSite) to give its flame a plume, remove it to take the plume away.
///
/// A site without one is drawn exactly as it always was: no extra entity, no extra draw. The smoke is the fire's own colour
/// ([`FlameSite::color`], or the scene's [`FlameLook::color`]), the sparks' orange moved to that hue ([`retint`](crate::retint)) and
/// pulled toward grey, so a green fire smokes a green-grey. Its strength is [`FlameLook::strength`] times the
/// [`density`](Self::density); both follow a changed look.
///
/// The smoke respects the site's [`depth_fade`](crate::FlameSite::depth_fade) (a fire whose source cannot be seen smokes nothing
/// that can) and its catch (a flame that has not caught smokes nothing). Its default density is a design default
/// (`docs/research/flame-smoke.md`); [`FireKind::smoke`](crate::FireKind::smoke) gives each kind its own.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
#[reflect(Component)]
pub struct FlameSmoke {
    /// How much smoke the fire gives off, `0.0..=`[`MAX_SMOKE_DENSITY`]: the brightness of the lit column. `0.0` gives none.
    pub density: f32,
}

impl Default for FlameSmoke {
    fn default() -> Self {
        Self { density: 0.8 }
    }
}

impl FlameSmoke {
    /// What is unusable in these settings, field by field; empty when they are fine. See
    /// [`Flicker::problems`](crate::Flicker::problems).
    pub fn problems(&self) -> Vec<crate::Problem> {
        let mut out = Vec::new();
        crate::model::problem::within(&mut out, "density", self.density, 0.0, MAX_SMOKE_DENSITY);
        out
    }

    /// The density as the share of [`MAX_SMOKE_DENSITY`] the mesh tag's low byte carries.
    fn share(&self) -> f32 {
        if self.density.is_finite() { (self.density / MAX_SMOKE_DENSITY).clamp(0.0, 1.0) } else { 0.0 }
    }
}

/// The smoke material: the fire's colour and clock, and the two colours the column passes through. Private: a host never builds one.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct SmokeMaterial {
    #[uniform(0)]
    base: LinearRgba,
    /// `x` seconds (wrapped hourly), `y` the tag's height scale, `z` 1 on the material of the sites that asked for the depth fade,
    /// `w` the strength.
    #[uniform(0)]
    params: Vec4,
    /// The column's colour just above the flame (`retint(SPARK_ORANGE, base)`).
    #[uniform(0)]
    warm: Vec4,
    /// The column's colour near its top (`retint(SPARK_DULL_RED, base)`).
    #[uniform(0)]
    cool: Vec4,
}

impl SmokeMaterial {
    /// The material of a fire of colour `mid` at `strength`; `depth_fade` as [`FlameSite::depth_fade`].
    fn new(mid: [f32; 3], strength: f32, max_height: f32, depth_fade: bool) -> Self {
        let mut material = Self {
            base: LinearRgba::NONE,
            params: Vec4::new(0.0, max_height, if depth_fade { 1.0 } else { 0.0 }, strength),
            warm: Vec4::ZERO,
            cool: Vec4::ZERO,
        };
        material.recolour(mid, strength);
        material
    }

    /// Writes the fire's colour and the two the column makes.
    fn recolour(&mut self, mid: [f32; 3], strength: f32) {
        let stop = |stop| {
            let [red, green, blue] = retint(stop, mid);
            Vec4::new(red, green, blue, 0.0)
        };
        self.base = LinearRgba { red: mid[0], green: mid[1], blue: mid[2], alpha: strength };
        self.warm = stop(SPARK_ORANGE);
        self.cool = stop(SPARK_DULL_RED);
    }
}

impl Material for SmokeMaterial {
    fn vertex_shader() -> ShaderRef {
        SMOKE_SHADER_PATH.into()
    }

    fn fragment_shader() -> ShaderRef {
        SMOKE_SHADER_PATH.into()
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

/// Registers the smoke's shader, material, assets and systems; [`FlamePlugin`](crate::FlamePlugin) calls it.
pub(crate) fn build(app: &mut App) {
    embedded_asset!(app, "smoke.wesl");
    app.add_plugins(bevy::pbr::MaterialPlugin::<SmokeMaterial>::default())
        .init_resource::<SmokeAssets>()
        .register_type::<FlameSmoke>()
        .add_systems(Startup, build_smoke_assets);
}

/// The quad, the scene's two materials (depth fade off and on) and one more for each colour a site asks for, which every smoke quad
/// shares.
#[derive(Resource, Default)]
pub(crate) struct SmokeAssets {
    mesh: Handle<Mesh>,
    material: Handle<SmokeMaterial>,
    faded_material: Handle<SmokeMaterial>,
    tinted: TintedMaterials<SmokeMaterial>,
    built: bool,
}

impl SmokeAssets {
    fn material_for(&self, depth_fade: bool) -> &Handle<SmokeMaterial> {
        if depth_fade { &self.faded_material } else { &self.material }
    }

    /// The id of the material a site of `color` smokes with, when it exists yet.
    fn material_id(&self, color: Option<Color>, depth_fade: bool) -> Option<AssetId<SmokeMaterial>> {
        match color {
            None => Some(self.material_for(depth_fade).id()),
            Some(color) => self.tinted.id(&TintKey::new(color, depth_fade)),
        }
    }

    /// The material a site of `color` smokes with, made if no quad has used it yet.
    fn material_of(
        &mut self,
        color: Option<Color>,
        depth_fade: bool,
        look: &FlameLook,
        materials: &mut Assets<SmokeMaterial>,
    ) -> Handle<SmokeMaterial> {
        match color {
            None => self.material_for(depth_fade).clone(),
            Some(color) => {
                let key = TintKey::new(color, depth_fade);
                self.tinted.handle(key, materials, || {
                    SmokeMaterial::new(key.rgb(), look.strength, look.max_height, depth_fade)
                })
            }
        }
    }
}

/// The smoke quad of a flame: a child of the [`Flame`], so it inherits the flame's catch and lean.
#[derive(Component)]
pub(crate) struct Smoke;

/// The marker that a [`Flame`] already has its smoke quad.
#[derive(Component)]
pub(crate) struct Smoked;

fn build_smoke_assets(
    mut assets: ResMut<SmokeAssets>,
    look: Res<FlameLook>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<SmokeMaterial>>,
) {
    assets.mesh = meshes.add(Rectangle::new(1.0, 1.0));
    assets.material =
        materials.add(SmokeMaterial::new(scene_colour(&look), look.strength, look.max_height, false));
    assets.faded_material =
        materials.add(SmokeMaterial::new(scene_colour(&look), look.strength, look.max_height, true));
    assets.built = true;
}

/// The half-extents of the culling box of a smoke quad whose flame stands at `scale`, the instance scale `update_flames` writes
/// (`x` the catch, `y` and `z` one more than the lean). The quad has the sparks' geometry, so its box is theirs
/// ([`ember_cull_extents`]), divided by the scale it inherits.
fn cull_box(height: f32, scale: Vec3) -> Vec3A {
    Vec3A::from(ember_cull_extents(height, scale.x, [scale.y - 1.0, scale.z - 1.0]))
}

/// A flame that has no smoke quad yet.
type UnsmokedFlame = (With<Flame>, Without<Smoked>);

/// Gives the flame of every [`FlameSmoke`] site one smoke quad, carrying the site's phase and height and, in the tag's brightness
/// byte, the density. The quad's culling box is the crate's own, for the reason the flame's is: the vertex stage draws a quad the
/// unit square's box does not hold, and the scale it inherits is data.
pub(crate) fn dress_smoke(
    mut commands: Commands,
    flames: Query<(Entity, &ChildOf, &Transform), UnsmokedFlame>,
    sites: Query<(&FlameSite, &FlameSmoke)>,
    mut smoke_assets: ResMut<SmokeAssets>,
    mut materials: ResMut<Assets<SmokeMaterial>>,
    look: Res<FlameLook>,
    flame_assets: Res<FlameAssets>,
) {
    if !smoke_assets.built || !flame_assets.built() {
        return;
    }
    for (flame, parent, at) in &flames {
        let Ok((site, smoke)) = sites.get(parent.parent()) else { continue };
        let packed = pack_flame_tag(site.phase, site.height, smoke.share(), flame_assets.max_height());
        let tag = MeshTag::new(packed);
        // The flame's scale as it stands: a flame dressed in mid-catch or in a wind is not at the identity.
        let height = unpack_flame_tag(packed, flame_assets.max_height()).1;
        let bound = Aabb { center: Vec3A::ZERO, half_extents: cull_box(height, at.scale) };
        let mesh = smoke_assets.mesh.clone();
        let material = smoke_assets.material_of(site.color, site.depth_fade, &look, &mut materials);
        commands.entity(flame).queue_silenced(move |mut flame: EntityWorldMut| {
            flame.insert(Smoked).with_children(|parent| {
                parent.spawn((
                    Name::new("Smoke"),
                    Smoke,
                    tag,
                    Mesh3d(mesh),
                    MeshMaterial3d(material),
                    Transform::IDENTITY,
                    NotShadowCaster,
                    // The unit square's box neither holds the column's quad nor survives the scale it inherits.
                    bound,
                    NoAutoAabb,
                ));
            });
        });
    }
}

/// What a smoke quad is read through: itself, its flame (its parent), its tag, its culling box and its material.
type SmokeQuad = (
    Entity,
    &'static ChildOf,
    &'static mut MeshTag,
    &'static mut Aabb,
    &'static mut MeshMaterial3d<SmokeMaterial>,
);

/// The flame and the site a smoke quad hangs on, and the height scale their tags are packed against.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct SmokeOwners<'w, 's> {
    flames: Query<'w, 's, (&'static ChildOf, &'static Transform), With<Flame>>,
    sites: Query<'w, 's, (&'static FlameSite, Option<&'static FlameSmoke>)>,
    assets: Res<'w, FlameAssets>,
}

impl SmokeOwners<'_, '_> {
    /// The site of `flame` and its smoke, and the instance scale `flame` stands at, when `flame` is a flame that hangs on a site.
    fn site_of(&self, flame: Entity) -> Option<(&FlameSite, Option<&FlameSmoke>, Vec3)> {
        let (parent, at) = self.flames.get(flame).ok()?;
        let (site, smoke) = self.sites.get(parent.parent()).ok()?;
        Some((site, smoke, at.scale))
    }

    /// The height scale a flame's tag is packed against ([`FlameLook::max_height`] at `Startup`).
    fn max_height(&self) -> f32 {
        self.assets.max_height()
    }
}

/// Follows each site's [`FlameSmoke`] and [`FlameSite::depth_fade`] with its quad, takes the quad away when the component goes,
/// keeps the quad's culling box in step with the flame's scale (it runs after `update_flames`, so the box is never a frame behind
/// the scale the frustum test multiplies it by), and ticks the shared clock and a changed [`FlameLook`].
pub(crate) fn update_smoke(
    mut commands: Commands,
    real: Res<Time<Real>>,
    look: Res<FlameLook>,
    mut smoke_assets: ResMut<SmokeAssets>,
    mut materials: ResMut<Assets<SmokeMaterial>>,
    mut quads: Query<SmokeQuad, With<Smoke>>,
    owners: SmokeOwners,
) {
    if quads.is_empty() {
        return;
    }
    let max_height = owners.max_height();
    for (entity, parent, mut tag, mut bound, mut material) in &mut quads {
        let Some((site, Some(wanted), scale)) = owners.site_of(parent.parent()) else {
            // The component is gone (or the flame is): the quad goes, and the flame may be dressed again.
            commands.entity(entity).despawn();
            commands.entity(parent.parent()).remove::<Smoked>();
            continue;
        };
        if smoke_assets.material_id(site.color, site.depth_fade) != Some(material.0.id()) {
            material.0 = smoke_assets.material_of(site.color, site.depth_fade, &look, &mut materials);
        }
        let packed = set_flame_brightness(tag.value, wanted.share());
        if packed != tag.value {
            tag.value = packed;
        }
        // Compared through `Deref`, so a quad whose flame did not change scale is not marked changed.
        let half_extents = cull_box(unpack_flame_tag(tag.value, max_height).1, scale);
        if bound.half_extents != half_extents {
            bound.half_extents = half_extents;
        }
    }
    if !smoke_assets.tinted.is_empty() {
        smoke_assets.tinted.sweep(&materials);
    }
    let elapsed = real.elapsed_secs_wrapped();
    let scene = scene_colour(&look);
    let scene_materials = [smoke_assets.material.id(), smoke_assets.faded_material.id()];
    for (id, is_scene) in scene_materials
        .into_iter()
        .map(|id| (id, true))
        .chain(smoke_assets.tinted.ids().map(|id| (id, false)))
    {
        let Some(mut made) = materials.get_mut(id) else { continue };
        made.params.x = elapsed;
        if look.is_changed() {
            if is_scene {
                made.recolour(scene, look.strength);
            } else {
                made.base.alpha = look.strength;
            }
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
    use crate::model::{FLICKER_DEPTH, ember_reach};

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(Assets::<Mesh>::default())
            .insert_resource(Assets::<crate::FlameMaterial>::default())
            .insert_resource(Assets::<SmokeMaterial>::default())
            .init_resource::<FlameLook>()
            .init_resource::<FlameAssets>()
            .init_resource::<SmokeAssets>()
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_secs_f64(1.0 / 60.0),
            ));
        app.add_systems(Startup, (build_flame_assets, build_smoke_assets))
            .add_systems(Update, (dress_flames, update_flames, dress_smoke, update_smoke).chain());
        app.update();
        app
    }

    fn site(height: f32, phase: f32) -> FlameSite {
        FlameSite {
            color: None,
            height,
            phase,
            kindle: 0.0,
            diameter: None,
            depth: FLICKER_DEPTH,
            depth_fade: false,
            fire: None,
        }
    }

    fn smoke_quads(app: &mut App) -> Vec<(Entity, u32)> {
        app.world_mut()
            .query_filtered::<(&ChildOf, &MeshTag), With<Smoke>>()
            .iter(app.world())
            .map(|(parent, tag)| (parent.parent(), tag.value))
            .collect()
    }

    /// The material a smoke quad is drawn with, copied out.
    fn smoke_of(app: &App, site: Entity) -> SmokeMaterial {
        let flame = app.world().get::<Children>(site).and_then(|c| c.first().copied()).expect("a flame");
        let quad = app.world().get::<Children>(flame).and_then(|c| c.first().copied()).expect("a smoke quad");
        let handle = app.world().get::<MeshMaterial3d<SmokeMaterial>>(quad).expect("a material").0.clone();
        app.world().resource::<Assets<SmokeMaterial>>().get(&handle).expect("it exists").clone()
    }

    #[test]
    fn only_a_site_that_asks_gets_a_smoke_quad_and_it_hangs_on_the_flame() {
        let mut app = app();
        let plain = app.world_mut().spawn((site(0.5, 0.0), Transform::default())).id();
        let smoking =
            app.world_mut().spawn((site(0.5, 1.0), FlameSmoke::default(), Transform::default())).id();
        app.update();
        app.update();
        let quads = smoke_quads(&mut app);
        assert_eq!(quads.len(), 1, "one site asked, so one quad, and a second frame does not twin it");
        let owner = app.world().get::<ChildOf>(quads[0].0).expect("the flame hangs on its site").parent();
        assert_eq!(owner, smoking, "the quad is the smoking site's flame's child");
        assert_ne!(owner, plain);
    }

    #[test]
    fn the_quads_tag_carries_the_sites_height_phase_and_the_density() {
        let mut app = app();
        app.world_mut().spawn((site(0.8, 2.0), FlameSmoke { density: 0.4 }, Transform::default()));
        app.update();
        app.update();
        let (_, tag) = smoke_quads(&mut app)[0];
        let (phase, height, share) = unpack_flame_tag(tag, 4.0);
        assert!((phase - 2.0).abs() < 0.03, "phase {phase}");
        assert!((height - 0.8).abs() < 1e-3, "height {height}");
        assert!((share - 0.4 / MAX_SMOKE_DENSITY).abs() < 0.005, "the density's share {share}");
    }

    #[test]
    fn a_changed_density_rewrites_the_tag_and_a_removed_component_takes_the_quad_away_and_can_return() {
        let mut app = app();
        let id =
            app.world_mut().spawn((site(0.5, 0.0), FlameSmoke { density: 0.2 }, Transform::default())).id();
        app.update();
        app.update();
        let before = smoke_quads(&mut app)[0].1;
        app.world_mut().get_mut::<FlameSmoke>(id).unwrap().density = 0.9;
        app.update();
        let after = smoke_quads(&mut app)[0].1;
        assert_ne!(before, after, "the density is in the tag");
        assert!((unpack_flame_tag(after, 4.0).2 - 0.9 / MAX_SMOKE_DENSITY).abs() < 0.005);

        app.world_mut().entity_mut(id).remove::<FlameSmoke>();
        app.update();
        app.update();
        assert!(smoke_quads(&mut app).is_empty(), "no component, no quad");

        app.world_mut().entity_mut(id).insert(FlameSmoke::default());
        app.update();
        app.update();
        assert_eq!(smoke_quads(&mut app).len(), 1, "added again, dressed again");
    }

    #[test]
    fn a_density_outside_its_range_is_reported_and_clamped_into_the_tag() {
        assert!(FlameSmoke { density: 0.6 }.problems().is_empty());
        assert_eq!(FlameSmoke { density: -0.1 }.problems().len(), 1);
        assert_eq!(FlameSmoke { density: MAX_SMOKE_DENSITY + 0.5 }.problems().len(), 1);
        assert_eq!(FlameSmoke { density: f32::NAN }.problems().len(), 1);
        assert_eq!(FlameSmoke { density: 9.0 }.share(), 1.0);
        assert_eq!(FlameSmoke { density: f32::NAN }.share(), 0.0);
    }

    /// A fire smokes in its own colour, and a scene that sets none smokes in the shipped one.
    #[test]
    fn a_fire_smokes_in_its_own_colour_and_follows_it_when_it_changes() {
        let mut app = app();
        let green = Color::from(LinearRgba::rgb(0.2, 1.0, 0.3));
        let tinted = app
            .world_mut()
            .spawn((FlameSite { color: Some(green), ..site(0.5, 0.0) }, FlameSmoke::default()))
            .id();
        let orange = app.world_mut().spawn((site(0.5, 1.0), FlameSmoke::default())).id();
        app.update();
        app.update();
        let stops = |m: &SmokeMaterial| [[m.warm.x, m.warm.y, m.warm.z], [m.cool.x, m.cool.y, m.cool.z]];
        assert_eq!(
            stops(&smoke_of(&app, orange)),
            [SPARK_ORANGE, SPARK_DULL_RED],
            "the scene's smoke is lit by the shipped fire"
        );
        for stop in stops(&smoke_of(&app, tinted)) {
            assert!(
                stop[1] > stop[0] && stop[1] > stop[2],
                "a green fire's smoke is green at both ends: {stop:?}"
            );
        }
        // A colour the host changes later reaches the smoke too.
        app.world_mut().get_mut::<FlameSite>(tinted).expect("the site").color =
            Some(Color::from(LinearRgba::rgb(0.1, 0.3, 1.0)));
        app.update();
        let [warm, _] = stops(&smoke_of(&app, tinted));
        assert!(warm[2] > warm[1] && warm[1] > warm[0], "now blue: {warm:?}");
    }

    /// The smoke joins no sorting: it only adds the light the fire puts into it.
    #[test]
    fn the_smoke_is_additive_so_it_joins_no_sorting() {
        let material = SmokeMaterial::new([1.0, 0.55, 0.18], 1.0, 4.0, false);
        assert!(matches!(material.alpha_mode(), AlphaMode::Add));
    }

    /// Points on the smoke quad, a 5 x 5 grid of its corners and interior, where `src/smoke.wesl`'s vertex stage puts them: the same
    /// constants, the same extents (the quad widens downwind by the lean at the top of the column), for a camera at `eye` whose right
    /// axis is `camera_right`. `origin` is the flame's foot, `size` the height it is drawn at, `wind` the instance scale's `y` and `z`
    /// less one.
    fn drawn_points(origin: Vec3, size: f32, wind: Vec2, eye: Vec3, camera_right: Vec3) -> Vec<Vec3> {
        const SOURCE_HEIGHT: f32 = 0.35;
        const HALF_WIDTH: f32 = 0.7;
        const RISE_MAX: f32 = 2.6;
        const MARGIN: f32 = 0.35;
        const LEAN_TOP: f32 = 3.9;
        let right = Vec3::new(camera_right.x, 0.0, camera_right.z).normalize();
        let centre = origin + Vec3::Y * (size * SOURCE_HEIGHT);
        let to_eye = eye - centre;
        let sin_e = (to_eye.y / to_eye.length().max(1e-4)).clamp(0.0, 1.0);
        let cos_e = (1.0 - sin_e * sin_e).max(0.0).sqrt();
        let reach = LEAN_TOP * wind.dot(Vec2::new(right.x, right.z));
        let (x_low, x_high) = (-HALF_WIDTH + reach.min(0.0), HALF_WIDTH + reach.max(0.0));
        let (v_low, v_high) = (-MARGIN, RISE_MAX * cos_e + MARGIN);
        let screen_up = to_eye.normalize().cross(right).normalize();
        let mut points = Vec::new();
        for i in 0..5 {
            for j in 0..5 {
                let (u, v) = (i as f32 / 4.0, j as f32 / 4.0);
                let across = x_low + (x_high - x_low) * u;
                let climb = v_low + (v_high - v_low) * v;
                points.push(origin + right * (across * size) + screen_up * (climb * size));
            }
        }
        points
    }

    #[test]
    fn a_smoke_quad_any_part_of_which_is_on_screen_is_never_culled() {
        // The quad the vertex stage draws (`drawn_points`), the box and the scale this crate gives it, and Bevy's own frustum
        // test: a fire whose smoke is culled while any of its quad is inside the frame loses its plume at the screen's edge.
        crate::flame::culling_tests::assert_a_quad_on_screen_is_never_culled(
            ember_cull_extents,
            ember_reach,
            drawn_points,
        );
    }

    /// The smoke quad's box as the frustum test sees it, with the scale it inherits from its flame: the half-extents in the
    /// quad's own axes, the flame's scale, and the box the test builds from the two.
    fn world_box(app: &mut App) -> Option<(Vec3, Vec3)> {
        let (flame, bound) =
            app.world_mut().query_filtered::<(&ChildOf, &Aabb), With<Smoke>>().iter(app.world()).next()?;
        let scale = app.world().get::<Transform>(flame.parent())?.scale;
        Some((scale, Vec3::from(bound.half_extents) * scale))
    }

    #[test]
    fn a_smoke_quads_box_keeps_its_size_through_the_catch_and_the_lean_on_every_frame() {
        // The quad inherits the flame's scale, which is the shader's data (its catch and its lean), and the frustum test
        // multiplies the box by it. The box must have been divided by it first, in the same frame the scale was written, or a
        // gusting fire's smoke is culled on screen for the frame the box lags.
        let mut app = app();
        app.insert_resource(crate::FlameWind { velocity: Vec2::new(3.0, -1.5) });
        let height = 0.9;
        app.world_mut().spawn((
            FlameSite { kindle: crate::KINDLE_SECONDS, ..site(height, 0.0) },
            FlameSmoke::default(),
            Transform::default(),
        ));
        let (mut checked, mut seen_small) = (0, false);
        for frame in 0..(crate::KINDLE_SECONDS * 60.0) as usize + 6 {
            app.update();
            let Some((scale, world)) = world_box(&mut app) else { continue };
            checked += 1;
            // The shader clamps the catch to 1, so the drawn size does too; the box's `x` is divided by the clamped catch
            // and multiplied by the real scale, and may exceed the cube by exactly the overshoot, never fall short of it.
            let size = height * scale.x.min(1.0).powf(2.0 / 3.0);
            let want = Vec3::splat(ember_reach([scale.y - 1.0, scale.z - 1.0]) * size);
            let slack = 1e-4 * (1.0 + want.max_element());
            assert!(
                (world.y - want.y).abs() < slack
                    && (world.z - want.z).abs() < slack
                    && world.x >= want.x - slack
                    && world.x <= want.x * scale.x.max(1.0) + slack,
                "frame {frame}: scale {scale}, the box measures {world} and the quad wants {want}"
            );
            seen_small |= scale.x < 0.3 && (scale.y - 1.0).abs() > 0.1;
        }
        assert!(checked > 30, "only {checked} frames had a smoke quad");
        assert!(
            seen_small,
            "the run never had a small, leaning flame, so it did not test the catch and the lean together"
        );
    }

    #[test]
    fn a_smoke_quad_carries_a_box_of_its_own_and_is_not_left_to_the_units_quad() {
        let mut app = app();
        app.world_mut().spawn((site(0.5, 0.0), FlameSmoke::default(), Transform::default()));
        app.update();
        app.update();
        let quad = app.world_mut().query_filtered::<Entity, With<Smoke>>().single(app.world()).unwrap();
        let entity = app.world().entity(quad);
        assert!(entity.contains::<Aabb>(), "a smoke quad has a culling box");
        assert!(entity.contains::<NoAutoAabb>(), "which Bevy is told not to recompute");
        assert!(
            !entity.contains::<bevy::camera::visibility::NoFrustumCulling>(),
            "and which the frustum test uses: an off-screen fire's smoke is not drawn"
        );
    }
}
