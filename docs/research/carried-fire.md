# A carried fire: the light sits where the flame is

A firelight that a figure carries is the crate's most common practical — a torch in a hand, a lantern on a
pole — and it is the one case where the crate's own shipped examples get the geometry wrong. The light a
torch is handed is placed **1.8 m above the flame drawn on that torch**, in mid-air over the bearer's
head, and sized for a slant it no longer has: 1.8 stops darker than the pool it was sized to light, and
a source the viewer cannot find. Measured on the current `main`, from the courtyard example's own debug
line: every handed-out torch light sits at `y = 3.60` while the nearest flame is at `y = 1.80`.

The defect is not the offset arithmetic alone. The crate gives a host three numbers to reconcile — the
`FlameSite`'s local transform, its `height`, and a `PoolCandidate::offset` in **world** axes — and no
statement of how they relate. A carried fire is where that shows first, because the flame and its light
are two entities on one prop and the prop is itself a child of a walking figure.

## Sources

| source | what it is | use |
|---|---|---|
| `10.7456_tojdac.1674124` — Doğaner, *The narrational role of lighting design in The Lights of the Dark City* (2025) | why a visible source has to be believed, and where the audience looks for it | the craft rule: the light's origin is read from the source object, and an unexplained light costs the scene its reality |
| `10.23856_6105` — *Harmony of light and shadow* | the "drawing" light and its placement against the subject | the craft's own vocabulary for a source's position as a number, not a mood |
| `10.13140_marples.nav.lighting.thesis` — Marples 2017, PhD thesis, *The influence of intrinsic perceptual cues on navigation and route selection in virtual environments* | shipped games' lighting for guidance: flame motion, Left 4 Dead's lit junctions, Alan Wake's carried torch | **the production source**: a flame is used as a light *and* an attractor at once, so the two must be one object |
| `10.1007_978-1-4842-4427-2` — *Ray Tracing Gems* (Haines & Akenine-Möller 2019), ch. 18 | the solid-angle form of a light's contribution: `dω = (n·−l)/‖X−Y‖² dA` | the technical claim: a light's *position* is its emitter, and the illuminance is inverse-square in the distance from that point |
| `PBR3_05_Color_and_Radiometry` — *Physically Based Rendering* 3rd ed., ch. 5 | irradiance `E = Φ/4πr²`, and Lambert's law | the same statement from the radiometry side, so the error can be stated in stops |
| `10.1017_jfm.2018.707` — Xia & Zhang, *A vortex-dynamical scaling theory for flickering buoyant diffusion flames* (JFM 2018) | a fire's flicker rate against its fuel bed | already adopted by this crate; the reason a carried fire's light and flame must share one `diameter` as well as one place |
| `docs/research/lighting-look.md`, `docs/research/flame.md`, `docs/research/light-pool.md` | this crate's adopted model for a practical, a flame and the pool | the parameters this note is consistent with |

## Craft principle

Doğaner states the mechanism the viewer uses, and it is about the **source object**, not about the light:

> "when the audience associates a visible light source in the environment of the characters, they accept
> the reality of space and time more easily. This method of illumination, which takes place within the
> story world … and which the audience sees first-hand, creates a sense of reality that is consistent
> within itself." (`10.7456_tojdac.1674124`)

The same paper separates the case that fails: lighting "whose physical source is not explained … finds
an intuitive response in the viewer's perception rather than reality". A light 1.8 m above the flame that
is its source is the second kind, and it is the kind a scene pays for. Marples' thesis is the same rule
on the games side, and it makes the flame do double duty:

> "A large variety of stimuli are used in this manner, one of the more commonly used examples of this is
> the motion of flame, this can be in the form of lanterns, flaming barrels, or burning objects. For
> instance, Left 4 Dead (Valve Corporation, 2008) uses both the motion and simultaneous illumination
> offered by flames, commonly flaming oil barrels." (`10.13140_marples.nav.lighting.thesis`)

"Both the motion and simultaneous illumination offered by flames" is the whole requirement in one phrase:
the thing the player watches and the thing that lights the street are the *same* flame. And the thesis
records the carried case explicitly — Alan Wake's "the torch the eponymous protagonist carries being
required to make the 'darkness' able to take damage" — where the flame's position is the player's hand
and the light's position is the flame's.

