# A wide fire burns as several tongues (`FlameTongues`)

The flame is one teardrop whose width is a fixed 0.4 of its height (`FLAME_HALF_WIDTH` in `src/flame.wesl`). That is a candle's and a torch's shape, and it is right for a bed a third as wide as the flame is tall. `FireKind::Campfire` is a 0.8 m bed under a 0.7 m flame and `FireKind::Bonfire` a 3 m bed under a 2 m one, and both drew one slender tongue 0.28 m and 0.8 m wide standing on the middle of a bed two to four times as wide as it: a very large candle. The campfire is the most common fire a game has, and the first one a developer places. Iteration 198 judged the kinds "GREAT for the presets as data, NOT a claim about how a scaled prop looks"; this is the look.

## Sources

| source | what it is | use |
|---|---|---|
| `10.13140_marples.nav.lighting.thesis` — Marples 2017, PhD thesis (corpus; quoted in [`carried-fire.md`](carried-fire.md), [`flame-smoke.md`](flame-smoke.md)) | craft | a flame is the thing the eye is sent to, so its shape is looked at, not glanced past |
| `10.32657_10220_48312` — *Strategies of colour and lighting design in the cinematography of Christopher Doyle* (corpus) | craft | a single practical, hard and small, sets the whole scene's ambience: one fire carries the frame, so it has to read as a fire of its size |
| GPU Gems 1, ch. 6, Nguyen, *Fire in the "Vulcan" Demo* (corpus, the book's markdown) | production | a fire is built of several flames that differ in size, position and phase, never one repeated: "it worked even better with two or more" |
| `s2011-advances-pdf-slides` — SIGGRAPH 2011 *Advances*, the fire talk (corpus; quoted in [`fire-colour.md`](fire-colour.md)) | production | each fire owns its emission; a larger fire is more of the same body, not a bigger single one |
| `10.3210_fst.26.1` — Delichatsios 2007, *A Correlation for the Flame Height in "Group" Fires* (Fire Sci. Technol. 26, 1; home-still catalog, DOI 10.3210/fst.26.1) | technical | when separate flames merge: the height at which two flames a distance `S` apart become one is `Z_m = 3 S` (eq. 6b, 7b), so a side tongue is a flame below that height |
| Heskestad, *Flame heights of fuel arrays with combustion in depth*, as restated in PMC6463528 (cited in [`fire-kinds.md`](fire-kinds.md)) | technical | the order of a flame's height against its bed; the kinds' heights stand, only the bed's width is new |

## Craft principle

Marples (corpus): "A large variety of stimuli are used in this manner, one of the more commonly used examples of this is the motion of flame, this can be in the form of lanterns, flaming barrels, or burning objects." A flame is what the eye is sent to, and the eye reads its form at once: a campfire is a heap of tongues of different heights that lean and part, with the hottest and tallest in the middle and a skirt of shorter ones round it. One tongue the size of a man's arm is a gas jet.

Doyle (corpus, the thesis on his colour and lighting): "Doyle uses small single source hard lighting to emulate the practical lighting source from the environment such as a street lamp and candle to set the ambience and environment of the scene." When a single fire carries the frame, the fire is looked at hardest, and an unconvincing one costs the whole shot.

## How shipping games do it

Nguyen, *Fire in the Vulcan Demo* (GPU Gems 1, ch. 6.2): "We started with a single animation, and it worked even better with two or more. In addition, we created particles with a random percentage of difference for a number of attributes: size, position, rate of decay, animation start frame, global transparency, and so on." A fire is many flames each with its own size, place and phase, and the variety is what makes it a fire: the chapter's own Figure 6-10 is "(a) Fire with less variety. (b) Fire with more variety". The 2011 *Advances* fire talk builds every fire from a body of particles each of which owns its emission ("additively accumulate grayscale 'hotness'"), so two additive bodies overlap into a brighter one, which is what a thicker part of a fire does. Games that draw a campfire from flame cards place a few at different heights and angles about the bed and sum them additively.

## Model adopted

`FlameTongues { count, spread }` on a `FlameSite`: `count` tongues (the site's own flame is the first, at the middle and at the site's height, so a one-tongue fire is the flame it was), `spread` the width of the disc their feet stand on, in the host's units. Each extra tongue is one more `Flame` child of the site (a separate `FlameSite`-less flame sharing the site's material, colour, depth fade, catch, snuff and wind), tagged `FlameTongue`, so everything a flame already does, a tongue does: it catches and goes out with its fire, leans in the wind, and is culled by its own box. The sparks and the smoke stay one quad each, on the first flame.

