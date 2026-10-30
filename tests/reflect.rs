//! Everything public is `Reflect`, so a BRP client (`bevy_fairy`, an editor panel) can read it: the
//! drawn `Flame`, which a host never spawns but does want to find, must be readable as a component.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use bevy::prelude::*;
use bevy::reflect::{GetTypeRegistration, TypeRegistry};
use bevy_lights::Flame;

#[test]
fn a_flame_can_be_read_through_reflection() {
    let mut registry = TypeRegistry::default();
    registry.register::<Flame>();
    let reflect_component = registry
        .get_type_data::<ReflectComponent>(Flame::get_type_registration().type_id())
        .expect("Flame registers as a component");

    let mut world = World::new();
    let torch = world.spawn_empty().id();
    let flame = world.spawn(Flame { kindled: 0.25, source: Some(torch) }).id();
    let entity = world.entity(flame);
    let reflected = reflect_component.reflect(entity).expect("the component is on the entity");
    let read = reflected.downcast_ref::<Flame>().expect("it is a Flame");
    assert_eq!(read.kindled, 0.25);
    assert_eq!(read.source, Some(torch));
}
