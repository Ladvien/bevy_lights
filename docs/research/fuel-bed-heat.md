# A bed of coals that lives with its fire (`FuelBed`)

[`fuel-bed.md`](fuel-bed.md) fixed what a bed of coals *is*: a surface with a temperature field on it, not a slab of constant `emissive`. It left the bed as **an example's code**: `examples/courtyard.rs` bakes `char_field` through `coal_radiance` into an `Rgba32Float` image at startup and lays it on a box, and `examples/campfire.rs` (the crate's own wide fires, iteration 283) still has the constant-emissive cylinder the note calls the defect: a flat peach plate under the logs in every capture of `FireKind::Campfire` and `FireKind::Bonfire`. A bed built that way is also **dead**: it is one frozen texture, the same brightness whether the fire above it flares, is being lit, or has been put out. Every fire this crate draws breathes, catches and goes out (`flicker`, `Kindle`, `Snuff`); the one part of a fire that is geometry does none of it. A host that wants a bed must copy ~120 lines of example code and then drive its material from the fire by hand.

`FuelBed` is the drop-in: add it to a `FlameSite` and the site gets a bed of coals under its flame that **breathes with the fire's flicker, warms up as the fire catches, and, when the fire is put out, goes on glowing and cools over a minute rather than vanishing with the flame**.

## Sources

| source | what it is | use |
|---|---|---|
| `10.32657_10220_48312` — *Strategies of colour and lighting design in the cinematography of Christopher Doyle* (corpus) | craft | a practical's glow must have a source in the frame: the motivated light is read off the thing that emits it |
| `s2011-advances-pdf-slides` — SIGGRAPH 2011 *Advances*, Fire/Flamethrowers/Thrusters (corpus) | production | the hot thing glows as a light and the glow cools: "hot objects splat their shape into the light volume textures to create local 'glow' lights" |
| `si/sig2024_Scintilla_Simulating_Combustible_Vegetation_for_Wildfires` — Scintilla, SIGGRAPH 2024, Sec. 5.7 (corpus) | technical | an ember's temperature falls by "Stefan-Boltzmann and Newton cooling laws": the cooling is exponential in the surface's excess temperature, so a bed's glow dies away rather than switching off |
| `10.1145_3130800.3130814` — Pirk et al. 2017, *Interactive wood combustion for botanical tree models* (corpus; already the basis of [`fuel-bed.md`](fuel-bed.md)) | technical | char is a layer that insulates; pyrolysis runs between two stated temperatures, so the bed is a field of states and the field moves with temperature |
| `PBR3_12_Light_Sources` — *Physically Based Rendering* 3rd ed. ch. 12 (corpus) | technical | "Blackbody emitters are perfectly diffuse; they emit radiance equally in all directions": a bed's radiance is a function of its temperature alone, not of the view |
| Rein 2009, *Smouldering Combustion Phenomena in Science and Technology* (IRECHE 1; found by `paper_search`, no DOI, `hdl.handle.net/1842/2678`) | technical, **not in the corpus** | glowing (smouldering) combustion of a char bed outlasts the flame by tens of minutes; the length of a game's cool-down is therefore a design choice, not a physical constant (see `Pending index`) |

## Craft principle

Doyle (corpus): "Doyle uses diegetic colour that originates within the narrative to provide motivation for the incandescent spot". The orange the audience sees on the walls has to be traceable to something they can see burning. [`fuel-bed.md`](fuel-bed.md)'s measurement says the same in numbers: the source must be the brightest thing on its own prop. A fire whose coals are one still picture is a source that does not respond to what it lights: when the flame flares the ground brightens and the coals under it do not. When the fire is put out the audience's last read of the scene is the coals: a campfire that is out is a ring of dull red, and that is the strongest "it went out, and it was alive a moment ago" cue a bed can give. A bed that goes black with the flame says "a prop was switched off".

## How shipping games do it

