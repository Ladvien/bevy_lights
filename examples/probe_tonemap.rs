//! Throwaway probe: three ways to make a coal bed emit, measured at 85 degrees of elevation.
mod common;
use bevy::asset::RenderAssetUsages;
use bevy::camera::Exposure;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy_lights::*;

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window { resolution: (1280, 720).into(), ..default() }),
            ..default()
        }),
        common::ExampleHarness,
    ));
    app.add_systems(Startup, setup);
    app.run();
}

fn field(seed: u32) -> Image {
    const N: u32 = 64;
    let mut data = Vec::new();
    for y in 0..N {
        for x in 0..N {
            let u = x as f32 / (N - 1) as f32;
            let v = y as f32 / (N - 1) as f32;
            let rgb = char_radiance(char_field(u, v, seed), [0.10, 0.045, 0.02], [0.35, 0.06, 0.008], [9.0, 1.7, 0.25]);
            data.extend_from_slice(&rgb);
            data.push(1.0);
        }
    }
    let bytes = data.iter().flat_map(|f| f.to_le_bytes()).collect::<Vec<u8>>();
    Image::new(
        Extent3d { width: N, height: N, depth_or_array_layers: 1 },
        TextureDimension::D2,
        bytes,
        TextureFormat::Rgba32Float,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    )
}

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>, mut images: ResMut<Assets<Image>>) {
    let tex = images.add(field(7));
    // Camera: 4 m from the first bed at 85 degrees, matching the loop's sweep.
    let (cx, cy, cz) = (7.0, 0.72, 5.0);
    let e: f32 = 85f32.to_radians();
    let u = 1.0 / 2f32.sqrt();
    commands.spawn((
        Camera3d::default(),
        Camera { clear_color: ClearColorConfig::Custom(Color::BLACK), ..default() },
        Exposure { ev100: 6.0 },
        Transform::from_xyz(cx + 9.0 * e.cos() * u, cy + 9.0 * e.sin(), cz + 9.0 * e.cos() * u)
            .looking_at(Vec3::new(cx, cy, cz), Vec3::Y),
    ));
    let bed = meshes.add(Cuboid::new(0.34, 0.05, 0.34));
    let bowl = meshes.add(Cuboid::new(0.8, 0.7, 0.8));
    let stone = materials.add(StandardMaterial {
        base_color: Srgba::rgb(0.10, 0.09, 0.09).into(), perceptual_roughness: 0.9, ..default()
    });
    // Three variants side by side, 2 m apart along x, each in its own bowl with its own light.
    let variants: [(&str, StandardMaterial); 3] = [
        ("A unlit", StandardMaterial {
            unlit: true, base_color: Color::WHITE, base_color_texture: Some(tex.clone()), ..default()
        }),
        ("B lit+emissive dark albedo", StandardMaterial {
            base_color: LinearRgba::rgb(0.012, 0.010, 0.009).into(),
            perceptual_roughness: 0.95,
            emissive: LinearRgba::new(1.0, 1.0, 1.0, 1.0),
            emissive_texture: Some(tex.clone()),
            emissive_exposure_weight: 1.0,
            ..default()
        }),
        ("C lit+emissive, charcoal albedo 0.04", StandardMaterial {
            base_color: Srgba::rgb(0.040, 0.035, 0.030).into(),
            perceptual_roughness: 0.95,
            emissive: LinearRgba::new(1.0, 1.0, 1.0, 1.0),
            emissive_texture: Some(tex.clone()),
            emissive_exposure_weight: 1.0,
            ..default()
        }),
    ];
    for (i, (name, mat)) in variants.into_iter().enumerate() {
        let x = 5.0 + i as f32 * 2.0;
        let m = materials.add(mat);
        commands.spawn((
            Name::new(format!("Probe {name}")),
            Mesh3d(bowl.clone()),
            MeshMaterial3d(stone.clone()),
            NotShadowCaster,
            Transform::from_xyz(x, 0.35, 5.0),
        )).with_children(|b| {
            b.spawn((
                Mesh3d(bed.clone()),
                MeshMaterial3d(m),
                Transform::from_xyz(0.0, 0.35, 0.0),
                NotShadowCaster,
            ));
            let lumens = fire_lumens(1000.0, 4.0, 1.25);
            b.spawn((
                PointLight { color: Color::srgb(1.0, 0.45, 0.15), intensity: lumens, range: 10.0, ..default() },
                Transform::from_xyz(0.0, 0.9, 0.0),
            ));
        });
    }
}
