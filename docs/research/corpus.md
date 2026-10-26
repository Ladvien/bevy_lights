# The home-still corpus — what it holds for this crate

*Inventoried 2026-10-25.* This is what a live search of the home-still library returned for the
crate's domains, not a wish list. Stems are file names under `markdown/<shard>/`; every hit below
came back from `distill_search` (chunks) or `abstract_search` (one hit per paper) against the
running library, and each is a document that is actually there.

Domains with no hits list the seed titles to look for. Seed titles are titles only: a DOI is resolved
from the catalog when a paper is imported, never typed from memory.

## How to search it

1. Sync once (credentials and endpoint as in skill `ingest-dois-via-anna-scidb`):

   ```sh
   aws --endpoint-url "$EP" s3 sync s3://home-still/markdown/ ~/.cache/bevy_lights_corpus/markdown/
   ```

   The library was ~660 MB of markdown across ~9 700 documents on 2026-09-13; it is larger now.
2. Search by **method phrase**, not topic word. `grep -ril "key-to-fill" ~/.cache/bevy_lights_corpus/markdown`
   finds the lighting-craft paper; `grep -ril "light"` finds half the library.
3. Or search the live index: `distill_search {query, include_text: false}` for method phrases,
   `abstract_search` for a paper's abstract, `markdown_read {stem, page}` to read.
4. Read the hit as a file or a page; cite it in a note's `## Sources` by stem.

The corpus's book and course-note stems are not DOIs: SIGGRAPH *Advances* is `s<year>-advances-*`,
books are `book_*`, ETH CGL papers are `eth-cgl-*`, and a paper with no DOI keeps a title-ish stem.
Catalog titles are missing for about half the rows, so search the text.

## Cinematography

Searched `key light`, `fill ratio`, `motivated light`, `practical`, `negative fill`, `chiaroscuro`,
`lighting ratio`, `low-key`.

| stem | what it is | use |
|---|---|---|
| `10.4324_9781003671220` | *Cinematography Beyond Technique* | the vocabulary and the intent of a shot's lighting |
| `10.5897_jmcs2022.0788` | *Looking for low-key: noir and neo-noir from a cinematographic perspective* | what a low key-to-fill ratio does to a scene's read |
| `10.7456_tojdac.1674124` | *The narrational role of lighting design in The Lights of the Dark City* | light as a story device; a burst of light foreshadowing |
| `10.31866_2617-2674.6.2.2023.289311` | *Peculiarities of using lighting in the process of film shooting* | practical technique |
| `10.2478_bsmr-2022-0020` | *Ecological approach to cinematographic lighting of the human face — a pilot study* | what a face reads under each key direction |
| `10.51903_pixel.v13i2.1108` | *The effect of cinematic lighting on story emotions in 3D animation film* | the emotion mapping a game inherits |
| `10.58709_niujhu.v8i3.1694` | *A critical scrutiny of cinematic lighting technique in … Yoruba Nollywood films* | a second tradition's conventions |
| `10.2991_assehr.k.220404.037` | *A study of natural lighting and quality of lighting in production house for cinematography* | mixed natural/practical |
| `10.33153_texture.v8i1.6243` | *An analysis of cinematographic elements in the film Autobiography* | worked example |
| `10.59298_iaajc_2025_1113741` | *Film noir: social commentary through cinematic style* | the noir palette |
| `10.1080_19443927.2019.1643401` | *Examining the pedagogy of theatre lighting* | the stage tradition the film one grew from |
| `10.51415_10321_2502` | *Controlled shadow in photography: the development of a technique for child portraiture* | shadow as the subject |

## Photography

Searched `inverse square`, `family of angles`, `specular highlight`, `colour temperature`.

| stem | what it is | use |
|---|---|---|
| `10.66308_air.e2026053` | *The influence of the lighting scheme on the perception of a premium product in subject photography* | specular placement and the family of angles in practice |
| `10.48550_arxiv.2104.00618` | *Real-time global illumination using OpenGL and voxel cone tracing* | the inverse-square law's fast falloff and where a game fakes it |
| `book_real_time_rendering_4th_akenine` pp. 409, 431 | RTR4: analytic light shapes, the cube map | falloff and specular |

Nothing in the corpus on the inverse-square law as a *photographic* subject (Hunter, Biver & Fuqua,
*Light: Science and Magic*). Every practical's falloff in this crate is Bevy's windowed inverse
square, cited from `bevy_pbr`'s own `getRangeFalloff`.

## CG and animation lighting

Searched `light rig`, `rim light`, `shot lighting`, `illustrative lighting`.