`model::tongue::flame_tongue(index, count, spread, height, seed)` places them (engine-free):

- **Where.** The extra tongues' feet lie on a Vogel (golden-angle) spiral over the disc, between 0.35 and 1.0 of the radius, with a hash of `seed` and the index for a little of the azimuth (±0.3 rad) and of the radius (±8 %), so any number fills the disc evenly and two fires do not repeat each other. The centre tongue is index 0.
- **How tall.** A mound: a tongue's height is `height · (1 − 0.4 · r / R)` times a jitter of 0.70 to 1.15, so the middle is the tallest and the skirt the shortest, never above 0.92 of the middle's, and never more than `MERGE_RATIO · r` (`MERGE_RATIO = 3`): two flames `S` apart are one flame above `3 S` (Delichatsios, below), so a tongue 0.1 m from the middle that stood 0.6 m would be the middle flame's own flank, not a tongue of its own.
- **How it leans.** In, towards the middle tongue: a fire draws its air in at its foot, and a row of parallel tongues (the first build, captured) reads as a crown of candles, where tongues that converge on a tip read as one fire. The sine of the tilt is `INWARD_LEAN · r / R` (`INWARD_LEAN = 0.30`, 17° at the rim), written the way the wind's lean is, so the shader needs nothing new: `update_flames` adds it to the wind's lean, turned to the world's axes by the site's rotation, and caps the sum at the wind's own tilt limit.
- **Its phase.** The site's phase plus a hash of the index, so no two tongues move together (Nguyen: "animation start frame"). Brightness is the fire's, shared: one fire breathes as one.

Delichatsios (home-still catalog): "Flame merging of the fire plumes will occur when their flame height is greater than the merging height, Z_m, defined by Equation 6b", with "Z_m / S = 5 sqrt(rho_f / rho_inf) ≈ 3" for a flame temperature of 1000 K and, from Thomas and Baldwin's experiments, "L_f / S = 3 - 3.3" (7b). Below `3 S` the flames are separate tongues and above it they are one: the model's tongues are the separate parts, and the centre tongue, which is taller than `3 S` for every neighbour, is the merged body they stand against.

## Parameters