The craft's own number for a source's placement is the photography tradition's "drawing light", which
`10.23856_6105` treats as a position with rules rather than a taste:

> "The light source that forms the main light and shadow pattern on the face of the person being
> photographed is called the 'drawing' light source. It is always located in the anterior hemisphere
> relative to the face, meaning that the person can see it without turning their head."

A carried fire is the one practical whose drawing light the *character* can see. That is exactly why its
position is not a free parameter.

## How shipping games do it

The production answer to "where does the light go" is that it **is** the emitter, and the only question is
how coarsely the emitter is modelled. *Ray Tracing Gems* ch. 18 states the contribution of a small light
patch as a solid angle that falls off with the square of the distance from the shading point:

> "In fact, a small patch dA at a point Y on a light source covers a solid angle
> `dω = |n × −l| / ‖X − y‖² dA`" (`10.1007_978-1-4842-4427-2`)

so a light's position is not a hint about where the source is — it *is* the source, and moving it moves
every illuminance in the scene by the square of the distance. `PBR3_05_Color_and_Radiometry` gives the
same law in the form a gaffer would use: `E = Φ/4πr²`, "This fact explains why the amount of energy
received from a light at a point falls off with the squared distance from the light."

The coarse modelling shipping games do is *radius*: a practical is an area emitter approximated by a
point, and the approximation is honest while the point is inside the glowing object. It stops being
honest the moment the point leaves it, because then the light has a visible source that does not emit.

Marples records what a shipped game does with the flame's own geometry, and the reason it is the flame
that is drawn rather than a glow: the motion "naturally draw[s] the eye of a player", and Left 4 Dead
"uses both the motion and simultaneous illumination offered by flames". A carried torch whose light is a
metre overhead has motion in one place and illumination from another, which is the one arrangement the
technique is not used for.

## Model adopted

**What was built.** Three parts, one commit each:

1. `model::reach::flame_light_offset(height)` — where a fire's light belongs, relative to a
   [`FlameSite`]'s own origin: `(0, height/2, 0)`. It is the flame billboard's own centre
   (`src/flame.wesl`'s vertex stage lifts the quad `size · 0.5` above the site's origin), so a light
   placed by it sits in the middle of the fire rather than at its foot or over the bearer's head. A plain
   function in `src/model/`, tested without an app.
2. `PoolCandidate::offset` documented against that helper, because the offset is in **world** axes and a
   candidate on the `FlameSite` entity must not also carry the lift its transform already has. The trap
   the courtyard fell into is spelled out where a host will read it.
3. The courtyard, the README quick start and the crate's doc example place a carried fire's light and its
   flame from the same numbers: a hand at one point, the flame's foot there, the light at
   `flame_light_offset(flame height)`, sized for the slant the light actually has.

**What it fixes, in numbers.** On `main`, a lantern-bearer's light is sized by
`torch_lumens(2.0, 0.95)` = 15 402 lm, which delivers 250 lx at 2 m from a light 0.95 m over the ground.
The light was actually at 3.6 m, so the same lumens deliver
`15402 / (4π · (2² + 3.6²))` = **72 lx** at that edge — **1.79 stops down**, and the pool the torch was
sized for is not lit at all. The light also has no source within 1.8 m of it.

**The design default.** A fire's light goes at the flame's centre, not at its tip and not at the fuel:
the centre is where a point-light approximation of a flame of that height is least wrong for the ground
around it, and it is the one point that is inside the drawn fire at every `height`. Design default.

## Parameters

