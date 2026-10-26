# The lighting look: key-to-fill, motivated sources and the read

The crate's three systems each do one job correctly and the courtyard still does not read as a night
scene. Measured on the baseline captures, not guessed:

- **The fire's falloff is right.** On the top-down frame's radial profile from a brazier's flame, the
  ground runs from 0.906 linear luminance at the flame to 0.0069 six metres out, a clean inverse
  square with no hard cut-off at `range`. That part needs nothing.
- **The pools are blown.** The same profile peaks at sRGB 255, 245, 232, and four metres of ground
  around every brazier sits above 200 DN. A blown pool is where a fire stops reading: the flame, an
  additive sprite, is the same brightness as the ground under it, so the frame has no hot spot.
- **There is no key-to-fill.** Every warm surface in the courtyard is lit by another fire's pool: four
  braziers of 60 000 lm with a 12 m range in a 10 m box overlap across the whole floor. Nothing
  anywhere in `src/` computes a ratio or a colour temperature; the fire's colour is a hand-picked
  `srgb(1.0, 0.60, 0.30)` and the moon is a hand-picked 0.3 lx.
- **The stone has no specular answer.** One roughness of 0.9 everywhere, so no fire puts a highlight
  on anything and no surface says "this is stone, and the fire is over there".

This note is the cinematographer's numbers that turn that into a night — what the lighting *is for*,
not how a single light moves. The three other notes own motion ([`fire-light.md`]), budget
([`light-pool.md`]) and the drawn fire ([`flame.md`]); this one owns **composition**: the ratio, the
colour temperature, and the size of a pool against the fire that lights it.

## Sources

| source | what it is | use |
|---|---|---|
| `10.4324_9781003671220` — Tal Lazar, *Cinematography Beyond Technique* (2026), ch. 10–11 | a story-first language of light: quality, emulation, the objectives of lighting, practical lighting | the vocabulary and the checklist this note adopts |
| `10.7456_tojdac.1674124` — Doğaner, *The narrational role of lighting design in The Lights of the Dark City* (2025) | motivated vs unmotivated sources; hard/soft as narrative | motivated-source rule and warm/cool separation |
| `10.2478_bsmr-2022-0020` — Huttunen, *Ecological approach to cinematographic lighting of the human face* (2022) | light direction, underlighting, "Godfather lighting", why we read light from above | the direction convention for a fire, and what it costs to break it |
| `10.5897_jmcs2022.0788` — Vanden Bossche, *Looking for low-key* (2023) | noir as *maximised* tonal contrast, not a palette | why the shadows must go dark for a fire to read |
| `10.32657_10220_48312` — *Strategies of colour and lighting design in the cinematography of Christopher Doyle* | chromatic zoning; chiaroscuro; specular placement; single-palette tonal control | the reference for a frame with one warm source |
| `10.23856_6105` — *Harmony of light and shadow* (2024) | source size against distance: the soft/hard table; inverse-square as the practitioner's rule | the falloff and softness numbers |
| `s2007-advances-pdf-slides-2-8-mb` and `s2007-advances-course-notes-0-4-mb` — Mitchell, Francke & Eng, *Illustrative rendering in Team Fortress 2* | the cool-to-warm hue shift on the half-Lambert term, and terminator reddening | how a shipping game keeps a figure readable in any light |
| `s2011-advances-pdf-slides` — *SIGGRAPH 2011 Advances*, the fire talk | emissive "hotness" mapped to a blackbody colour | a fire's colour as a temperature, not a hand-picked RGB |
| `PBR3_12_Light_Sources` — *Physically Based Rendering*, ch. 12 | blackbody emission, colour temperature, "over 5000 K cool, 2700–3000 K warm" | the numbers behind a warm practical against a cool fill |
| `book_real_time_rendering_4th_akenine` — RTR4 §8, §9 | tone mapping, light and colour, metals' specular F0 | why a hot core must be below clipping |

## Craft principle

Lazar's chapter 10 sets out what lighting is *for*, and it is the frame this crate's docs adopt:

> "Lighting is not only expected to make visual elements visible. While light provides a wide range of
> tones in the scene, it maintains the consistency of colour values and plays a determining role in
> colour balance. … Lighting also assumes functions such as emphasising the textures and details of
> surfaces, creating atmospheric effects, and adding emotional depth to the narrative."

A practical is a **motivation**, not the exposure. Doğaner quotes Bordwell & Thompson on exactly this,
and it is the rule that governs a game's torches as much as a film set's:

> "In most fiction films, the table lamps and street lamps you see in the mise-en-scene are not the
> main sources of lighting for filming. … But these visible light sources will motivate the lighting
> decisions made during the production. The director will usually try to create a lighting design that
> is compatible with the resources on set."

