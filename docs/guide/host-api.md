# What a host touches

The components a game spawns or marks are `Reflect` and registered by the plugins — `Fire`, `Flicker`, `Kindle`,
`PoolCandidate`, `FlameSite` — so a BRP front end can read and drive them, and so are the resources a
host sets, `LightPool` (size, reach, gain, rank, hysteresis, fade, reserve, shadow cap, the slot light's template)
`FlameLook` (colour, strength, the set-once `max_height`), `FlameWind` / `FlameGust` (the wind the flames lean in, and a gusting one the plugin writes into it) and `FlameMotion` (how much of a moving fire's own velocity counts as wind). The crate spawns `PoolLight` (registered
too: recolour it for a palette; name its lights what you like) and `Flame`, a plain component a host
finds or counts and never spawns. `Kindled { entity }` is the message a host hears when a fire catches.

## Saving a scene

The `serde` feature derives `serde` on the components and resources a host saves — `Flicker`, `Kindle`,
`Snuff`, `PoolCandidate`, `PoolLight`, `FlameSite`, `FlameLook`, `Rank`, `LightPool` — and turns on Bevy's
own `serialize` for the colours they carry. `PointLight` has no serde derive, so `LightPool` saves its
slot template as the fields the pool copies onto its lights — all of them but `intensity` — and every
field is optional on the way in.

`Fire` is `serde` too, so a placed fire is one line of a scene file.

## Built on

| crate | why |
|---|---|
| [`bevy`](https://github.com/bevyengine/bevy) 0.20 | the engine: `PointLight`, `SpotLight`, `Material`, `MeshTag` |
| [`serde`](https://github.com/serde-rs/serde) | optional derives on every public component and resource (`serde` feature) |
