# The guide

The detail behind the README. Every page here is a moved section of it — the README carries the pitch,
the quick start and one bullet per feature; the numbers, the traps and the measured costs live here.

Read in this order if you are new:

1. [`fires.md`](fires.md) — the flicker, the kindle and the snuff, the six fire kinds, and `Fire`, which
   writes a whole fire from one component.
2. [`light-pool.md`](light-pool.md) — the budgeted pool of real lights: ranking, hysteresis, the
   cross-fade, the shadow budget and the template that reaches every light.
3. [`flames.md`](flames.md) — the shader flames: a fire's own colour, tongues for a wide fire, sparks,
   smoke and its trail, wind and gusts, and a fire that moves.
4. [`fuel-bed.md`](fuel-bed.md) — the bed of coals: a temperature field, not a constant `emissive`.
5. [`lighting-look.md`](lighting-look.md) — the cinematographer's numbers as plain functions.

Then the reference-shaped pages:

- [`model.md`](model.md) — every curve in the crate as an engine-free `f32` function.
- [`host-api.md`](host-api.md) — what a host spawns, marks and reads.
- [`plays-well-with.md`](plays-well-with.md) — which of the family owns what, so two crates do not both
  own the sun.
- [`research.md`](research.md) — the research note behind every default.

Media recipes (frames, GIF encoding and the traps) are in [`docs/DEMOS.md`](../DEMOS.md); the list of
every file in `docs/media/`, with the command that regenerates it, is
[`docs/media/README.md`](../media/README.md).