| name | value | provenance |
|---|---|---|
| `MAX_TONGUES` | 9 | design default: three rings of the centre's size fill any bed the kinds have; more is overdraw |
| `MERGE_RATIO` | 3.0 | Delichatsios 2007, eqs. 6b and 7b (3 to 3.3) |
| ring radius | 0.35 to 1.0 of `spread / 2` | design default, judged in captures |
| mound | `1 − 0.4 r / R`, jitter 0.70 to 1.15 | design default, judged in captures |
| `INWARD_LEAN` | 0.30 (sine of the rim's tilt) | design default, judged in captures: 0.0 is a row (round 1), 0.30 a fire |
| `FireKind::Campfire.tongues()` | 6 tongues over 0.55 m | design default (a 0.8 m bed with the logs' ring on the outside; the flame stands on the middle part) |
| `FireKind::Bonfire.tongues()` | 9 tongues over 1.6 m | design default (a 3 m pyre; 7 over 1.8 m, the first build, left the middle of a mass of gaps) |
| other kinds | none | a candle, a lantern's wick, a torch and a 0.34 m brazier are one tongue: the bed is no wider than a third of the flame |

## Validation targets

- A campfire and a bonfire, seen from three metres at the eye, from a graze and from above, read as a heap of flames of different heights with a middle tongue the tallest; no two tongues are the same shape; the feet are inside the bed and the skirt does not float off it.
- A one-tongue fire, and every scene that does not use the component, draws zero pixels differently from the build before.
- The layout is a pure function: the same inputs give the same tongues; the centre is at the origin and the tallest; every foot is inside the disc; no tongue stands taller than `MERGE_RATIO` times its distance from the middle; no two feet coincide.
- A fire put out or lit again takes all its tongues with it (catch and `Snuff` are the site's), and in a wind every tongue leans the one way.
- Cost: a campfire is `count` flames, one draw each; `bench_flames` with the component on every flame reads what the entities cost (about 0.4 µs a flame a frame on the CPU), and a fire off the screen costs only that.

## What great looks like

A campfire at night, close: a broad, lively mound of orange with a bright yellow heart, three or four tongues leaving it at different heights and leaning slightly different ways, the tallest in the middle, their feet lost in the bed's glow; the tongues part and rejoin as they breathe at the one rate. From above, a ring of overlapping blobs that fills the bed, not one disc in the middle. The bonfire is the same animal at four times the size: a mass, not a single tongue. Neither shows a row.

## Built, and how it was judged

By rounds, each judged by looking (`examples/campfire.rs`, 1280 × 720, frame 240, a campfire from 2.5 m and a bonfire from 5 m at the eye, a graze, from above, in a 1.5 m/s wind, and a 30-frame sequence at 0.05 s):

- **Round 0, the build before.** One slender tongue (28 cm and 80 cm wide) on a 0.8 m and a 3 m bed: a very large candle. Reproduced before any change (`LIGHTS_TONGUES=0`).
- **Round 1, separate flames, parallel** (a throwaway scene of one `FlameSite` per tongue, then the crate's, 5 tongues over 0.5 m and 7 over 1.8 m): the campfire is fuller and reads as a fire; the bonfire is a row of pointed teeth with gaps between them, a crown, each one a candle. Not yet.
- **Round 2, converging.** Each outer tongue leans in towards the middle one by `INWARD_LEAN · r / R` (0.30 at the rim), the layout's azimuth jitter and the mound's spread widened, and a bonfire given nine tongues over 1.5 m: the tongues overlap into one body that narrows to a tip, and the sequence shows them parting and rejoining. Nine tongues over 1.5 m on a campfire is as wide as its stones and too much.
- **Round 3, the kinds' numbers** (6 over 0.55 m, 9 over 1.6 m) and the final build: campfire, bonfire, wind, graze and top-down judged as below.

Checked against the validation targets: a scene that does not use the component is zero pixels different (ten courtyard views, three with one or two pixels off by 1 DN: the GPU's own noise); the layout's invariants are unit tests (`model::tongue`: every foot on the bed and off the middle, no tongue above `MERGE_RATIO` times its distance from the middle, no two feet within 6 % of the bed's width, every lean pointing at the middle and never more than `INWARD_LEAN`); every tongue catches and leans with its fire (`tongues::tests`). Cost is in `docs/DEMOS.md`: 0.18 µs a flame a frame on the CPU, and about 0.15 ms of the transparent pass with a campfire and a bonfire filling a third of the screen.

Known limits. From a graze across a log pile the tongues in front of the middle one show as narrow vertical streaks between the logs, as the fire would show between them, and a bed seen from straight above is a ring of small discs (each tongue's own overhead disc), not a single heat; a campfire given more than six tongues over its 0.55 m is wider than the stones; the tongues' tip shapes are the one shader's, and differ only by clock and height.

## Deviations

- The tongues are separate quads, not one wide shape in the shader. A wide single quad would need a second per-instance number (the tag's 32 bits are full) and a union of silhouettes in a shader that already carries the wind's lean, the overhead disc and the depth fade; a separate flame carries all of them for free and a fire that does not ask for tongues is untouched. The cost is `count` draws for a fire that asks.
- The merge height is used as a *clamp on a tongue's height* (a tongue cannot stand taller than three times its distance from the middle), not as a simulation of merging: the centre tongue stands for the merged flame.
- The layout, the mound's slope and jitter, the ring radii and the two kinds' counts and spreads are design defaults, judged in captures.

## Pending index

- `10.3210_fst.26.1` (Delichatsios 2007) is in the home-still catalog and was read through `distill_search`; its markdown is not in the local corpus mirror (`~/.cache/bevy_lights_corpus`).
- A source for how many tongues a fire of a given bed shows, measured: none in the corpus (the Delichatsios correlation gives heights of merged flames, not a count of tongues).