SIGGRAPH 2011 *Advances* (corpus, the fire talk): "hot objects splat their shape into the light volume textures to create local 'glow' lights", and the talk's own admission that "was quite difficult to hide the 'particle' nature of the system". The production answer is that a fire's hot surface is a source of its own that carries the fire's state (its heat), and that the *fuel* is not a texture painted once: its glow is a driven quantity. The crate's version of that is one shader driven by one number, the bed's heat. (Practice in shipped campfires, an emissive map whose strength follows the fire and embers that outlast it, is general knowledge and not cited here: no corpus document describes it.)

## Model adopted

A bed has **heat** `h`, `0..=1`: `1` is a fire burning, `0` is cold. The bed is drawn from two things:

- **the field**: `char_field`'s heap of overlapping lumps, as in the courtyard, made *tileable* (`char_tile`, a periodic variant: the heap's cell hashes wrap at an integer period) so one 256 x 256 texture covers a bed of any width at the coals' own physical size (`BED_TILE` = 0.36 m, six coarse lumps). The bed cools from its middle to its rim, as the plume is axisymmetric ([`fuel-bed.md`](fuel-bed.md), Xia and Zhang), applied in the shader by the disc's radius, not by the texture;
- **the ramp**: `coal_radiance`'s three stops (char, ember, fuel) are three **temperatures**, 950 K, 1020 K and 1300 K. The bed's heat moves every stop's temperature down by the same number of kelvin: `T(h) = T - 450 K (1 - h)`. At heat `0` the hottest coal is at 850 K, under the Draper point (about 798 K plus the dark margin a display needs), ten thousand times under its burning radiance, and every stop's radiance follows the blackbody law down, hue and all: a cooling bed goes from orange through a deepening red to nothing, which is how coals die, instead of a bright bed dimming through grey. This is `bed_radiance(field, heat)`, on `blackbody_radiance`, a new unclamped form of `blackbody_luminance` that goes below the 1000 K floor those two share (a bed cooling is the one thing that needs the range under it). At heat `1` the ember and fuel stops agree with `coal_radiance`'s to a few per cent (a test pins it); the charred stop differs on purpose, because `coal_radiance` clamps it to 1000 K and floors it at a reflected grey where this bed's ash is drawn as its own dark colour. `coal_radiance` and the courtyard's bed are untouched.

The ramp's radiances are in cd/m², which a host's exposure turns into pixels; the crate's other layers (flame, sparks, smoke) are display-referred ("an additive billboard tinted by the scene's fire colour reads as fire under any exposure"), so the bed is too: the ramp table is divided by the fuel stop's brightest channel at heat 1 and multiplied by `BED_PEAK`, and the material multiplies by `FlameLook::strength`, the scene's own fire strength. **The bed is an opaque cut-out, not an additive layer** (the first build was additive; see the rounds below): the fire has lit the ground round the bed orange, and an additive glow on that has no dark between the lumps to read them against. The disc carries its own dark ash and char (`ASH`, lit by the flame above it, so it goes dark with the fire), `discard`s past a ragged edge (a bright lump holds on further out than the ash between lumps), and lies a centimetre and a half above the site's foot. It is drawn before every transparent layer, so the additive flames burn over it, and the logs and stones above it hide it through the depth test.

**The heat's clock.** The site's fire is its linked light (`FlameSite::fire`). Per frame, on virtual time:

