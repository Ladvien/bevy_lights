//! Embers: the sparks a fire throws up.
//!
//! Add [`FlameEmbers`] to a [`FlameSite`](crate::FlameSite) and its [`Flame`] gets one more child, a camera-facing additive quad
//! (`src/embers.wesl`) on which sparks leave the flame, climb the plume under drag, weave and cool from yellow through red to
//! nothing. There is no particle state: every spark is a pure function of the shared clock and an integer hash of its slot and
//! cycle, so the host pays one draw per fire and the sparks of a fire are the same on every machine.
//!
//! The quad is the flame's child, so it takes the fire's catch and the wind's lean from the flame's own instance scale: a fire
//! being lit sheds nothing until it has caught, a fire put out stops, and a wind blows the sparks further than it leans the flame.
//! A fire whose sparks are off the screen draws none of them: the quad carries a culling box of its own
//! ([`ember_cull_extents`](crate::ember_cull_extents)), wide enough for the quad's downwind reach and kept in step with the
//! flame's scale.
//!
//! The design and its sources are in `docs/research/flame-embers.md` and `docs/research/flame-culling.md`.

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
use crate::model::fire_colour::{CORE_YELLOW, SPARK_DULL_RED, SPARK_ORANGE};
use crate::model::{ember_cull_extents, pack_flame_tag, retint, set_flame_brightness, unpack_flame_tag};

/// The embers shader, embedded in the binary by [`FlamePlugin`](crate::FlamePlugin).
const EMBER_SHADER_PATH: &str = "embedded://bevy_lights/embers.wesl";

/// The most sparks a second one fire sheds: the shader draws sixteen slots and a spark lives up to 1.8 s, so eight a second is the
/// rate at which every slot is always busy. [`FlameEmbers::rate`] above it draws at it.
pub const MAX_EMBER_RATE: f32 = 8.0;

/// Sparks leaving a fire: add it to a [`FlameSite`](crate::FlameSite) to give its flame embers, remove it to take them away.
///
/// A site without one is drawn exactly as it always was: no extra entity, no extra draw. The sparks are the fire's own colour
/// ([`FlameSite::color`], or the scene's [`FlameLook::color`]): the shipped orange sparks, yellow when new and a dull red as they
/// cool, moved to that hue ([`retint`](crate::retint)), so a green fire sheds green sparks. Their strength is
/// [`FlameLook::strength`]; both follow a changed look.
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
/// clock, and the three colours a spark passes through. Private: a host never builds one.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct EmberMaterial {
    #[uniform(0)]
    base: LinearRgba,
    /// `x` seconds (wrapped hourly), `y` the tag's height scale, `z` 1 on the material of the sites that asked for the depth
    /// fade, `w` the strength.
    #[uniform(0)]
    params: Vec4,
    /// A spark's colour when it has almost cooled (`retint(SPARK_DULL_RED, base)`).
    #[uniform(0)]
    dull: Vec4,
    /// A spark's colour in the middle of its life (`retint(SPARK_ORANGE, base)`).
    #[uniform(0)]
    mild: Vec4,
    /// A spark's colour at its hottest (`retint(CORE_YELLOW, base)`).
    #[uniform(0)]
    hot: Vec4,
}

impl EmberMaterial {
    /// The material of a fire of colour `mid` at `strength`; `depth_fade` as [`FlameSite::depth_fade`].
    fn new(mid: [f32; 3], strength: f32, max_height: f32, depth_fade: bool) -> Self {
        let mut material = Self {
            base: LinearRgba::NONE,
            params: Vec4::new(0.0, max_height, if depth_fade { 1.0 } else { 0.0 }, strength),
            dull: Vec4::ZERO,
            mild: Vec4::ZERO,
            hot: Vec4::ZERO,
        };
        material.recolour(mid, strength);
        material
    }

