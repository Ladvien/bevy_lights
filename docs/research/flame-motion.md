# A fire that moves leans against its own motion

Every flame the crate draws leans only in the wind (`FlameWind`, `docs/research/flame-wind.md`). The courtyard's 48 torch-bearers walk circles at 0.15 to 2.4 m/s and every one of their flames stands as upright as a brazier's, and the pooled light each is handed stands upright with it. A torch carried through still air is in a wind: the air a moving fire meets is the air it moves through, so a walker's flame streams behind it, more at a jog, and settles upright over a fraction of a second when the walker stops. The same is true of a lantern swung on a pole, a brazier on a cart and a fire on a boat. The host would have to differentiate its own transforms and write a per-fire wind to get it, and the crate's one wind is a world resource, so it could not even then: this note adds the fire's own velocity to the wind each flame feels, with no host work.

## Sources

| source | kind | what it is | use |
|---|---|---|---|
| `10.7456_tojdac.1674124`, Doğaner, *The narrational role of lighting design in The Lights of the Dark City* (2025), Figure 2's caption and the "Piercing Light" section | craft | a film-lighting analysis that names a lamp's *motion* as the cue: "Pulsing light effect from the rhythmic motion of the ceiling lamp", and a character carrying a light through a scene | **the craft source**: a light that moves moves the scene's light and shadow with it, and it is read as moving only if its flame says so |
| `10.13140_marples.nav.lighting.thesis`, Marples 2017, *The influence of intrinsic perceptual cues on navigation and route selection in virtual environments*, around lines 3424 and 3990 | production | shipped games' use of a flame's motion as a cue, and a carried torch as a game's own light source (Alan Wake's) | **the production source**: "one of the more commonly used examples of this is the motion of flame, this can be in the form of lanterns, flaming barrels, or burning objects" |
| Welker & Sliepcevich (1966), *Bending of wind blown flames from liquid pools*, Fire Technology 2, with Pritchard & Binding's constants as printed in DGMR, *Technische methode voor het modelleren van plasbranden* (eqs. 7-9), and Wen §3.1 | technical | the tilt law `tan θ / cos θ = c · Fr^a · Re^b`, already `flame_tilt` (`docs/research/flame-wind.md`) | **the technical source**: the law's `U` is the speed of the air *past the fuel bed*; a bed carried through still air at `U` meets the same air as a stationary one in a wind of `U` |
| `docs/research/flame-wind.md`, `flame-lean-light.md`, `flame-gust.md` | this crate's own | the lean, the light that follows it, the wind that gusts | the parameters this note is consistent with |

## Craft principle

A cinematographer's moving practical is a light that carries its own motion: the swung lamp, the torch walked down a corridor, the lantern a character holds up as they turn. What tells the audience the light is moving is not the pool on the floor alone but the flame itself and the shadows that sweep with it. Doğaner's Figure 2 is exactly that cue: "Pulsing light effect from the rhythmic motion of the ceiling lamp." A lamp that swings and a flame that does not lean away from the swing reads as a lamp lit by a technician's bulb, not by a fire. The same paper's piercing light, "when a film character investigates a crime scene with a flashlight", is a light that travels with a body; the audience reads the body's pace in the light's pace.

## How shipping games do it

Marples names the mechanism, if not the lean: games lean on the flame as the cue that draws the eye and says a place is alive. "Games designers encourage both gaze direction and exploration by utilising elements of seemingly unimportant or natural motion to draw the player's gaze to the point of interest or in the desired direction of travel. A large variety of stimuli are used in this manner, one of the more commonly used examples of this is the motion of flame, this can be in the form of lanterns, flaming barrels, or burning objects." And of a carried light: "the torch the eponymous protagonist carries being required to make the 'darkness' able to take damage". A torch the player carries is the game's own light source, and a carried flame that stands rigid as the carrier runs is the tell of a prop. The corpus holds no production write-up of how a shipped game leans or streams a carried flame (see Pending index), so the lean law below is the physical one rather than a studio's number.

## Model adopted

A fire's flame leans downwind of the air *relative to the fire*. A flame whose site moves at ground velocity `v` through air moving at `w` meets `w - v`, so

`air = w - gain · v`

and the lean, `flame_tilt(|air|, diameter)` towards `air / |air|`, is the crate's existing law with the relative speed in place of the wind's. The carrier's walk is therefore `flame_tilt` at the walking pace, directed behind it.

