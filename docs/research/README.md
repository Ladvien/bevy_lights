# Research notes

One note per system, committed before its code. Each has the headings `AGENTS.md` requires:
`## Sources`, `## Craft principle`, `## How shipping games do it`, `## Model adopted`,
`## Parameters`, `## Validation targets`, `## Deviations`, `## Pending index`.

A default changes only with a cited source recorded under `Deviations`, or with the note saying
plainly that it is a design default.

- [`fire-light.md`](fire-light.md) — the flicker and the kindle: how a fire's light moves, and how it
  takes hold.
- [`light-pool.md`](light-pool.md) — the budgeted pool of real lights, and why the hand-off must not
  pop.
- [`flame.md`](flame.md) — the billboarded shader flame: one material, one draw, a tag per fire.
- [`flame-flicker.md`](flame-flicker.md) — the drawn flame and its light as one fire: brightness on the
  light's clock, the silhouette on real time.
- [`flicker-depth.md`](flicker-depth.md) — how far a fire dips, who chooses, and a measured ceiling the
  default sits well inside.
- [`flame-silhouette.md`](flame-silhouette.md) — the flame's edges, the puff law that would shape them,
  and the per-flame diameter that blocks it.
- [`flame-culling.md`](flame-culling.md) — a fire off the screen costs almost nothing: the box a billboard
  needs, why the instance scale must be divided out of it, and what 20 000 flames cost before and after.
- [`flame-kindle.md`](flame-kindle.md) — a lit flame must *grow* with the light it motivates, and the
  scale that was written but never drawn.
- [`carried-fire.md`](carried-fire.md) — where a hand-held fire's light belongs, and the lumens it needs
  from there.
- [`lighting-look.md`](lighting-look.md) — the composition: key against fill, a light's colour from its
  temperature, and the lumens a fire needs for a pool of a given size.
- [`pool-hand-off.md`](pool-hand-off.md) — the pool's hand-off: hysteresis (the overlap rule) and a
  fade, so a light that changes owner does not pop.
- [`pool-template.md`](pool-template.md) — every setting of the pool's template on every pool light,
  live and saved; Bevy's soft-shadow and contact-shadow levers, measured.
- [`pool-foci.md`](pool-foci.md) — the pool for more than one viewer: a shared budget with one focus per
  viewer, why a plain nearest-focus distance starves the sparse viewer, and the light a moving camera
  turns to.
- [`fire-colour.md`](fire-colour.md) — a fire's own colour on its flame, its sparks and the pooled light
  it is handed: one hue for the visible source and the light it throws, as the same ramp rotated.
- [`corpus.md`](corpus.md) — what the home-still library actually holds for cinematography,
  photography, CG and animation lighting, game-lighting craft, many-light rendering, fire physics and
  perception, with the stems to read and the gaps to fill.
