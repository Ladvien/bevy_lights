//! THROWAWAY (iteration 277): N flame sites on a grid, camera sees a few; times app.update() headless.
use std::time::Instant;

use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::window::ExitCondition;
use bevy::winit::WinitPlugin;
use bevy_lights::*;

#[derive(Resource)]
struct Idle(bool);

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin { primary_window: None, exit_condition: ExitCondition::DontExit, ..default() })
            .disable::<WinitPlugin>(),
        FlamePlugin,
    ))
    .insert_resource(Idle(false))
    .configure_sets(Update, LightsSystems::Flame.run_if(|idle: Res<Idle>| !idle.0))
    .add_systems(Startup, setup);
    app.finish();
    app.cleanup();
    let mut samples = Vec::new();
    for i in 0..env_usize("ZZ_FRAMES", 400) {
        let start = Instant::now();
        if i == 5 {
            match std::env::var("ZZ_MODE").as_deref() {
                Ok("idle") => app.world_mut().resource_mut::<Idle>().0 = true,
                Ok("hidden") => {
                    let mut q = app.world_mut().query_filtered::<&mut Visibility, With<Flame>>();
                    for mut v in q.iter_mut(app.world_mut()) {
                        *v = Visibility::Hidden;
                    }
                }
                Ok("hidden_idle") => {
                    app.world_mut().resource_mut::<Idle>().0 = true;
                    let mut q = app.world_mut().query_filtered::<&mut Visibility, With<Flame>>();
                    for mut v in q.iter_mut(app.world_mut()) {
                        *v = Visibility::Hidden;
                    }
                }
                _ => {}
            }
        }
        app.update();
        if i >= 100 {
            samples.push(start.elapsed().as_secs_f64() * 1000.0);
        }
    }
    samples.sort_by(|a, b| a.total_cmp(b));
    let mean = samples.iter().sum::<f64>() / samples.len() as f64;
    println!(
        "zz_field flames={} frames={} mean_ms={:.3} p50_ms={:.3} min_ms={:.3}",
        env_usize("ZZ_FLAMES", 1000),
        samples.len(),
        mean,
        samples[samples.len() / 2],
        samples[0]
    );
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let mut target = Image::new_fill(
        Extent3d { width: 1280, height: 720, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        default(),
    );
    target.texture_descriptor.usage |=
        TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_SRC;
    let target = images.add(target);
    let n = env_usize("ZZ_FLAMES", 1000);
    let side = (n as f32).sqrt().ceil() as usize;
    for i in 0..n {
        let x = (i % side) as f32 * 3.0;
        let z = (i / side) as f32 * 3.0;
        commands.spawn((FireKind::Torch.site(), Transform::from_xyz(x, 0.0, z)));
    }
    commands.spawn((
        Camera3d::default(),
        RenderTarget::Image(target.into()),
        Transform::from_xyz(4.0, 1.5, 12.0).looking_at(Vec3::new(4.0, 1.0, 0.0), Vec3::Y),
    ));
}