- *burning* (no `Snuff` on the light): `h` rises towards `1` with time constant `BED_WARM_SECONDS` (a fire being lit warms its bed in seconds, behind the flame's own 0.6 s catch); a bed dressed under a fire that is already burning starts at `1`;
- *put out* (a `Snuff` on the light): `h` falls with `exp(-t / cooling)`, `cooling` = `FuelBed::cooling` seconds. The flame is gone in `SNUFF_SECONDS` (1.2 s); the coals keep their glow and cool (Newton: the excess temperature decays exponentially, Scintilla Sec. 5.7). Removing the `Snuff` (a relight) warms the bed again from where it had cooled to;
- *breathing*: the heat the bed is **drawn** at is `h` less a share of the fire's own flicker dip, `h (1 - BED_BREATH (1 - swing))` with `swing` = `flicker_swing` of the same fire (the flame's own brightness byte, the light's own factor). A fire whose flame flares has coals that brighten with it: the air the puffing flame draws in feeds the bed, and a lit scene whose flame and ground both pulse while the bed between them is still reads as three layers, not as one fire. The share is small because the glow is steep in temperature: a 6 % dip in heat is about 8 K at the hottest stop and already a 25 % swing in brightness.

The heat rides each bed's `MeshTag`, so **one shared material draws every bed** (the field texture and the ramp table are the material's; a bed's heat and its offset into the tile are its own).

## Parameters

| name | value | source |
|---|---|---|
| `FuelBed::diameter` | the bed's width in metres; a kind's: campfire 0.7, bonfire 1.9, brazier 0.30 | design default: the flame's tongues stand on the bed (`FlameTongues::spread` 0.55 and 1.6 m) and the bed reaches a little past their feet, and stays narrower than the kind's fuel (`FireKind::diameter`: 0.8, 3.0, 0.34) |
| `FuelBed::seed` | `0`: which window of the shared coal field this bed shows | design default |
| `FuelBed::cooling` | 60 s, the time constant of the coals' cooling after the fire is put out; the hottest coal is at a thousandth of its burning brightness (`BED_VISIBLE_SHARE`) after about 66 s, which is when the disc is hidden | design default: Newton's law fixes the *shape* (exponential), the length is a game's choice (Rein: a real bed smoulders for tens of minutes, which no game scene waits for); a host that wants the real length sets it |
| `BED_COOL_KELVIN` | 450 K | design default: at heat `0` the hottest coal is at 850 K, dark on any display (350 K left it at a thousandth of its brightness, a faint red that never went out) |
| `BED_WARM_SECONDS` | 4 s | design default: a bed warms behind the flame's catch |
| `BED_BREATH` | 0.06 | design default, above: the swing is a quarter of the brightness at a 6 % dip |
| `BED_PEAK` | 3.5, the hottest coal's brightest channel at heat 1, in linear light, before `FlameLook::strength` | design default, set by capture (rounds below): 10 and 6 read as one orange mass, 3.5 as deep red lumps with orange cores |
| `ASH` | `(0.030, 0.017, 0.011)` at a burning bed, `0.15` of it at heat `0` | design default: dark brown, a little over a fifth of the lit ground round it |
| `BED_TILE` | 0.36 m, six coarse lumps | [`fuel-bed.md`](fuel-bed.md): "the bed is 0.34 m and a coal is ~5 cm, which is six across" |

## Validation targets

- Heat `1` is `coal_radiance`'s ember and fuel stops to a few per cent; heat is monotone: no channel of any sample brightens as heat falls; a cold bed is under half a per cent of the burning peak in every entry (`tests` in `model::bed`).
- A cooling bed's hue goes redder as it dims (the red channel's share never falls with heat), which a grey dimming would not.
- The tile is periodic: `char_tile(u, v) == char_tile(u + 1, v) == char_tile(u, v + 1)` and it has the courtyard field's shape (both ends of the range, a mean in `0.35..0.65`).
- A fire put out: the bed's heat is still above 97 % of a burning bed's one second later (the strongest "it was alive" cue), under 3 % after four time constants, and a bed under a fire that is out for good is hidden once it is dark; a bed dressed under a fire that has not caught is cold and one under a fire that has burned is hot.
- A relight warms the bed back; a removed `FuelBed` takes its disc away and can come back; a changed `diameter` resizes it; a `NaN` or non-positive diameter draws no bed and never panics (`tests` in `bed`).
- On screen: the coals are patches of orange on dark ash, not a plate; they breathe with the flame (a few per cent of display value); the logs and tongues occlude them correctly; the rim is ragged, not a hard circle.
- Cost: one draw per bed, a four-vertex quad, one texture sample and four table reads a fragment; on the CPU `update_beds` is about 0.06 µs a bed a frame (5 000 beds, 300 frames, a headless app) plus a `MeshTag` write on a frame the quantised heat changes. On the GPU two beds cost 10 to 25 µs of the opaque pass (`LIGHTS_GPU_TIMING=1`, interleaved against the build before it).

