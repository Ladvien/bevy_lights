# A pooled fire's life: Kindle and Snuff on a pool candidate

`Kindle` (iteration `flame-kindle.md`) and `Snuff` (`flame-snuff.md`) act on a `Flicker` light the host owns. A fire that
lives in the **pool** — a torch in a bearer's hand, one of thousands of virtual braziers — has no light of its own: the
pool lends it one. Today a host that douses such a torch has only two moves, both wrong: remove `PoolCandidate` (the
slot hands off at once, so the light cross-fades over `fade_seconds` while the flame drawn on it, with nothing to link to,
stays at full size) or despawn it. The flame and the light disagree for as long as the fade runs, which is the one
disagreement `flame-snuff.md` says the viewer can check.

## Sources

| source | what it is | use |
|---|---|---|
| `10.7456_tojdac.1674124` — Doğaner, *The narrational role of lighting design in The Lights of the Dark City* (2025) | craft: a visible practical motivates the light | the drawn flame and the light it motivates must go out as one |
| `s2011-advances-pdf-slides` — SIGGRAPH 2011 *Advances in Real-Time Rendering in Games*, Fire/Flamethrowers/Thrusters | shipping production fire | a fire's end is a cross-fade of size and emission, not a cut |
| `10.1038_s41598-020-78229-x` — Gergely et al. (2022) | technical: candle flame state | a starved flame quiets before it goes out; the amplitude of the swing is not the thing that changes |
| `docs/research/pool-hand-off.md` (Brown, *Cinematography* overlap quote) | craft: constant intensity across a move | a fire's change of state must not be confused with a hand-off |

## Craft principle

> "A table lamp or street lantern on a set is not usually the main source of light used by the technical crew, as these
> sources may be inadequate or uncontrolled. However, these objects determine the motivation of the light in the scene."
> (`10.7456_tojdac.1674124`)

The practical motivates the light, so the two change together. Which real light happens to be lent to the practical is
an implementation detail the audience never sees; a torch that is going out must dim whether or not the pool lent it a
light this frame.

## How shipping games do it

> "Crossfade to a smokey particle over the lifetime." (`s2011-advances-pdf-slides`)

The fire's end is a fade on the fire's own lifetime. A pooled light is the *rendering* of that fire, so the state belongs
to the fire (the prop), not to the slot: a slot that changes owner mid-snuff must still show the owner's fade.

## Model adopted

`Kindle` and `Snuff` are legal on an entity carrying `PoolCandidate` (and no `Flicker`). The crate advances them on
virtual time, writes `Kindled` / `Snuffed` for the candidate's entity, and the pool multiplies the candidate's
`intensity` by `kindle_gain(elapsed) · snuff_gain(elapsed)` — the product `flicker_lights` writes for a light of its
own. The candidate's `FlameSite::fire` names it and the flame follows through the existing link, so no flame code
changes. A candidate whose `Snuff` has finished (`elapsed >= SNUFF_SECONDS`) **no longer wants a light**: it leaves the
pool's list, its slot is freed for the next candidate and the cross-fade of its departing light starts from 0, so the
slot is never held by a dead fire.

The **rank** uses the candidate's un-scaled `intensity`: a fire mid-snuff keeps its slot rather than being handed off
while it dims (a hand-off layered on a fade is two changes at once), and a fire mid-kindle is judged by what it will
be. Re-lighting is the same as for a light: remove `Snuff`, insert `Kindle::default()`.

## What great looks like

A bearer's torch doused at the edge of the pool: the light on the stone and the flame in its hand fall together over a
second, no slot hand-off dip, the freed slot taken by the next-brightest torch with its own fade-in.

## Parameters

None new. `KINDLE_SECONDS` and `SNUFF_SECONDS` are those of `flame-kindle.md` and `flame-snuff.md`; the product rule
matches `flicker_lights`.

## Validation targets

- A pooled candidate with `Snuff` at `elapsed = t` lends a light of `intensity · snuff_gain(t)`; at the end exactly 0
  and `Snuffed { candidate }` is written once.
- A finished snuff frees its slot to the next candidate; `PoolBudget::wanting` drops by one.
- A pooled candidate with `Kindle` lends `intensity · kindle_gain(t)` and `Kindled` is written once.
- A host that never uses either pays nothing: no new column on the candidate scan (the advance system matches only
  entities that carry `Kindle` or `Snuff`).

## Deviations

None to a default.

## Pending index

None.

## Iteration 276's findings (issue #11)

### Sources