    /// Writes the fire's colour and the sparks' three colours it makes.
    fn recolour(&mut self, mid: [f32; 3], strength: f32) {
        let stop = |stop| {
            let [red, green, blue] = retint(stop, mid);
            Vec4::new(red, green, blue, 0.0)
        };
        self.base = LinearRgba { red: mid[0], green: mid[1], blue: mid[2], alpha: strength };
        self.dull = stop(SPARK_DULL_RED);
        self.mild = stop(SPARK_ORANGE);
        self.hot = stop(CORE_YELLOW);
    }
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

/// The quad, the scene's two materials (depth fade off and on) and one more for each colour a site asks for, which every ember
/// quad shares.
#[derive(Resource, Default)]
pub(crate) struct EmberAssets {
    mesh: Handle<Mesh>,
    material: Handle<EmberMaterial>,
    faded_material: Handle<EmberMaterial>,
    tinted: TintedMaterials<EmberMaterial>,
    built: bool,
}

impl EmberAssets {
    fn material_for(&self, depth_fade: bool) -> &Handle<EmberMaterial> {
        if depth_fade { &self.faded_material } else { &self.material }
    }

    /// The id of the material a site of `color` sheds its sparks with, when it exists yet.
    fn material_id(&self, color: Option<Color>, depth_fade: bool) -> Option<AssetId<EmberMaterial>> {
        match color {
            None => Some(self.material_for(depth_fade).id()),
            Some(color) => self.tinted.id(&TintKey::new(color, depth_fade)),
        }
    }

    /// The material a site of `color` sheds its sparks with, made if no quad has used it yet.
    fn material_of(
        &mut self,
        color: Option<Color>,
        depth_fade: bool,
        look: &FlameLook,
        materials: &mut Assets<EmberMaterial>,
    ) -> Handle<EmberMaterial> {
        match color {
            None => self.material_for(depth_fade).clone(),
            Some(color) => {
                let key = TintKey::new(color, depth_fade);
                self.tinted.handle(key, materials, || {
                    EmberMaterial::new(key.rgb(), look.strength, look.max_height, depth_fade)
                })
            }
        }
    }
}

/// The ember quad of a flame: a child of the [`Flame`], so it inherits the flame's catch and lean.
#[derive(Component)]
pub(crate) struct Ember;

/// The marker that a [`Flame`] already has its ember quad.
#[derive(Component)]
pub(crate) struct Embered;

fn build_ember_assets(
    mut assets: ResMut<EmberAssets>,
    look: Res<FlameLook>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<EmberMaterial>>,
) {
    assets.mesh = meshes.add(Rectangle::new(1.0, 1.0));
    assets.material =
        materials.add(EmberMaterial::new(scene_colour(&look), look.strength, look.max_height, false));
    assets.faded_material =
        materials.add(EmberMaterial::new(scene_colour(&look), look.strength, look.max_height, true));
    assets.built = true;
}

/// The half-extents of the culling box of an ember quad whose flame stands at `scale`, the instance scale `update_flames` writes
/// (`x` the catch, `y` and `z` one more than the lean). The quad is the flame's child at the identity, so the frustum test
/// multiplies the box by that scale, and [`ember_cull_extents`] divides it out first.
fn cull_box(height: f32, scale: Vec3) -> Vec3A {
    Vec3A::from(ember_cull_extents(height, scale.x, [scale.y - 1.0, scale.z - 1.0]))
}

/// Gives the flame of every [`FlameEmbers`] site one ember quad, carrying the site's phase and height and, in the tag's brightness
/// byte, the rate. The quad's culling box is the crate's own ([`ember_cull_extents`]), for the reason the flame's is: the vertex
/// stage draws a quad the unit square's box does not hold, and the scale it inherits is data.
pub(crate) fn dress_embers(
    mut commands: Commands,
    flames: Query<(Entity, &ChildOf, &Transform), UndressedFlame>,
    sites: Query<(&FlameSite, &FlameEmbers)>,
    mut ember_assets: ResMut<EmberAssets>,
    mut materials: ResMut<Assets<EmberMaterial>>,
    look: Res<FlameLook>,
    flame_assets: Res<FlameAssets>,
) {
    if !ember_assets.built || !flame_assets.built() {
        return;
    }
    for (flame, parent, at) in &flames {
        let Ok((site, embers)) = sites.get(parent.parent()) else { continue };
        let packed = pack_flame_tag(site.phase, site.height, embers.share(), flame_assets.max_height());
        let tag = MeshTag::new(packed);
        // The flame's scale as it stands: a flame dressed in mid-catch or in a wind is not at the identity.
        let height = unpack_flame_tag(packed, flame_assets.max_height()).1;
        let bound = Aabb { center: Vec3A::ZERO, half_extents: cull_box(height, at.scale) };
        let mesh = ember_assets.mesh.clone();
        let material = ember_assets.material_of(site.color, site.depth_fade, &look, &mut materials);
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
                    // The unit square's box neither holds the sparks' quad nor survives the scale it inherits.
                    bound,
                    NoAutoAabb,
                ));
            });
        });
    }
}