And the two-source minimum that makes a scene read rather than merely be visible:

> "the lighting of the scene will start from the assumption that any subject normally requires two
> light sources: a key light and a fill light." (Bordwell & Thompson 2004, quoted in
> `10.58709_niujhu.v8i3.1694`)

The ratio between them is the composition. Vanden Bossche's account of noir is that the style is
*Hollywood lighting pushed to an extreme*, not a different lighting:

> "noir is Hollywood mannerism, it takes the widely accepted ideal of expressivity and extends it to new
> extremes" (Keating 2010, quoted in `10.5897_jmcs2022.0788`), and "Chiaroscuro … stark contrasts in
> lighting, deep focus cinematography … tended to be more present in films noir."

Hardness is a function of the source's *size against its distance*, and the practitioner's table is
explicit (`10.23856_6105`, Table 1): soft with no visible shadow at 1:1, soft with smooth transitions
from 1:3 to 1:6, "sufficiently contrasty lighting with fuzzy shadow contours" from 1:6 to 1:10, and
"contrast lighting with clear shadow boundaries" at 1:10. A torch is a fist-sized flame: at 2 m it is
1:40 and gives hard shadows; a brazier's coal bed is 0.66 m across, so at 3 m it is 1:5 and reads soft.
That is the same fire reading two ways by distance, and it is free.

Direction is a convention with a cost when broken. Huttunen, on light from below:

> "Because light does not usually come from below in nature and it, therefore, presents the face
> unnaturally, it was – and still is – often used to call the audience's attention to criminal
> characters … it can make a character look 'weird, dangerous, or at best untrustworthy'."

So a fire must read from **above and to one side** of what it lights — a fire at a figure's feet is
underlighting — which is what a torch held at 1.8 m and a brazier at 0.9 m already do, and what a
flame on the ground does not.

Colour: PBR's chapter 12 is the number to quote — "Generally speaking, color temperatures over 5000 K
are described as 'cool,' while 2700–3000 K is described as 'warm'." A wood flame is far below that,
around 1800–2000 K, and the SIGGRAPH 2011 fire talk is the production statement that its colour is
*derived from a blackbody*, not authored ("Particle comp pass converts hotness to a blackbody color
texture").

## How shipping games do it

Team Fortress 2 is the published statement of the cool-to-warm rule as a *shading* rule, quoted here
from the course notes' technical section:

> "This is consistent with our observations that illustrated shadows often tend toward cool colors, not
> black, and that there is often a slight reddening at the terminator."

and its purpose is legibility, which is the same problem a dark courtyard has:

> "the shading techniques are designed to quickly convey geometric information in our desired
> illustrative style using variation in luminance and hue, so that game players are consistently able
> to visually 'read' the scene and identify other players in a variety of lighting conditions."

That is the bar: a figure at the edge of a fire's reach must be readable, and it is read by *hue*
(warm lit side against a cool shadow side) rather than by luminance alone, because luminance in a dark
scene runs out.

The other production statement this note leans on is the dark-city lesson from `s2006-advances-full-
course-notes-pdf`: a night needs more than one apparent source —

> "even in the midst of a night there is light in our somber downtown corner. Notice the large number
> of perceived light sources visible … such as the bright street lights, the blinking neon sign … the
> sky also appears to illuminate various objects".

A frame with one source is a brown wash; a frame with a warm key, a cool sky fill and a specular
answer on the stone is a night.

## Model adopted

One engine-free module, `src/model/cinematography.rs`: the craft's numbers as plain functions, so a
host computes what a cinematographer computes and a capture can be judged against the same numbers. It
lives in `model/`, so it needs no app and no GPU to test.

1. **`key_to_fill(key_lux, fill_lux)`** and **`stops(ratio)`** — a scene's key over its fill, as a
   number and in stops. The module names the craft's three conventional ratios with the reason for
   each: `LOW_KEY` 8:1 (3 stops, a night exterior), `MOON_KEY` 4:1 (2 stops, a courtyard where the
   fire must clearly win) and `HIGH_KEY` 2:1 (1 stop, a lit interior). Two sources are the minimum and
   the ratio between them is the composition; the crate had no vocabulary for it at all.
2. **`fill_for(key_lux, ratio)`** — the inverse, and the useful direction: a host knows what its fire
   puts on the ground and wants a night, so it asks for the fill illuminance that makes 8:1 instead of
   hand-picking a moon. That hand-picked 0.3 lx is the baseline's fault.
3. **`fire_lumens(lux, reach, over)`** — the lumens a practical needs for the ground `reach` out and
   `over` below it to receive `lux`. Bevy maps a point light's lumens to Φ/4π candela, so the inverse
   square inverts exactly, and this is the same geometry as `raised_light` the other way round. This
   is how a pool gets a **size** instead of an arbitrary 60 000 lm: ask for "12 lx at 4 m" and the
   number falls out. The baseline's 60 000 lm at 0.9 m is 20× what a 0.66 m coal bed puts out and it
   blows the pool it lights.
4. **`blackbody_colour(kelvin, strength)`** — a light's colour from its colour temperature: Planck's
   law integrated against the CIE 1931 observer, converted to linear sRGB and normalised to
   `strength`. 1900 K is a deep red-orange and 8000 K a cold blue, from one formula rather than a
   hand-picked triple, so a fire is coloured by the temperature it burns at.

`courtyard` is the proof and the demonstration: it sizes every brazier's and every torch's lumens from
a pool radius and a target lux, colours the fires at 1900 K and the moon at 8000 K, derives the moon's
illuminance from the target ratio, and **prints the measured key-to-fill** at a brazier's pool against
the shadow, so the number this note claims is the number the frame has.

Nothing here lights anything on its own. The crate does not own the moon and never adds a
`DirectionalLight`: the fill is the host's, and the crate hands it the number.

## Parameters

| parameter | value | source |
|---|---|---|
| fire colour temperature | 1900 K | design default in the wood-flame band (1800–2200 K); PBR ch. 12 for "warm" being 2700–3000 K and below |
| moon fill colour temperature | 8000 K | design default: PBR's "over 5000 K is cool"; a moon is reflected sunlight and reads blue to a dark-adapted eye |
| courtyard key-to-fill target | 4:1 (2 stops, `MOON_KEY`) | design default between the two-source minimum and low key; `10.5897_jmcs2022.0788` for contrast as the noir convention |
| low-key reference | 8:1 (3 stops, `LOW_KEY`) | design default: Vanden Bossche's "stark contrasts", `10.23856_6105` Table 1 for the softness that keeps it readable |
| high-key reference | 2:1 (1 stop, `HIGH_KEY`) | design default, `10.58709_niujhu.v8i3.1694` for key and fill as the two-source minimum |
| brazier's pool | 12 lx at 4 m, from a coal bed 0.9 m over the ground | design default for `ev100` 3.0 (Bevy's exposure: 12.5 lx at `ev100` 3 is mid-grey). It sizes the lumens through `fire_lumens`, replacing the baseline's 60 000 lm |
| torch's pool | 12 lx at 2 m, from a flame 1.8 m over the feet | the same, and `10.23856_6105` Table 1: a 0.35 m flame at 2 m is 1:6, "sufficiently contrasty … with fuzzy shadow contours" |
| brazier softness | 0.66 m source at 3 m (1:4.5) | `10.23856_6105` Table 1: 1:3–1:6 is "soft with smooth transitions" |

