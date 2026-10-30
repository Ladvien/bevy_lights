# A catching flame is taller than its share of the light

`flame-kindle.md` made a flame's drawn size follow its catch (`took`, 0..1, the same `kindle_gain` / `snuff_gain` the light's intensity is multiplied by) and wrote the size as `tag.y * took`: a straight line. Iterations 205 and 220 each noted the result and left it: through a catch or a snuff the flame is "a small tongue" or "a sliver" while its light is already most of the way up, because the light goes with the burn (`took`) and the drawn area goes with `took` squared. This note replaces the straight line with the law fire-science gives.

Scope: the drawn size of a `Flame` and of its ember quad. Not in scope: the colour ramp (`mix(EMBER_RED, colour, took)` stays on `took`), the light, the catch envelope, the flame's width relation to its fuel bed.

## Sources

| source | what it is | use |
|---|---|---|
| `10.3210_fst.26.1`, Delichatsios 2007, *A Correlation for the Flame Height in "Group" Fires*, Fire Science and Technology 26(1) | technical: flame height against heat release rate | the exponent: 2/3 in the intermediate regime |
| Heskestad, *Fire Plumes, Flame Height, and Air Entrainment* (SFPE Handbook; DOI `10.1007/978-1-4939-2565-0_13` found by `paper_search`, not in the corpus), restated in `docs/research/fire-kinds.md` as `L = 0.235·Q^0.4 − 1.02·D` | technical | the large-fire exponent, 2/5 |
| `docs/research/flame-kindle.md`, `docs/research/flame-fire-link.md` | this crate | `took` is the burn's share; the light's intensity is that same share |
| `10.1017_jfm.2018.707`, Xia & Zhang 2018 | technical (already in `flame-kindle.md`) | a smaller burning area gives a shorter flame |
| `10.7456_tojdac.1674124`, Doğaner, *The narrational role of lighting design in The Lights of the Dark City* | craft (already in `flame-relight.md`) | the visible source motivates the light |
| `s2011-advances-pdf-slides`, SIGGRAPH 2011 *Advances in Real-Time Rendering*, Fire / Flamethrowers / Thrusters | production (already in `flame-kindle.md`) | size is a lifetime channel driven by the fire's state |

## Craft principle

Doğaner, quoted in `flame-kindle.md`: "these objects determine the motivation of the light in the scene." The audience checks that motivation by eye, on the one frame sequence where both ends are on screen: a fire catching or going out. A light at 0.8 under a flame that has the drawn area of 0.4 reads as a light that is ahead of its fire.

## How shipping games do it

The 2011 *Advances* fire talk, quoted in `flame-kindle.md`: "Ramp the alpha to fade in/out over particle lifetime" and "shrink particles as they fade". A shipping flame's size is a curve authored against its brightness, not a straight line; the curve is the art, and a straight line is what an unedited ramp gives.

## Model adopted

The burn `Q` is what the light's intensity follows (`took`, the same function). Delichatsios, restating the intermediate-regime result of his handbook chapter: "the slope of a line through the data is 2/3 ... This power relationship ... implies ... the intermediate regime ... of the flame heights having a 2/3-power law dependence on the heat release rate." At a fixed bed, a flame's height therefore grows as `Q^(2/3)` while it is small; Heskestad's large-fire law is `Q^0.4`. A catch begins small, so the flame's drawn size is `took^(2/3)`:

| took (share of the burn) | old size | new size |
|---|---|---|
| 0.1 | 0.10 | 0.22 |
| 0.3 | 0.30 | 0.45 |
| 0.5 | 0.50 | 0.63 |
| 0.8 | 0.80 | 0.86 |
| 1.0 | 1.00 | 1.00 |

Both ends are unchanged (a settled flame is exactly as before; a snuffed one is still a point). The colour's `took` is left alone: the ember rim redness is a heat statement, not a size one. The ember quad takes the same size, so the sparks do not outgrow the flame they leave.

## Parameters

`CATCH_SIZE_EXPONENT = 2/3` in `src/flame.wesl` and `src/embers.wesl`. Design default for the *mapping* (a billboard's one drawn size stands for a flame's height and width together; Delichatsios's law is for height at a fixed bed), with the exponent from the source.

## Validation targets

- A settled flame (took = 1) is pixel-identical to the baseline.
- At took 0.3 and 0.5 the flame's bbox height is about 0.45 and 0.63 of the settled height (was 0.30 and 0.50), measured on the relight sequence.
- The light and the flame read together: no sequence frame where the ground is lit and no flame is drawn.

## Deviations

- The exponent is the intermediate-regime 2/3, not Heskestad's 0.4, because a catch spends its time at small `Q*`. Heskestad's paper is not in the corpus; Delichatsios's formulas did not survive PDF text extraction, the 2/3 slope is quoted from his prose.
- One scalar sizes width and height together; the source gives height only. Design default.

## What great looks like

Relight a brazier: the flame is a visible tongue as the light first returns and is full height as the light settles; on the way down the flame does not vanish while the ground is still lit.

## Pending index

Heskestad's SFPE chapter and Delichatsios's handbook chapter (`Combustion and Flame 70, 33-46`, 1987) are not in the corpus. `10.3210_fst.26.1` is downloaded; `scribe_convert` timed out on this pass, so it is not yet indexed.