/// A flame that has no ember quad yet. The extra tongues of a wide fire ([`FlameTongue`](crate::FlameTongue)) have none: the sparks leave
/// the fire once, from the middle flame.
type UndressedFlame = (With<Flame>, Without<Embered>, Without<crate::FlameTongue>);

/// What an ember quad is read through: itself, its flame (its parent), its tag, its culling box and its material.
type EmberQuad = (
    Entity,
    &'static ChildOf,
    &'static mut MeshTag,
    &'static mut Aabb,
    &'static mut MeshMaterial3d<EmberMaterial>,
);

/// The flame and the site an ember quad hangs on, and the height scale their tags are packed against.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct EmberOwners<'w, 's> {
    flames: Query<'w, 's, (&'static ChildOf, &'static Transform), With<Flame>>,
    sites: Query<'w, 's, (&'static FlameSite, Option<&'static FlameEmbers>)>,
    assets: Res<'w, FlameAssets>,
}

impl EmberOwners<'_, '_> {
    /// The site of `flame` and its embers, and the instance scale `flame` stands at, when `flame` is a flame that hangs on a site.
    fn site_of(&self, flame: Entity) -> Option<(&FlameSite, Option<&FlameEmbers>, Vec3)> {
        let (parent, at) = self.flames.get(flame).ok()?;
        let (site, embers) = self.sites.get(parent.parent()).ok()?;
        Some((site, embers, at.scale))
    }

    /// The height scale a flame's tag is packed against ([`FlameLook::max_height`] at `Startup`).
    fn max_height(&self) -> f32 {
        self.assets.max_height()
    }
}