**Where `v` comes from.** A component on every site, `FlameVelocity`, holds the site's last world position and its ground velocity, and one system (`track_flame_velocity`) differences the site's `GlobalTransform` each frame, on virtual time (the host's world moves on virtual time: a world at half speed is a walker at half speed, and a paused world has no velocity change to track). The raw velocity is smoothed by a first-order low pass with time constant `FlameMotion::response`, because the host's positions are not clean: a character moved in a fixed step and drawn at the display's rate shows frames with no step and frames with two (a noisy raw velocity), a swinging arm is a velocity that reverses each half second (the flame should sway with it, not buzz) and a flame's own lean has inertia. The step into the filter is rejected when it implies more than `MAX_TRACKED_SPEED`, which is a teleport (a respawn, an editor drag across the map), not a motion; a site's first frame only seeds its position.

**Who reads it.** `update_flames` takes the lean of every flame from the relative air of its site, and the pool's `lean_of` takes the shift of the light's position from the *same* relative air, so the light stands in the leaned flame as it does in a wind (`flame-lean-light.md`); the embers and the smoke read the lean from the flame's own instance scale and need no change. A site at rest (velocity exactly zero) takes the wind alone, which is bit-for-bit what it did before; with `FlameMotion::gain` zero every site does.

**What does not change.** The flicker, the kindle, the snuff and the pool's hand-off are untouched. The tongues' inward lean is summed with the (now relative) wind lean and capped as before.

## Parameters

| parameter | value | source |
|---|---|---|
| `FlameMotion::gain` | 1.0 | physics: the relative air is the wind less the fire's own velocity (Galilean). 0 turns the feature off; a host that wants a calmer carried flame (a torch bearer whose flame should read at the camera's angle at a walk, say) sets a fraction |
| `FlameMotion::response` | 0.15 s | design default: a little under one puff of a torch-sized fire (a 0.1 m bed puffs at `1.5 / √D` = 4.7 Hz, a period of 0.21 s, `flicker_rate`), slow enough to average a fixed-step character's 64 Hz against a 60 Hz display (about nine frames) and a physics jitter, quick enough that a start and a stop read at once; about 0.45 s from upright to 95 % of the lean |
| `MAX_TRACKED_SPEED` | 40 m/s | design default: above a galloping horse (about 20 m/s) and a cart; a step that implies more in one frame is a teleport |
| the smallest velocity kept | 0.01 m/s | design default: below it the filter snaps to zero, so a fire that has stopped is a still fire again and costs the tracker nothing (0.01 m/s is a lean of about 3° at `flame_tilt` for a 0.1 m torch, under a pixel's worth of lean at the courtyard's distances) |
| axis | the ground plane (XZ) | as `FlameWind::velocity`; a vertical motion (a torch raised or lowered) is not a lean |

## What great looks like

A torch-bearer crossing the courtyard: the flame streams behind the walker at an angle that grows with the pace (a stroll leans it about 35 to 45 degrees, a hurry puts it past 55), the pooled light and the sparks go with it, the walker's shadow of the torch rolls with the lean, and when the walker stops the flame stands up again over about half a second, not at once and not never. A lantern swung on a pole sways its flame against the swing, and its light with it. A brazier in a wind with a bearer walking into it leans more, and with the wind behind them less. A fire that stands still is the picture it was, to the pixel.

## Validation targets

- `smooth_velocity` (engine-free): a constant step converges to the true velocity (95 % in about `3 · response`), is frame-rate independent (`dt` of 1/30 and 1/120 reach the same value at the same time), holds at `dt = 0`, ignores a step over `MAX_TRACKED_SPEED`, snaps to exactly zero under 0.01 m/s, and a `NaN` in any input returns the previous velocity.
- `relative_air(wind, v, gain)`: `gain = 0` is the wind exactly; a headwind adds and a tailwind subtracts; the result is the wind when `v` is zero.
- Headless app: a site moved 1 m/s along +X for a second leans along -X by `flame_tilt(1, d)` within a few percent, the same site at rest leans by nothing; a site moving with the wind at the wind's speed does not lean; a paused world (a zero virtual step) holds the lean; a teleport leaves the lean as it was; `gain = 0` is the old transform scale.
- The pooled light of a moving candidate stands where `lean_light_shift` puts it for the relative air, and is hidden or absent exactly as before for a fire at rest.
- Capture: the courtyard against the build before, at the same seeds and views: a bearer at the same place and phase at the same frame has a flame leaning behind its heading, a bearer at rest and the braziers are zero pixels different. A frame sequence of one bearer starting and stopping (an example switch) shows the lean building and settling, not snapping.
- Cost (`bench_flames`, 5 000 torches): a fire at rest costs the tracker no more than about 0.02 µs a frame; a moving one a tilt law's two `powf`s.

## Deviations

- The lean is horizontal: the flame is a billboard whose lean channel is on the ground plane, as for a wind.
- The flame's *length* does not shorten with the relative speed, as with a wind (Wen's second half, `flame-wind.md`).
- A real torch run at a sprint is blown out; the crate caps the lean at `FLAME_TILT_LIMIT` (70 degrees) and never snuffs a fire for it (what puts a fire out is the host's `Snuff`).
- `FlameMotion` is one resource for every fire; a per-fire gain is not offered until a host asks.
- The relative air is the wind less the *site's* velocity: the air a carrier's own body drags along with it (a boundary layer at the hand) is not modelled.
- The lean of a site follows the translation of its `GlobalTransform` and ignores its rotation: a spun lantern's flame (on the axis of spin) does not lean, and a flame off the axis of a spinning arm leans with the velocity of its own point, which is the right answer.

## Pending index

- Welker & Sliepcevich 1966 and Pritchard & Binding 1992 remain the primary sources for the tilt law (`flame-wind.md`), not downloaded.
- No production write-up of a shipped game's carried-flame lean is in the corpus (`distill_search`, 2026-10-31, "character carrying a torch, flame streams behind when walking or running": Marples and unrelated hits only); `FlameMotion::gain` is the dial a studio's own number would set.