| stem | what it is | use |
|---|---|---|
| `s2007-advances-pdf-slides-2-8-mb` | Mitchell, Francke & Eng 2007, *Illustrative rendering in Team Fortress 2* | the warm/cool half-Lambert ramp a practical sits in |
| `s2010-advances-pdf-slides-3` | SIGGRAPH 2010 *Advances* | production light rigs |
| `s2016-advances-slides-pdf-15-mb` | SIGGRAPH 2016 *Advances*: a lighting section | production practice |
| `s2025-advances-materials-3` | SIGGRAPH 2025 *Advances*, Ubisoft Montréal | current studio practice |

Nothing in the corpus on Birn's *Digital Lighting and Rendering* or Katatikarn & Tanzillo's
*Lighting for Animation*. Look for: **Birn, *Digital Lighting and Rendering*** (3rd ed., New Riders);
**Katatikarn & Tanzillo, *Lighting for Animation*** (CRC).

## Game lighting craft

Searched `lighting design`, `player attention`, `tension`, `GDC`, `Advances`, `immersion`.

| stem | what it is | use |
|---|---|---|
| `10.26503_dl.v2016i1.793` | *Integrating curiosity and uncertainty in game design* | attention as a design resource |
| `10.3389_frvir.2022.919163` | *Joyful adventures and frightening places — designing emotion-inducing virtual environments* | environment → emotion, with lighting among the levers |
| `10.1609_aiide.v11i1.12778` | *Targeting horror via level and soundscape generation* | light and dark as a mechanic |
| `fdg2014_fdg2014_wip_14` | *The effect of surroundings on gaming experience* | the room's light vs the game's |
| `fdg2014_fdg2014_paper_35` | FDG 2014, guiding players without taking agency | attention direction |
| `10.1109_vis4dh57440.2022.00007` | *Data feel: exploring visual effects in video games to support sensemaking* | VFX legibility |
| `10.26503_dl.v2026i4.3008` | *Neurodivergent pleasures in game environmental design* | who the lighting is for |
| `10.1007_978-3-642-33542-6_2` | *Gaming after dark — visual patterns and their significance for atmosphere and emotional experience* | what players read in a dark screen |
| `10.54254_2755-2721_2025.ld29491` | *Game scenes direct players' feelings and narrative experience through architectural language* | the set the lights hang on |
| `10.2218_ear.2024.8958` | *The architecture of the video game Stray* | a shipped night city's composition |
| `10.37679_trta.1142904` | *Sessiz oyunlar*: German expressionism in Inside, Little Nightmares, Monochroma | the low-light silhouette tradition |
| `10.14742_apubs.2017.733` | *Using virtual and augmented reality to study architectural lighting* | measuring what a viewer reads |

The published GDC and SIGGRAPH course talks are here as `s<year>-advances-*`; El-Nasr's game-lighting
papers (ELE, tension through lighting) are **not** in the corpus. Patry 2021, *Real-time samurai
cinema*, is `s2021-advances-pdf-66-mb` and does contain night-scene lighting practice (ll. 1883-2060
is its atmospheric lighting).

## Many-light rendering

Searched `clustered`, `tiled shading`, `light culling`, `many lights`, `z-binning`.

| stem | what it is | use |
|---|---|---|
| `book_real_time_rendering_4th_akenine` pp. 918-935 | RTR4, many-light management: z-binning, clustered shading | the survey and the numbers |
| `s2025-advances-materials-9` | *Stochastic tile-based lighting* (SIGGRAPH 2025) | the current research alternative: sample and denoise |
| `s2020-advances-pdf-slides` | SIGGRAPH 2020 *Advances*: improved culling for tiled and clustered rendering | per-tile light counts at silhouettes |
| `s2025-advances-materials` | SIGGRAPH 2025 *Advances*: clustered shading from Persson & Olsson on | the lineage |
| `s2011-advances-pdf-slides-2` | SIGGRAPH 2011 *Advances*, tiled forward | the original tile-based pipeline |
| `10.1007_978-3-319-24589-8_3` | Rodrigues & Clua 2015, real-time lighting for procedural 2D isometric game terrain | the same budget problem at a smaller scale |
| `book_level_of_detail_3d_graphics_luebke` ch. ? | Luebke et al., *Level of Detail for 3D Graphics* | the perceptual budget argument |

Olsson, Billeter & Assarsson 2012, *Clustered deferred and forward shading* (HPG), **is not in the
corpus**; Sanctuary cites it by DOI `10.1145/2343045.2343095`, and `paper_get` on that DOI returns
nothing. Look for it, or read Patry 2021 and the 2025 *Advances* note in its place.

## Fire and flicker physics

Searched `puffing frequency`, `flame oscillation`, `flicker`, `candle`, `buoyant plume`, `pool fire`.

