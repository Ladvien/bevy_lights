//! THROWAWAY (iteration 277): N flame sites on a grid, camera sees a few; prints frame time.
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::window::PresentMode;
use bevy_lights::*;

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: PresentMode::AutoNoVsync,
                    resolution: (1280, 720).into(),
                    ..default()
                }),
                ..default()
            }),
            FrameTimeDiagnosticsPlugin::default(),
            FlamePlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, report)
        .run();
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let mut target = Image::new_fill(Extent3d { width: 1280, height: 720, depth_or_array_layers: 1 }, TextureDimension::D2, &[0, 0, 0, 255], TextureFormat::Rgba8UnormSrgb, default());
    target.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_SRC;
    let target = images.add(target);
    let n = env_usize("ZZ_FLAMES", 1000);
    let side = (n as f32).sqrt().ceil() as usize;
    for i in 0..n {
        let x = (i % side) as f32 * 3.0;
        let z = (i / side) as f32 * 3.0;
        commands.spawn((FireKind::Torch.site(), Transform::from_xyz(x, 0.0, z)));
    }
    commands.spawn((Camera3d::default(), RenderTarget::Image(target.into()), Transform::from_xyz(4.0, 1.5, 12.0).looking_at(Vec3::new(4.0, 1.0, 0.0), Vec3::Y)));
}

fn report(diag: Res<DiagnosticsStore>, mut frame: Local<u32>, mut sum: Local<f64>, mut count: Local<u32>, mut exit: MessageWriter<AppExit>) {
    *frame += 1;
    if *frame > 150 {
        if let Some(ft) = diag.get(&FrameTimeDiagnosticsPlugin::FRAME_TIME).and_then(|d| d.value()) {
            *sum += ft;
            *count += 1;
        }
    }
    if *frame >= 450 {
        println!("zz_field flames={} frames={} mean_ms={:.3}", env_usize("ZZ_FLAMES", 1000), *count, *sum / (*count).max(1) as f64);
        exit.write(AppExit::Success);
    }
}
