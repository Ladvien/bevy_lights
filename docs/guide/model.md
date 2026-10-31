# Engine-free maths

Every curve this crate runs is a plain `f32` function with no `bevy` in it, and `tests/leaf.rs` keeps them
that way: a host can reuse the same numbers in its own systems, and the crate's own light, flame and pool
cannot drift from them. The functions live in `bevy_lights::model::<module>` and the common ones are
re-exported flat from `bevy_lights`.

| module | what it holds |
|---|---|
| `model::fire` | `flicker_factor`, `flicker_factor_of`, `flicker_rate` and `PUFFING_CONSTANT`; `flicker_swing` (the sized-or-un-sized choice, and the depth the light and the flame both read) with `flicker_depth`, `FLICKER_DEPTH`, `MAX_FLICKER_DEPTH`, `CALM_FLICKER_DEPTH`, `WHIPPING_FLICKER_DEPTH` and `flicker_mean`; `kindle_gain` and `snuff_gain` with `KINDLE_SECONDS` / `SNUFF_SECONDS` / `SMOKE_TAIL_SECONDS`; `flame_tilt` with `FLAME_TILT_LIMIT`; `fire_phase`; `DEFAULT_FIRE_DIAMETER` |
| `model::reach` | `raised_light` and `RANGE_MARGIN` (how far a raised light reaches, and how bright it must be); `flame_light_offset` (where a fire's light belongs inside its flame, so a flame and the light it motivates cannot drift apart); `lean_light_shift` (where that light goes when the flame leans) |
| `model::motion` | `relative_air` and `smooth_velocity` — the air a moving fire meets, and the velocity it feels — with `MOTION_RESPONSE`, `MIN_TRACKED_SPEED` and `MAX_TRACKED_SPEED` |
| `model::wind` | `gust_velocity`, `gust_unit`, `GUST_INTENSITY` / `GUST_TIMESCALE` / `GUST_TERMS`: the gusting wind as a pure function of the clock and a seed |
| `model::pool` | `hand_off` and `preferred` (the overlap rule, on distances and on any "smaller is better" quantity) with `fade_factor` / `fade_out_factor`, `HYSTERESIS` and `FADE_SECONDS`; `shadow_set`, the shadow budget's rule; and the shared pool's `focus_quota`, `quota_cutoff` and `shared_value` |
| `model::tag` | `pack_flame_tag` / `unpack_flame_tag` with `set_flame_brightness`; `flame_cull_extents` with `FLAME_REACH`, and `ember_cull_extents` / `ember_reach` — the boxes a frustum test needs for a billboard the vertex stage reshapes |
| `model::fire_colour` | `retint` (the shipped orange ramp moved to a fire's hue), `flame_mid` and `FIRE_KELVIN` |
| `model::tongue` | `flame_tongue`, `Tongue`, `MAX_TONGUES` and `MERGE_RATIO`: where a wide fire's tongues stand, how tall each is and how it leans |
| `model::bed` | `bed_radiance`, `bed_ramp`, `bed_heat_step`, `bed_drawn_heat`, `bed_visible_heat`, `bed_offset`, `pack_bed_tag` / `unpack_bed_tag` with the `BED_*` constants and `DEFAULT_BED_COOLING` |
| `model::char` | `char_field`, `char_tile`, `char_radiance`, `coal_radiance`, `TILE_LUMPS` and `EMBER_AT`: the coal field and the ramp from char through ember to fuel |
| `model::cinematography` | `key_to_fill` and `stops` with `LOW_KEY` / `MOON_KEY` / `HIGH_KEY`; `fill_for`; `fire_lumens`, `fire_lux` and `fire_reach`; `ground_lux`, `ground_reach` and `ground_ramp` — the illuminance on flat ground, the circle it reaches and the linear ramp a light grid is fed; `blackbody_colour`, `blackbody_luminance`, `blackbody_radiance` and `white_balanced` |
| `model::problem` | `Problem` and `ProblemKind`: one unusable field named, with why. Every settings type's `problems()` returns these |

`Problem` is what an editor shows beside a field: the crate never refuses a value (a negative `base` or a
`NaN` phase still builds and runs), so a host that reads settings from a file or a panel calls
`problems()` and shows the list before the value reaches a light. See
[what a host touches](host-api.md) for what the crate itself warns about.

[The lighting look](lighting-look.md) covers the cinematography page in prose.