## Deviations

- **The crate does not meter exposure.** The pool sizes are a *look* number in lux, and nothing here
  multiplies a light by a camera's `Exposure`. The ownership map gives exposure to `bevy_elements`, and
  the example states the `ev100` its lux are chosen for.
- **No Purkinje shift.** A dark-adapted eye loses colour and shifts blue; modelling it needs a
  post-process this crate must not own, so the moon's 8000 K is a stand-in and this note says so
  rather than implying the crate simulates scotopic vision.
- **The pools are sized smaller than the baseline's.** 4 m for a brazier against a 12 m `range` is a
  deliberate cut: the fire's reach is what it can light to 12 lx, and the falloff past it is Bevy's
  windowed inverse square rather than a plateau. It also stops four braziers lighting the whole floor
  between them, which is what made every surface warm and the ratio meaningless.

## Validation targets

- `key_to_fill(12.0, 1.5)` is 8.0 within 1e-3, and `stops(8.0)` is 3.0 within 1e-5.
- A zero or negative fill does not divide by zero or return NaN: the ratio clamps at a finite ceiling
  and `stops` of it is finite.
- `fill_for(12.0, LOW_KEY)` round-trips through `key_to_fill` to `LOW_KEY` within 1e-3.
- `fire_lumens(12.0, 4.0, 1.2)` puts 12 lx on the ground at 4 m: the `raised_light` intensity for
  those lumens, over `4π · slant²`, is 12 lx. The two functions are inverses, and a test says so.
- `blackbody_colour(1900.0, 1.0)` is warmer than `blackbody_colour(8000.0, 1.0)` (red over blue is
  larger), both are finite and in gamut, and `strength` scales all three channels by the same factor.
- In the courtyard: **no lit ground pixel sits at 255, 245, 232 or brighter at a brazier's pool** (the
  baseline's peak), the measured key-to-fill at a brazier's pool against the shadow 4 m away is **at
  least 4:1**, and a bearer at the pool's edge reads as a warm rim on a cool body.

