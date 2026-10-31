# A fire's own colour: `FlameSite::color` and `PoolCandidate::color`

Every flame in a scene was one colour. `FlameLook::color` is a resource: the middle of the flame's ramp for **all** of them, and the sparks' colours are constants in `src/embers.wesl` that ignore it. The pool's lights are one colour too (`LightPool::light.color`, the template's). So a host could have a scene of orange torches or a scene of blue ones, and could not have a blue gas lantern, a green witch-fire or a violet shrine brazier among orange torches, which is the commonest thing a fantasy or sci-fi game does with fire, and the commonest thing a level designer does with a light's *colour*: a lit sconce is orange and a cursed one is green. A real light of the host's own (a `Flicker` on a `PointLight`) already takes any colour; the flame drawn on it and the pooled light handed to a torch did not. That is the defect this note fixes: **the light's colour and the flame's colour must be one number**, and a fire must be able to carry it.

## Sources

- **Craft:** `10.7456_tojdac.1674124`, Doğaner 2025, *The narrational role of lighting design in The Lights of the Dark City* (corpus, lines 114 and 120 of the on-disk markdown): the source and its colour are one thing, and a hue is a statement.
- **Production:** `s2011-advances-pdf-slides`, SIGGRAPH 2011 *Advances*, the fire talk (corpus, line 1290): "Particle comp pass converts hotness to a blackbody color texture". A fire's colour is a *ramp* its effect owns, looked up from how hot a point is, not a global tint.
- **Technical:** `PBR3_12_Light_Sources`, Pharr, Jakob & Humphreys, *Physically Based Rendering* 3rd ed. §12.1.1 (corpus): a blackbody's colour is a function of temperature alone, "useful for modeling non-blackbody emitters", which is the statement that a fire that is *not* a soot-incandescent hydrocarbon fire has no temperature to read its colour from and must be given one. And *Flame*, Wikipedia, section "Color" (web, read 2026-10-31 through the API, https://en.wikipedia.org/wiki/Flame; not in the corpus, see `Pending index`): "Flame color depends on several factors, the most important typically being black-body radiation and spectral band emission", and "Specific colors can be imparted to the flame by introduction of excitable species with bright emission spectrum lines".

## Craft principle

Doğaner, on a coloured source seen in the frame:

> "The light source and its colour work together as two elements that cannot be separated from each other in a film. The visibility of the source determines the effect of the colour of the light on the narrative."

and on what a hue says:

> "In sci-fi productions, cool colours are generally preferred; blue or green lights create a sterile, technological and alien atmosphere."

A gaffer does not gel the light and leave the lamp's visible body white; the visible flame and the light it throws are one object, and a viewer who sees an orange flame throw green light reads a mistake, not a mood. The crate already holds that line for brightness (a flame breathes on its light's own curve, `flame-flicker.md`) and for place (the light stands in the flame, `fire-kinds.md`). It did not hold it for colour.

## How shipping games do it

The 2011 *Advances* fire talk renders the emissive part of its particles into a "hotness" texture and then: "Particle comp pass converts hotness to a blackbody color texture, adds it on top of smokey particles". The mapping from *how hot* to *what colour* is a lookup the effect owns, so a different fire is a different ramp over the same hotness, not a tint multiplied over one ramp. A shipping game's fire library is that: a few effects, each with its own palette, one of which is the orange the rest of the scene is lit by.

## Model adopted

A fire has **one colour** (`mid`: the middle of its ramp, what `FlameLook::color` has always been) and the rest of the ramp is *the same flame in that hue*:

- `model::fire_colour::retint(stop, mid)` takes a colour `stop` of the shipped orange ramp and moves it by the transform that takes the shipped mid-colour `DEFAULT_MID = (1.0, 0.55, 0.18)` to `mid`, in HSV on the linear values: the hue rotates by `hue(mid) − hue(DEFAULT_MID)`, the saturation scales by `sat(mid) / sat(DEFAULT_MID)` (clamped to 1), the value by `val(mid) / val(DEFAULT_MID)`. A `mid` with no hue (a grey or white fire) keeps the hue and drops the saturation to the grey's own (a magnesium-white flame whose rim is dark grey and whose core is white).
- **The fire family keeps the shipped ramp.** The rotation alone is wrong near the shipped colour, and the first build found it: the courtyard's `FlameLook::color` is a 1900 K blackbody, `(1.0, 0.18, 0.0)`, 16° redder than `DEFAULT_MID`, and rotating the rim (4.7°) by −16° put it at −11.6°, a **pink** rim on every torch in the showcase (a capture diff against the old build: 3 514 pixels differed by 1 % or more, up to 66 DN, all on flame tips). The ramp's rim and core were tuned for the red-to-yellow arc of hues (a wood fire, a candle, a coal, an ember) and a scene has always been free to move its middle colour inside it without moving them. So `retint` is the identity for a `mid` whose hue is in `[0°, 60°]` and whose saturation is at least 0.4, and fades into the rotation over the 60° of hue beyond either end of the arc (and over saturation 0.1 to 0.4): a green, blue, violet or cyan fire has turned all the way, a white one has no hue and is grey, a yellow-green one is half-way. The ramp is continuous in `mid` (a hue slid across the wheel changes a stop by at most 0.03 a half-degree).
- The flame's ramp is `{ edge: retint(EDGE_RED), mid, core: retint(CORE_YELLOW), ember: retint(EMBER_RED) }`, computed on the CPU and written into the flame material, where they replace three constants of the shader. The sparks' three stops (`DULL_RED`, `ORANGE`, `CORE_YELLOW`) are retinted the same way: the sparks are the fire's own colour.
- For every `mid` in the fire family `retint` is the identity (not an approximation: the function returns its input), so **a scene that sets no colour, or any red, orange or yellow of its own, draws exactly the pixels it drew before**.
- A flame with `FlameSite::color: Some(c)` is drawn with a material of colour `c` (`FlameLook::color` for `None`): one material per distinct colour (and per depth-fade setting), shared by every flame of it, held by the flames that use it and released with the last of them. A scene that sets no colour has the two materials it had.
- A pooled light takes `PoolCandidate::color: Some(c)` from the candidate that holds it, on the frame the owner changes (the light is fading in from nothing then, so the change is not seen) and the departing owner's light keeps the old colour as it fades out. A candidate with `None` leaves the light as the template (or the host's palette) set it; a light the crate tinted is put back to the colour it had when the next owner has none.
- The hue is the host's. `FireKind` carries no colour: the six kinds are one fuel and one hue (soot-incandescent wood, wax and oil), so the kind's colour is the scene's. A host that wants a blue-gas kind sets `color` on the site and the candidate it builds from the kind.