## What great looks like

A campfire at night seen from 2.5 m: the flame stands in a bed of dark coals with orange and yellow lumps in it, the brightest under the flame's axis and dying out to ash at the rim; when the flame flares the nearest lumps brighten a little with it. Put the fire out and the flame shrinks to nothing in a second while the lumps stay: a ring of dull orange going slowly to red, the smoke lifting off above them (`flame-smoke-tail.md`), until the bed is a dark patch with two or three red points left. Light it again and the lumps warm back to orange as the flame catches.

## Built, and how it was judged

`examples/campfire.rs` (a campfire and a bonfire, each `FireKind::bed()`), 1280 x 720, frame 240, from a viewer's eye, from 2.4 m straight down, at 45°, from 1.2 m, at a graze and from 11 m; a snuff run (`LIGHTS_SNUFF=1.0 LIGHTS_COOLING=6`, 14 frames 1.25 s apart) and 61 consecutive frames of a burning bed. Each round was judged by looking.

- **r0, the build before**: the example's constant-emissive cylinder, a pale peach plate under every campfire and bonfire; nothing that breathes, cools or is lit.
- **r1, additive disc, peak 10**: the bed read as a cloud of orange smear over the ground, with no dark to read the lumps against: the fire's own pool lights the ground to nearly the glow's brightness. NOT YET.
- **r2, opaque cut-out with ash, peak 10**: dark char between the lumps and a ragged rim, but every lump a saturated orange and the whole bed one mass. NOT YET.
- **r3, peak 3.5** (and 6 as a comparison): deep red lumps with orange cores and dark seams (6 is paler and flatter). From 2.4 m, at 45° and from 1.2 m it reads as a bed of coals; at a graze and from the viewer's eye the logs and stones cover most of it, as they would.
- **The cool-down** (`snuff_sheet`: burning, 2 s, 4.5 s and 8.4 s after the douse with a 6 s time constant): the flame and the light are gone in 1.2 s and the coals stay, orange then red then nothing, with the logs standing dark against them: the strongest frame of the set.
- **The breath** (61 frames, a bed region and the flame's core region, grey level): the bed's region ranges 110 to 128 (standard deviation 5, about 7 % of its level) and rises with the flame's own swing over the same frames, 216 to 240 (the flame's core is near white).

## Known limits

- The rim's cut-out is a hard `discard`: its stairs are visible at 4x zoom (the lumps' own edge is soft). `AlphaMode::AlphaToCoverage` would smooth it under MSAA and falls back to the same cut-out without it; not done, since the edge is ragged by design and a pixel step in it reads as a chipped coal.
- The courtyard's brazier beds are still the example's own (a box with an emissive texture): they do not follow their fire, and migrating them to `FuelBed` changes the hero GIF.

## Deviations

- The crate's bed is **display-referred** (a share of the display's white), where [`fuel-bed.md`](fuel-bed.md)'s example bed is physical cd/m² times a hand-set `BED_GAIN` 0.2 at one exposure: the crate's other layers are display-referred, and the example's number is the same hand-tuned share in another unit. The courtyard's bed is unchanged.
- The tile's period is an integer 6 coarse and 10 fine lumps (`char_field`'s are 5.6 and 9.4 over the bed): a periodic heap needs integer cell counts, so the tile is a new function, `char_tile`, and `char_field` is untouched.
- The bed's ramp reaches below the 1000 K floor that `blackbody_colour` and `blackbody_luminance` share, by a new function, `blackbody_radiance` (down to 500 K); the two older functions are unchanged.
- The disc is flat: a heap of coals has relief the glow cannot show from a grazing angle. The logs, the tongues and the stones round the bed cover the angles where it would.
- Cooling is exponential in the heat, not a heat equation: one number per bed, not a field. The lumps cool together.

## Pending index

- `Smouldering Combustion Phenomena in Science and Technology` (Rein 2009, `hdl.handle.net/1842/2678`) has no DOI, so `paper_download` cannot take it; the length of a real bed's glow after the flame is out should be read from it and the `cooling` default compared.