| source | what it is | use |
|---|---|---|
| `10.26503_dl.v2005i1.147` — *Shadowplay: simulated illumination in game worlds* (the Silent Hill 2 analysis) | craft: lighting as a scripted time of day | a scene whose practicals are mostly out is a *designed* state, not an edge case |
| `s2020-advances-pdf-slides` — SIGGRAPH 2020 *Advances*, id Tech 7 (Doom Eternal) | shipping production: why a CPU-side light scan became the bill | the per-light cost a host's artists can multiply is the cost that matters |
| `book_real_time_rendering_4th_akenine`, ch. 20 (*Efficient Shading*), the light-culling summary | technical: a culling step that is paid whether or not it helps | a per-candidate step must cost no more in the quiet state than in the busy one |
| `bevy_ecs-0.20.0/src/query/fetch.rs:3249-3264` (`Option<T>` `set_archetype` / `set_table`) | the engine: an optional column is matched once per table | the cost of reading `Option<&Snuff>` on a candidate that has none |

### Craft principle

> "A grey dawn permeates the final stage of the game at the Lake Side hotel; thus "Silent Hill 2" is played out over one day."
> (`10.26503_dl.v2005i1.147`)

A designer scripts the hours of a scene, and the hour a torch is put out is the one the player reads as *morning*. The crate
gives a host exactly that move (`Snuff` on a pooled candidate) and keeps the fire out until the host removes it, so a town
at dawn, or a saved scene whose torches were put out, holds thousands of candidates that each carry a finished `Snuff`.
That is the state the pool meets most often in a day-night game, and the one where nothing is lit: it must cost no more than
the night did.

### How shipping games do it

> "Used Clustered Binning in Doom [Olson12]" / "Wanted to move to GPU culling to reduce CPU load" / "Artists wanted to place
> more lights and decals" (`s2020-advances-pdf-slides`, id Tech 7)

Artists raise the light count until the CPU-side scan over them is the bill, and the studio's answer was to make the
per-light step a constant, not to ask for fewer lights. This crate's scan is the CPU one: `assign_pool` visits every
candidate (5000 in the bench, "5000 entries a frame" in `src/pool.rs`), so a step that is O(fires out) *per candidate* is
the quadratic the studio would have had to remove.

### Technical

> "Just as frustum culling will cost additional time with no benefit if all objects are always in view, so will some
> techniques provide little benefit under various conditions." (RTR4, ch. 20)

The same holds for a reject test: it must be the cheap half of the filter. `assign_pool` collected every candidate whose
`Snuff` had finished into a `Vec<Entity>` and ran `Vec::contains` on it for **every** candidate, before the reach filter. That
is N·|out| entity comparisons a frame: with all 5000 out, N(N+1)/2 = 12.5 million.

**Measured** on big (load average 51, so compare ratios): the bench at 5000 candidates, 2000 lights, pool 16, three
interleaved rounds of the unfixed code with the new `BENCH_SNUFFED` switch. `pool_p50_us`: 426 / 1448 / 435 µs with no fire
out; 5051 / 4465 / 4832 µs with 2500 out; 11338 / 7685 / 6771 µs with all 5000 out. Whole frame `min_us`: 430 / 407 / 430 µs
against 5590 / 5652 / 5586 µs, 13 times. The pool's own frame is the 0.5 ms the notes budget; a dawn made it 5 to 11 ms.

### Model adopted

The candidate scan reads the fire's state where it reads the candidate: `CandidateRead` gains `Option<&'static Snuff>`, and the
filter drops a candidate whose `Snuff` has `elapsed >= SNUFF_SECONDS`. The `snuffs` query and the `out: Vec<Entity>` go. The
semantics are unchanged (a candidate with a `Snuff` in progress keeps its slot, rank still uses the plain `intensity`, a
finished one wants nothing); only the lookup changes, from a search over a list to a read of the row being visited.

Why not `fires.get(entity)` (the issue's other suggestion): it is a random access per candidate (an entity-location lookup,
then the table row), 5000 of them a frame for the sake of the rare fire that is out. `Option<&Snuff>` is decided once per
*table* (`set_table` sets `matches`, `fetch` returns `None` for a row of a table without the column), so a world with no
`Snuff` anywhere pays one boolean per row and one with every fire out pays one pointer read.

### Validation targets

- `BENCH_SNUFFED=5000` `pool_p50_us` within 1.25x of `BENCH_SNUFFED=0` (the issue's acceptance), same machine, interleaved.
- `BENCH_SNUFFED=0` unchanged from the unfixed binary, within the noise of the same interleaving.
- `a_pooled_fire_that_is_snuffed_dims_its_light_to_nothing_and_frees_the_slot` passes unchanged; a fire loaded already out
  never holds a light and is not announced.

### Deviations

The target above under *Validation targets* ("no new column on the candidate scan") is kept in its intent and not its letter:
the column is new, and it is the table-level `Option` that makes it free for a host that never uses `Snuff` (the first
bullet of this section's targets measures that). No default changes.

### Pending index

None. Home-still was up (distill on Cuda, scribe on both hosts) on 2026-10-30; the four sources were read from the local
corpus and the vendored Bevy source.
