//! A component that arrives with settings that cannot work is reported once, in the log, by the plugin
//! that owns it.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::io::Write;
use std::sync::{Arc, Mutex};

use bevy::log::tracing_subscriber::{self, fmt::MakeWriter};
use bevy::prelude::*;
use bevy_lights::*;

#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);

impl Write for Captured {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for Captured {
    type Writer = Captured;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

/// The log lines an app's first two frames write after `spawn` ran on it.
fn log_of(spawn: impl FnOnce(&mut World)) -> String {
    let log = Captured::default();
    let subscriber = tracing_subscriber::fmt().with_writer(log.clone()).with_ansi(false).finish();
    let _guard = bevy::log::tracing::subscriber::set_default(subscriber);
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, FlickerPlugin));
    spawn(app.world_mut());
    app.update();
    app.update();
    String::from_utf8(log.0.lock().unwrap().clone()).unwrap()
}

#[test]
fn the_first_bad_flicker_is_named_once_however_many_follow() {
    let log = log_of(|world| {
        for _ in 0..50 {
            world.spawn(Flicker { phase: f32::NAN, base: -1.0, ..default() });
        }
    });
    assert_eq!(log.matches("Flicker").count(), 1, "{log}");
    assert!(log.contains("phase") && log.contains("base"), "{log}");
    assert!(log.contains("WARN"), "{log}");
}

#[test]
fn a_sane_fire_says_nothing_and_each_kind_has_its_own_warning() {
    let quiet = log_of(|world| {
        world.spawn(Flicker::at_mean(800.0, FLICKER_DEPTH));
        world.spawn(Snuff::default());
    });
    assert!(quiet.is_empty(), "{quiet}");

    let both = log_of(|world| {
        world.spawn(Flicker { phase: f32::NAN, ..default() });
        world.spawn(Snuff { elapsed: -1.0 });
    });
    assert!(both.contains("Flicker") && both.contains("Snuff"), "{both}");
}

/// The log lines of an app with `LightPoolPlugin` after `edit` ran on the pool before each of `frames`.
fn pool_log(frames: usize, mut edit: impl FnMut(usize, &mut LightPool)) -> String {
    #[derive(Component)]
    struct Focus;
    let log = Captured::default();
    let subscriber = tracing_subscriber::fmt().with_writer(log.clone()).with_ansi(false).finish();
    let _guard = bevy::log::tracing::subscriber::set_default(subscriber);
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, FlickerPlugin, LightPoolPlugin::<Focus>::new()));
    // The log guard is thread-local: run the system that warns on this thread.
    app.edit_schedule(Last, |schedule| {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });
    for frame in 0..frames {
        edit(frame, &mut app.world_mut().resource_mut::<LightPool>());
        app.update();
    }
    String::from_utf8(log.0.lock().unwrap().clone()).unwrap()
}

#[test]
fn a_pool_written_bad_every_frame_is_one_line_and_a_new_mistake_after_a_fix_is_another() {
    let log = pool_log(8, |frame, pool| match frame {
        0..=3 => pool.reach = f32::NAN,
        4 => pool.reach = 60.0,
        _ => pool.gain = -1.0,
    });
    assert_eq!(log.matches("LightPool").count(), 2, "{log}");
    assert!(log.contains("`reach`") && log.contains("`gain`"), "{log}");
}

#[test]
fn an_untouched_pool_and_a_sane_edit_say_nothing() {
    let log = pool_log(4, |frame, pool| pool.size = 4 + frame);
    assert!(log.is_empty(), "{log}");
}