/// Follows each site's [`FlameEmbers`] and [`FlameSite::depth_fade`] with its quad, takes the quad away when the component goes,
/// keeps the quad's culling box in step with the flame's scale (it runs after `update_flames`, so the box is never a frame
/// behind the scale the frustum test multiplies it by), and ticks the shared clock and a changed [`FlameLook`].
pub(crate) fn update_embers(
    mut commands: Commands,
    real: Res<Time<Real>>,
    look: Res<FlameLook>,
    mut ember_assets: ResMut<EmberAssets>,
    mut materials: ResMut<Assets<EmberMaterial>>,
    mut embers: Query<EmberQuad, With<Ember>>,
    owners: EmberOwners,
) {
    if embers.is_empty() {
        return;
    }
    let max_height = owners.max_height();
    for (entity, parent, mut tag, mut bound, mut material) in &mut embers {
        let site = owners.site_of(parent.parent());
        let Some((site, Some(wanted), scale)) = site else {
            // The component is gone (or the flame is): the quad goes, and the flame may be dressed again.
            commands.entity(entity).despawn();
            commands.entity(parent.parent()).remove::<Embered>();
            continue;
        };
        if ember_assets.material_id(site.color, site.depth_fade) != Some(material.0.id()) {
            material.0 = ember_assets.material_of(site.color, site.depth_fade, &look, &mut materials);
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
    if !ember_assets.tinted.is_empty() {
        ember_assets.tinted.sweep(&materials);
    }
    let elapsed = real.elapsed_secs_wrapped();
    let scene = scene_colour(&look);
    let scene_materials = [ember_assets.material.id(), ember_assets.faded_material.id()];
    for (id, is_scene) in scene_materials
        .into_iter()
        .map(|id| (id, true))
        .chain(ember_assets.tinted.ids().map(|id| (id, false)))
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
    use crate::model::{FLICKER_DEPTH, ember_reach, unpack_flame_tag};

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(Assets::<Mesh>::default())
            .insert_resource(Assets::<crate::FlameMaterial>::default())
            .insert_resource(Assets::<EmberMaterial>::default())
            .init_resource::<FlameLook>()
            .init_resource::<FlameAssets>()
            .init_resource::<EmberAssets>()
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_secs_f64(1.0 / 60.0),
            ));
        app.add_systems(Startup, (build_flame_assets, build_ember_assets))
            .add_systems(Update, (dress_flames, update_flames, dress_embers, update_embers).chain());
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

    fn ember_quads(app: &mut App) -> Vec<(Entity, u32)> {
        app.world_mut()
            .query_filtered::<(&ChildOf, &MeshTag), With<Ember>>()
            .iter(app.world())
            .map(|(parent, tag)| (parent.parent(), tag.value))
            .collect()
    }

    /// The material an ember quad is drawn with, copied out.
    fn sparks_of(app: &App, site: Entity) -> EmberMaterial {
        let flame = app.world().get::<Children>(site).and_then(|c| c.first().copied()).expect("a flame");
        let quad =
            app.world().get::<Children>(flame).and_then(|c| c.first().copied()).expect("an ember quad");
        let handle = app.world().get::<MeshMaterial3d<EmberMaterial>>(quad).expect("a material").0.clone();
        app.world().resource::<Assets<EmberMaterial>>().get(&handle).expect("it exists").clone()
    }

    /// A fire sheds sparks of its own colour, and a scene that sets none sheds the shipped orange ones.
    #[test]
    fn a_fire_sheds_sparks_in_its_own_colour_and_follows_it_when_it_changes() {
        use crate::model::fire_colour::{CORE_YELLOW, SPARK_DULL_RED, SPARK_ORANGE};
        let mut app = app();
        let green = Color::from(LinearRgba::rgb(0.2, 1.0, 0.3));
        let tinted = app
            .world_mut()
            .spawn((FlameSite { color: Some(green), ..site(0.5, 0.0) }, FlameEmbers::default()))
            .id();
        let orange = app.world_mut().spawn((site(0.5, 1.0), FlameEmbers::default())).id();
        app.update();
        app.update();
        let spark = |m: &EmberMaterial| {
            [[m.dull.x, m.dull.y, m.dull.z], [m.mild.x, m.mild.y, m.mild.z], [m.hot.x, m.hot.y, m.hot.z]]
        };
        assert_eq!(
            spark(&sparks_of(&app, orange)),
            [SPARK_DULL_RED, SPARK_ORANGE, CORE_YELLOW],
            "the scene's sparks are as shipped"
        );
        let [dull, mild, hot] = spark(&sparks_of(&app, tinted));
        for stop in [dull, mild, hot] {
            assert!(
                stop[1] > stop[0] && stop[1] > stop[2],
                "a green fire's sparks are green at every stage: {stop:?}"
            );
        }
        // A colour the host changes later reaches the sparks too.
        app.world_mut().get_mut::<FlameSite>(tinted).expect("the site").color =
            Some(Color::from(LinearRgba::rgb(0.1, 0.3, 1.0)));
        app.update();
        let [_, mild, _] = spark(&sparks_of(&app, tinted));
        assert!(mild[2] > mild[1] && mild[1] > mild[0], "now blue: {mild:?}");
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

    /// Points on the ember quad, a 5 x 5 grid of its corners and interior, where `src/embers.wesl`'s vertex stage puts them: the
    /// same constants, the same extents (the quad widens downwind by the drift a spark has at the top of its rise), for a camera
    /// at `eye` whose right axis is `camera_right`. `origin` is the flame's foot, `size` the height it is drawn at, `wind` the
    /// instance scale's `y` and `z` less one.
    fn drawn_points(origin: Vec3, size: f32, wind: Vec2, eye: Vec3, camera_right: Vec3) -> Vec<Vec3> {
        const SOURCE_HEIGHT: f32 = 0.35;
        const HALF_WIDTH: f32 = 0.7;
        const RISE_MAX: f32 = 2.6;
        const MARGIN: f32 = 0.35;
        const DRIFT: f32 = 1.5;
        let right = Vec3::new(camera_right.x, 0.0, camera_right.z).normalize();
        let centre = origin + Vec3::Y * (size * SOURCE_HEIGHT);
        let to_eye = eye - centre;
        let sin_e = (to_eye.y / to_eye.length().max(1e-4)).clamp(0.0, 1.0);
        let cos_e = (1.0 - sin_e * sin_e).max(0.0).sqrt();
        let reach = DRIFT * wind.dot(Vec2::new(right.x, right.z)) * RISE_MAX;
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
    fn an_ember_quad_any_part_of_which_is_on_screen_is_never_culled() {
        // The quad the vertex stage draws (`drawn_points`), the box and the scale this crate gives it, and Bevy's own frustum
        // test: a fire whose sparks are culled while any of their quad is inside the frame sheds none at the screen's edge.
        crate::flame::culling_tests::assert_a_quad_on_screen_is_never_culled(
            ember_cull_extents,
            ember_reach,
            drawn_points,
        );
    }

    /// The ember quad's box as the frustum test sees it, with the scale it inherits from its flame: the half-extents in the
    /// quad's own axes, the flame's scale, and the box the test builds from the two.
    fn world_box(app: &mut App) -> Option<(Vec3, Vec3)> {
        let (flame, bound) =
            app.world_mut().query_filtered::<(&ChildOf, &Aabb), With<Ember>>().iter(app.world()).next()?;
        let scale = app.world().get::<Transform>(flame.parent())?.scale;
        Some((scale, Vec3::from(bound.half_extents) * scale))
    }

    #[test]
    fn an_ember_quads_box_keeps_its_size_through_the_catch_and_the_lean_on_every_frame() {
        // The quad inherits the flame's scale, which is the shader's data (its catch and its lean), and the frustum test
        // multiplies the box by it. The box must have been divided by it first, in the same frame the scale was written, or a
        // gusting fire's sparks are culled on screen for the frame the box lags.
        let mut app = app();
        app.insert_resource(crate::FlameWind { velocity: Vec2::new(3.0, -1.5) });
        let height = 0.9;
        app.world_mut().spawn((
            FlameSite { kindle: crate::KINDLE_SECONDS, ..site(height, 0.0) },
            FlameEmbers::default(),
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
        assert!(checked > 30, "only {checked} frames had an ember quad");
        assert!(
            seen_small,
            "the run never had a small, leaning flame, so it did not test the catch and the lean together"
        );
    }

    #[test]
    fn an_ember_quad_added_to_a_flame_mid_catch_has_the_right_box_from_its_first_frame() {
        // `FlameEmbers` can be added to a fire that is already catching in a wind, so the quad is born under a scale that is
        // not the identity; its box must be built for that scale and not the identity's.
        let mut app = app();
        app.insert_resource(crate::FlameWind { velocity: Vec2::new(0.0, 4.0) });
        let height = 0.9;
        let id = app
            .world_mut()
            .spawn((FlameSite { kindle: crate::KINDLE_SECONDS, ..site(height, 0.0) }, Transform::default()))
            .id();
        for _ in 0..12 {
            app.update();
        }
        app.world_mut().entity_mut(id).insert(FlameEmbers::default());
        let mut frames = 0;
        // Until the quad exists no box is seen; the frame it does, it is already right.
        while world_box(&mut app).is_none() {
            app.update();
            frames += 1;
            assert!(frames < 5, "the quad never appeared");
        }
        let (scale, world) = world_box(&mut app).unwrap();
        assert!(scale.x < 0.9 && (scale.z - 1.0).abs() > 0.1, "the flame is mid-catch and leaning: {scale}");
        let size = height * scale.x.min(1.0).powf(2.0 / 3.0);
        let want = ember_reach([scale.y - 1.0, scale.z - 1.0]) * size;
        assert!((world.y - want).abs() < 1e-3 * (1.0 + want), "y: {world} against {want}");
        assert!((world.z - want).abs() < 1e-3 * (1.0 + want), "z: {world} against {want}");
    }

    #[test]
    fn an_ember_quad_carries_a_box_of_its_own_and_is_not_left_to_the_units_quad() {
        let mut app = app();
        app.world_mut().spawn((site(0.5, 0.0), FlameEmbers::default(), Transform::default()));
        app.update();
        app.update();
        let quad = app.world_mut().query_filtered::<Entity, With<Ember>>().single(app.world()).unwrap();
        let entity = app.world().entity(quad);
        assert!(entity.contains::<Aabb>(), "an ember quad has a culling box");
        assert!(entity.contains::<NoAutoAabb>(), "which Bevy is told not to recompute");
        assert!(
            !entity.contains::<bevy::camera::visibility::NoFrustumCulling>(),
            "and which the frustum test uses: an off-screen fire's sparks are not drawn"
        );
    }
}