## Parameters

| number | value | provenance |
|---|---|---|
| `DEFAULT_MID` | `(1.0, 0.55, 0.18)` | `FlameLook::default().color`, `flame.md`: design default (a campfire's orange) |
| `EDGE_RED` | `(0.70, 0.06, 0.005)` | `src/flame.wesl`, unchanged: design default |
| `CORE_YELLOW` | `(1.0, 0.78, 0.30)` | `src/flame.wesl`, unchanged: design default |
| `EMBER_RED` | `(0.95, 0.20, 0.02)` | `src/flame.wesl`, unchanged: design default (the catch's colour, `flame.md` iteration 9) |
| sparks `DULL_RED`, `ORANGE`, `CORE_YELLOW` | `(0.80, 0.11, 0.01)`, `(1.0, 0.42, 0.07)`, `(1.0, 0.78, 0.30)` | `src/embers.wesl`, unchanged: design defaults |
| hue/saturation/value transform | as above | design default: it is the smallest map that takes one ramp to another |
| the fire family | hue `[0°, 60°]`, fading over 60° beyond each end; saturation fading in over `0.1..0.4` | design default: the arc of hues the shipped rim and core were tuned for, and the arc every scene's `FlameLook::color` has lived in (the courtyard's 1900 K blackbody is at 10.8°, `DEFAULT_MID` at 27.1°) |

The hue offsets the transform preserves are the shipped orange ramp's own: the rim sits `-22.3°` from the middle (hue 4.7° against 27.1°), the core `+14.1°`, the catch's ember `-15.5°`, the sparks' `DULL_RED` `-19.5°` and `ORANGE` `-4.5°`; a blue fire (`(0.1, 0.3, 1.0)`, hue 226.7°) therefore has a cyan-blue rim (204°), a blue middle and a pale violet-white core (241°).

## Validation targets

- `retint` is the identity for every stop at every mid in the fire family (the scene's orange, a 1900 K blackbody, a candle's yellow, a deep red, an HDR fire); at a mid well outside it (blue, green, violet, cyan) the shipped mid retints to exactly that colour; and it is continuous across the whole hue wheel.
- Every retinted stop is finite and non-negative for any `mid`, including zero, NaN, negative and above one (a palette's HDR fire).
- A flame with no `color` draws **zero** differing pixels against the old build (courtyard, default and ground cameras, two seeds).
- A flame with a `color` and a pooled light with the same one read as one fire: a captured frame of a blue torch has its blue light on the stone round it and its flame blue; the sparks follow.
- No pop: a pool's per-half mean luminance across a hand-off between candidates of different colours departs from the mean of its neighbours by no more than the existing pop test's 1 DN (`docs/DEMOS.md`).
- No leak: a scene whose flames' colours animate every frame (a host cross-fading a colour) holds a bounded number of materials.

## What great looks like

A torch-bearer in a blue-flamed gas lantern's light that reads as one lamp: the body of the flame a clean saturated blue with a darker rim and a pale core, as the orange flame has its red rim and yellow core, and not as an orange flame tinted blue (a muddy grey where the red rim and the blue body meet); the stone round it blue-lit and cool against the orange pools of its neighbours. The cinematographer's complaint is the one Doğaner names, a visible source whose colour and whose light disagree.

## Deviations

- The sparks used to ignore `FlameLook::color` (fixed orange stops); they now follow it, so a scene whose `FlameLook::color` is not the default gets sparks in its own hue. The default scene is unchanged to the pixel.
- `FlameLook::color`'s green channel is still clamped to 1 before the ramp is built (a palette's oversaturated fire does not blow the ramp out); a per-site colour is clamped the same way, so the two paths agree.

## Pending index

- *Flame*, Wikipedia (web; the section on colour is quoted above) is not in the home-still corpus. A primary source for the chemiluminescence of premixed flames was not read: Kojima, Ikeda & Nakajima 2005, *Basic aspects of OH(A), CH(A), and C2(d) chemiluminescence in the reaction zone of laminar methane-air premixed flames* (Combustion and Flame, DOI 10.1016/j.combustflame.2004.10.002, from `paper_search`) has no open copy; Wrede et al. 2025, *Fire-X: Extinguishing Fire with Stoichiometric Heat Release* (DOI 10.1145/3763338, abstract only: "realistic rendering of flames (e.g., laminar-to-turbulent transitions and blue-to-orange color shifts)") is behind ACM's 403. `paper_download` refused both on 2026-10-31.