## Iteration 8: the specular answer, and where a highlight belongs

**The defect, measured.** The ground is one material (`polished_stone`, roughness 0.55, reflectance
0.06) over a single 40 × 40 m plane. On the ground-level capture its luminance runs to a maximum of 217
but **p99 is 136**: a broad gradient with no local maximum anywhere. A fire in the room puts no
highlight on anything, so the frame never says *where* the fire is except by the falloff of its pool.

**What the craft says.** Doyle's cinematography, in the corpus, names the technique this is missing:

> "Christopher Doyle uses specular lighting techniques in *Fundamentally Happy* (2015). Specular lighting
> is a directional and focus lighting technique to concentrate and highlight the specific area or action
> in the scene. The lighting technique create strong highlights and shadows within the frame, thus
> bringing viewer's attention to the lit area." (`10.32657_10220_48312`)

and RTR4 is the reason it has to be *tight*: a highlight is the mirror lobe of the BRDF, so its size is
set by the surface's roughness — a rough surface spreads the lobe until it is indistinguishable from the
diffuse term, which is exactly what 0.55 does across a 40 m plane.

**Model adopted.** No new API: a highlight is a *material* property, and the crate owns lights, not
surfaces. The change is in the example, which is where the look is demonstrated:

- The courtyard floor becomes a **grid of flagstones** — one plane per stone, with a small gap — instead
  of one plane. Each stone gets a roughness between 0.18 and 0.32 from the example's seeded `Draw`, so
  the floor is not uniform and some stones answer a fire sharply while their neighbours stay matte. That
  irregularity is the point: a floor of identical mirrors reads as a wet car park, and a floor of
  identical matte reads as paper.
- Each stone is **inset by a mortar gap**, so the mortar (rougher) shows between them and the frame has
  a second, darker material in the same plane — the "two roughnesses in one surface" that makes a
  highlight legible against its neighbours.

**Validation target.** On the ground-level capture, the floor's luminance distribution must gain a tail
that a broad gradient cannot produce: p99.9 at least 1.4 × p99 (today 1.4 × 136 / 188 — the ratio is
already 1.38, so the number that has to move is the **spread**: the count of pixels within 12 DN of the
frame's maximum must go from a handful to a placed patch, and the maximum must sit over a fire's pool
rather than at a flame's core).

**Deviations.** The flagstones are a demo material, not a crate one: `bevy_lights` must not ship a
stone shader, and a host's own floors are its art direction. What the crate gains is nothing — which is
correct for this item — and what the *crate's note* gains is the measurement and the craft behind it.

## Pending index

Nothing from this note is unindexed: all nine stems were read live from the running library on
2026-10-26 and are listed under `Sources`. Gaps worth importing later:

- Birn, *Digital Lighting and Rendering*, and Katatikarn & Tanzillo, *Lighting for Animation* — the two
  CG-lighting books `corpus.md` already names as absent, and the ones that would replace several
  "design default" rows above with a citation.
- Hunter, Biver & Fuqua, *Light: Science and Magic* — for the family-of-angles treatment of a
  specular highlight on stone, which `flame.md` and this note both leave to a design default.
- A fire-physics paper with a colour-temperature measurement of a wood flame, to replace the
  1800–2200 K band with a cited number (the same gap `fire-light.md` records for the flicker rate).

## What great looks like

A frame a cinematographer would call a night, judged at full frame and at 4x on the fires:

- **One warm key with a visible edge.** Each brazier has a pool whose brightness falls by half within
  about two metres and reaches the moon's level by six — a shoulder, not a plateau and not a disc.
- **Cool shadows, not black.** The unlit ground and the shadow side of every pillar carry a blue-cyan
  tint at 8–15 DN; the fire's shadow side carries a slight warm terminator, and no surface inside a
  fire's reach is dead black.
- **Motivated everywhere.** Every pool has a visible flame at its source, and the flame's size is an
  order of magnitude below the pool it lights (a 0.35 m torch flame lighting a 4 m patch is wrong; a
  0.35 m flame lighting 2 m is right). No light in mid air without a flame, no flame without a pool.
- **Figures readable at the fire's edge.** A bearer at the edge of a brazier's reach reads as a person
  in silhouette with a warm rim and a cool face, not as a black capsule.
- **A specular answer.** Stone near a fire shows a small, placed highlight where the fire is; the
  ground does not look like matte paper.
- **The flame does not clip.** Its core is bright but its hottest channel is a little under 255, so a
  bloom pass would have headroom; the fire's colour is the same temperature at every fire.
- **A night that reads as a night.** With the fires out, the frame is legible and blue; with them lit,
  the frame is warm where the fire reaches and blue where it does not, and the boundary between the
  two is soft.
