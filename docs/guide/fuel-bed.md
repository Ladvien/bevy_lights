# The fuel bed

A bed of coals is a *surface with a temperature field on it*, and a constant `emissive` is the one thing
a real bed never is: it clips to a white slab the moment the camera looks down at it, and hides the flame
standing in it. `char_field` is the field — a heap of overlapping coal shards, bright lumps with dark ash
between them, seeded so a capture is reproducible — and `char_radiance` / `coal_radiance` map a sample to
a colour through char, ember and fuel. `coal_radiance` writes those three stops as **temperatures**, so a
bed's brightness and its hue come from one model and cannot drift apart at a different exposure.

**`FuelBed`: a bed that lives with its fire.** Add `FuelBed { diameter, seed, cooling }` to a `FlameSite`
(`FireKind::bed()` gives a campfire's, a bonfire's and a brazier's, and `None` for the rest) and the site gains one
flat disc at its foot with the field laid on it at the coals' own size (a wider bed has more coals, not larger ones):
dark ash with orange and yellow lumps in it, hottest under the flame and ragged at the rim. The coals follow
the site's fire (`FlameSite::fire`): they **breathe with its flicker**, **warm as the fire catches** (`Kindle`) and, when
it is put out (`Snuff`), **go on glowing and cool** with time constant `cooling` seconds (60 by default; the
glow is gone in a minute and a half) instead of vanishing with the flame, the way a campfire that is out is a ring of dull red
going to black ([`docs/research/fuel-bed-heat.md`](../research/fuel-bed-heat.md)). A heat moves every temperature of the ramp by the same number of
kelvin, so the bed goes from orange through a deepening red to nothing, not through grey: `bed_radiance(field, heat)`
is that ramp and `bed_heat_step` the dynamics, as plain functions. One shared material draws every bed (a bed's own heat and
offset ride its `MeshTag`), so fifty campfires are fifty draws, and a site without the component pays nothing.

![A campfire's bed of coals from 2.4 m straight down, in four panels: burning, 0.75 s after the fire is put out with the flame nearly gone and the coals still orange, 2.0 s after with a dim ground and the coals glowing red, and 4.5 s after with only a dull red left](../media/fuel-bed.png)

*`cargo run --release --example campfire` with `LIGHTS_EYE=-2.6,2.4,0.5 LIGHTS_LOOK=-2.6,0,0 LIGHTS_SNUFF=1.0 LIGHTS_COOLING=6` (the campfire's `FireKind::bed()`; an 840 × 728 PNG of four 420 × 330 crops of a 1280 × 720 run at frames 30, 105, 180 and 330, the captions burned in): the fire is put out at 1.0 s, its light and flame are gone in 1.2 s and the coals go on glowing, cooling here with a 6 s time constant instead of the default 60 s so a capture can show it. The logs are lit by the fire's light and stand dark against the coals once it is out.*

`examples/courtyard.rs` still lays its own bed on its brazier bowls (an `Rgba32Float` image of the field as a box's `emissive_texture`, its numbers chosen by measuring the stops in display DN, because a ramp whose middle stop is past the display's white point cannot show the structure underneath it): a bowl is a prop with a soot lining the example paints by hand, and its bed does not follow the fire.