**Filled 2026-10-26.** `10.1017_jfm.2018.707` (Xia & Zhang 2018, *J. Fluid Mech.*, open access) is
in the library and indexed: it states the `f ∝ D^{-1/2}` law with the factor 1.5 and fits
`f_p = 0.48 √(g/D)` to the pool-fire data of six earlier studies, so it carries Cetegen & Ahmed 1993's
result as well as its own vortex-shedding derivation. `docs/research/fire-light.md` adopts the law.
Still missing: the primary Cetegen & Ahmed 1993 (not open access; `paper_download` found no copy) and
Kitahata et al. 2005 on candle oscillation.

**Nothing else in the corpus as of 2026-10-25.** The combustion hits are biomass burning and atmospheric
chemistry (`10.5194_acp-11-6445-2011`, boreal forest fire emissions), not flame dynamics. Every
fire-light rate in this crate is therefore a **design default** inherited from Sanctuary, and
`docs/research/fire-light.md` says so at each one.

Look for:

- Cetegen & Ahmed 1993, *Periodic instability of buoyant plumes and pool fires*, Combustion and
  Flame 93(1-2) — the puffing frequency against pool diameter.
- Kitahata, K. et al. 2005, *Oscillation and synchronization in the combustion of candles*,
  J. Phys. Chem. A 109 — the 1-2 Hz range, and two candles synchronising.
- Malalasekera, Versteeg & Gilchrist 1996, *A review of research and an experimental study on the
  pulsation of buoyant diffusion flames and pool fires*, Fire and Materials.
- Hamins, Yang & Kashiwagi 1992, on the heat-release-rate dependence of the puffing frequency.

This is the crate's first backlog item: a fire-flicker model with a size parameter, replacing the
doubled sine.

## Perception and tone

Searched `Purkinje`, `scotopic`, `tone mapping`, `exposure`, `night vision`.

| stem | what it is | use |
|---|---|---|
| `s2021-advances-pdf-66-mb` p. 160 | *Real-time samurai cinema*: rod contributions to colour perception (Cao et al. 2008) | why a night scene's colours shift |
| `book_fundamentals_computer_graphics_marschner` | *Fundamentals of Computer Graphics*: photopic vs scotopic viewing, colour constancy | the perceptual frame |
| `Understanding-Exposure-for-Reverse-Tone-Mapping` | *Understanding exposure for reverse tone mapping* | what an exposure means |
| `Evaluation-of-Reverse-Tone-Mapping-through-Varying-Exposure-Conditions` | the same, measured | |
| `10.1109_tvcg.2019.2898741` | Denes et al. 2019, *Temporal resolution multiplexing* | critical flicker fusion: how fast a flicker must be to fuse |
| `book_real_time_rendering_4th_akenine` ch. 8 | RTR4, light and colour | the survey |

A practical's flicker at 1-2 Hz is *well below* critical flicker fusion, which is why it reads as
movement and not as a strobe — and why an amplitude above ~20 % starts to hurt.

## Lighting look (added with `lighting-look.md`, 2026-10-26)

Read live for iteration 1's note; all of these are in the library and indexed.

| stem | what it is | use |
|---|---|---|
| `10.7456_tojdac.1674124` | Doğaner, the narrational role of lighting design in *The Lights of the Dark City* | motivated sources; hard vs soft as narrative |
| `10.2478_bsmr-2022-0020` | Huttunen, ecological approach to cinematographic lighting of the face | light direction conventions; underlighting |
| `10.32657_10220_48312` | Strategies of colour and lighting design in the cinematography of Christopher Doyle | chiaroscuro, chromatic zoning, specular placement |
| `10.23856_6105` | *Harmony of light and shadow* | source size : distance softness table; inverse square |
| `s2007-advances-course-notes-0-4-mb` | Mitchell, Francke & Eng 2007, TF2, the full course note | the cool-to-warm ramp and terminator reddening, quoted |
| `s2006-advances-full-course-notes-pdf` | SIGGRAPH 2006 *Advances*, the dark stormy city | a night built from many apparent sources |
| `s2011-advances-pdf-slides` | SIGGRAPH 2011 *Advances*, the fire talk | hotness → blackbody colour |
| `PBR3_12_Light_Sources` | *Physically Based Rendering* 3rd ed., ch. 12 | blackbody, colour temperature, warm/cool bands |
| `10.26503_dl.v2005i1.147` | *Shadowplay: simulated illumination in game worlds* | the game-side history of lighting as a read |
| `10.1525_fq.2004.57.3.24` | *The emergence of filmic artifacts* | what a digital camera's night costs, for the exposure gap |
