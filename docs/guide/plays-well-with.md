# Plays well with

This crate owns **practical lights** and nothing else.

| what | whose |
|---|---|
| Sun, moon, sky, exposure, weather, the lightning flash | [`bevy_elements`](https://github.com/bevy-coven/bevy_elements) (this crate never meters exposure and never adds a `DirectionalLight`) |
| A gameplay illuminance grid (how lit a tile is, for AI or stealth); feed it a fire with `ground_ramp`, a linear ramp fitted to the two lux circles the grid is read at | [`bevy_light_grid`](https://github.com/bevy-coven/bevy_light_grid) |
| Trees, forest-fire spread | [`bevy_flora`](https://github.com/bevy-coven/bevy_flora) |
| Who lights a lamp, and when | [`bevy_society`](https://github.com/bevy-coven/bevy_society) or the game |
| Flicker, kindle, the pool of real lights, the shader flames, the placement helpers | **this crate** |

A pool light is a light in mid-air with no source of its own: a host styles it (recolour it for the
palette) and usually tags it so its flame is not drawn there — the flame belongs on the torch.