| parameter | value | source |
|---|---|---|
| a fire's light offset from its `FlameSite` origin | `(0, h/2, 0)` for a flame `h` tall | the billboard's own centre in `src/flame.wesl` (`+ size * 0.5`); design default for the choice of centre rather than tip |
| a carried torch's flame height, courtyard | 0.35 m (0.5 m for a lantern) | unchanged from `main`; `docs/research/flame.md` |
| a carried torch's hand | 0.30 m out, 0.45 m up the bearer's side, 0.12 m forward | design default: out of the figure's own silhouette, which is where a hand is; the point is that the flame and its light share it, not the exact number |
| a torch's pool | 2.0 m to its edge at 250 lx | `examples/courtyard.rs` `TORCH_POOL`, from `docs/research/lighting-look.md` |
| the light's `over` term when sizing a torch | the height of the light above the ground, i.e. the hand's, not a separate number | `fire_lumens(lux, reach, over)` is only correct for the `over` the light actually has; the bug this iteration fixes was passing one `over` and placing the light at another |

## Validation targets

For `model::reach::flame_light_offset`:

- `flame_light_offset(h).y` is exactly `h/2` for `h` in 0.05, 0.35, 0.5, 1.1, 4.0.
- The offset's `x` and `z` are exactly zero: a fire's light is on its axis, so a host adds nothing else.
- A zero, negative or `NaN` height gives a finite offset with `y >= 0`, so a misconfigured flame gets a
  light at its foot rather than a light below the floor.

For the scene, measured on the courtyard's own debug output and captures:

- **Every handed-out torch light is inside its own flame's drawn volume.** Measured as the distance from
  the light to the nearest flame centre, which must be under half that flame's height. Baseline: 1.80 m
  from the nearest flame, which is **5.1×** the 0.35 m flame's half-height.
- **A torch's pool reaches its target illuminance.** The ground 2 m from a lit torch at 250 lx, against
  72 lx on the baseline.
- No new clipping, no CPU regression, and the flicker/swing unchanged.

## Deviations

- **The light's height and the flame's height are two numbers that happen to be equal, not one number.**
  A `FlameSite` with `kindle > 0` grows its flame from nothing, so its light's height is its *full* height
  from the first frame even while the flame is small. That is deliberate: `Kindle` fades the light's
  intensity, and a light that slid up the flame as it caught would be a new artefact for a 0.6 s event.
  Recorded here because it is the one place the two are not literally derived from each other.
- **The crate still does not place the light for the host.** `flame_light_offset` is a helper, not a
  system: `PoolCandidate::offset` stays the host's, because a host may pool a light on an entity with no
  flame at all (the pool's own docs say so) and because a crate that moved a host's candidate would be
  owning a prop's layout. This is an API-documentation fix plus one function, not a new component.
- **`PoolCandidate::offset` remains in world axes and is not rotated with the candidate.** That is right
  for a torch swung by an arm (`light-pool.md`), and it is why a *carried* fire's offset is best computed
  from the flame rather than eyeballed per prop.

## Pending index

Read live from the running library on 2026-10-26; all sources above are indexed. Gaps:

- No source in the library gives a **measured** offset between a flame's visual centroid and the
  light-estimating point a viewer infers for it. The centre is therefore a design default. A
  psychophysics study of source localisation for small emitters would replace it.
- `10.1145_3072959.3095816` and `10.1080_10509674.2013.782936`, both added to the library on 2026-10-26,
  have no catalog title; they are listed in `corpus.md` as unclassified rather than cited.
- The flicker amplitude gap is unchanged (15 iterations): Chamberlin & Rose 1928 is not open access and
  Maynard 2013 is unobtainable.

## What great looks like

Judged at the courtyard's ground-level camera, the close torch camera, and the hero camera:

- **The light has a source you can point at.** At every angle, a lit torch's bright pool on the ground
  and on the nearest figure has a flame at its centre. No pool without a flame, no flame without a pool
  anywhere its light reaches.
- **The flame is where a hand would be.** A torch's flame is out to the bearer's side and below the head,
  never growing out of the skull, and it moves with the walking figure without lag.
- **The torch lights the street it walks.** A figure carrying a real light puts a readable patch of warm
  ground under itself at the hero camera, brighter than the moon fill by about a stop, and the patch is
  centred under the flame.
- **Nothing about the fire changed.** Size, colour, flicker rate, swing, kindle and the hand-off's
  cross-fade are the same numbers as before; only the geometry they are applied at moved.
- **Cheap.** One helper function; no new system, no new per-frame work.

[`FlameSite`]: ../src/flame.rs
