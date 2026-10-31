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
| `s2025-advances-materials-7` | *Fast as Hell: idTech 8 Global Illumination* (SIGGRAPH 2025) | "contact shadows for every light" in a shipped many-light renderer (`pool-template.md`) |
| `s2025-advances-materials-12` | Narkowicz & Costa, *MegaLights* (UE5, SIGGRAPH 2025) | screen traces and contact shadows for many lights |

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
| `10.3758_bf03200497` | Meng & Sedgwick, *Distance perception mediated through nested contact relations among surfaces* | a cast shadow tells resting from floating (`pool-template.md`); `catalog_read` returns 404 |

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

## Added by the 2026-10-26 re-scan (iteration 17)

`catalog_recent` (200 rows) found these in the library; none was in the lists above. All are read
live from the running index, not from memory.

| stem | what it is | use |
|---|---|---|
| `10.1007_978-1-4842-4427-2` | *Ray Tracing Gems* (Haines & Akenine-Möller 2019), ch. 18 | many-light importance sampling; the solid-angle form of a light's falloff, `dω = (n·−l)/‖X−Y‖² dA` |
| `PBR3_05_Color_and_Radiometry` | *Physically Based Rendering* 3rd ed., ch. 5 | irradiance `Φ/4πr²` and Lambert's law, stated from the source rather than remembered |
| `10.13140_marples.nav.lighting.thesis` | Marples 2017, PhD thesis, *The influence of intrinsic perceptual cues on navigation and route selection in virtual environments* | **the shipping-game source for light as guidance**: flame motion as an attractor, Left 4 Dead's lit junctions, Alan Wake's carried torch |
| `10.1109_cvpr52733.2024.00408` | *Holo-Relighting* (CVPR 2024) | volumetric portrait relighting from one image; an estimated environment, not a set practical |
| `10.1109_tvcg.2012.112` | *Rich intrinsic image decomposition of outdoor scenes from multiple views* | separating illumination from albedo; what a bake throws away |
| `10.24867_grid-2022-p95` | *The influence of the surrounding space on the lighting conditions in a photographic scene* | how a room's own surfaces change the light a source delivers |
| `10.2507_daaam.scibook.2008.39` | *Influence of the lighting on the portrait photographs shot with the digital photography system* | the key's placement against the subject's axis, measured |
| `10.24191_idealogy.v11i1.833` | *Identifying features in studio lighting tutorial videos for photography education* | how the craft is taught: source size, distance, angle as the three levers |
| `10.1145_3386569.3392390` | *Portrait shadow manipulation* (SIGGRAPH 2020) | a shadow as a designed object, not a by-product |
| `10.1145_3072959.3095816` | a SIGGRAPH 2017 paper (title not in the catalog) | unclassified; read before citing |
| `10.1080_10509674.2013.782936` | a 2013 *Digital Creativity* paper (title not in the catalog) | unclassified; read before citing |
| `10.1088_1755-1315_1099_1_012041` | *Light4Health eLearning course* | health research for interior lighting design: illuminance targets by task |
| `10.1145_3386496` | *Real-time illumination and visual coherence for photorealistic AR/MR* | the lighting-coherence problem a practical has when it is composited |
| `10.1007_s12144-023.04621-1`, `10.3389_fpsyg.2022.945669`, `10.48550_arxiv.2610.14334`, `10.48550_arxiv.2606.30317` | psychology and MCP-tooling papers | **not lighting**; listed so the next scan does not re-investigate them |
| `10.55677_ijhrsss_15-2026-vol03i06` | *Prompted props, human pipelines* (Begemann & Hutson 2026) | AI-generated props for game environments; relevant to asset pipelines, not to a light's geometry |

Two `EmbedSkip` rows in the scan (`10.5463_thesis.122`, `10.54612_a.3e1a1pjirr`) failed on
`gpu busy: 1842 MB free < 5000 MB required`; both are unrelated to lighting.

## Added by the 2026-10-30 re-scan (iteration 276)

`catalog_recent` (200 rows, 23:20-23:40 UTC) is a bulk import of a graphics and geometry-processing library plus unrelated
books and papers. Read live from the running index. Nothing in it is cinematography, photography or fire physics; two groups
are worth a row, the rest are listed so the next scan does not re-investigate them.

| stem | what it is | use |
|---|---|---|
| `PBR3_01_Introduction` to `PBR3_15_Light_Transport_III_Bidirectional_Methods` (15 chapters; `PBR3_05` and `PBR3_12` are listed above) | *Physically Based Rendering* 3rd ed. | the radiometry and light-source references stay `PBR3_05` and `PBR3_12`; `PBR3_04_Primitives_and_Intersection_Acceleration` is the spatial-structure reference for any future candidate culling |
| `Idiomatic Rust- Code like a Rustacean`, `Effective Rust- 35 Specific Ways to Improve Your Rust Code` | Rust idiom books (Anna's Archive imports) | not lighting; the reference for per-frame code that must not allocate or scan |
| `Evaluation-of-Reverse-Tone-Mapping-through-Varying-Exposure-Conditions` | already listed under *Perception and tone* | re-embedded, unchanged |
| the mesh, garment, motion-synthesis, retargeting and parameterization papers (`Laplacian-*`, `Instant-Field-Aligned-Meshes`, `Neural-Cages-*`, `MoDi-*`, `Frame-Fields-*`, `Cosserat-Rods-*` and their `_supp` files), the medical, psychology and relationship books and papers (`NEJM200009073431002`, `Fight Right-*`, `Legion A Novel*`) | geometry processing, animation, health | **not lighting** |

## Added by the 2026-10-30 re-scan (iteration 278)

`catalog_recent` (200 rows, 02:04-02:22 UTC on the 31st) is a re-embedding of the SIGGRAPH library (`sig2024_*`, `sig2026_*`, the `s2006`-`s2025` *Advances* courses) plus unrelated books and papers. Read from the on-disk markdown, not from memory. Nothing in it is cinematography, photography or practical-fire craft; the rows that bear on this crate are below, the rest is listed so the next scan does not re-investigate it.

| stem | what it is | use |
|---|---|---|
| `s2024-advances-pdf-2-6-mb` | *Seamless cluster rendering on mobile* (SIGGRAPH 2024 *Advances*) | clustered lights on a mobile budget: how many real lights a shipping renderer affords, for `LightPool`'s default sizes |
| `s2024-advances-pdf-48-mb` | *Hemispherical lighting insights* (SIGGRAPH 2024 *Advances*) | least-squares and spherical-harmonic encodings of irradiance: the technical reference when a practical's wash on a floor is compared with a probe's |
| `s2024-advances-pdf-7-mb` | Frostbite GIBS (SIGGRAPH 2024 *Advances*) | global illumination a game's lights are added to; context for the pool's bounce-free fill |
| `sig2024_Area_ReSTIR_Resampling_for_Real-Time_Defocus_and_Antialiasing`, `sig2026_Spatio-Temporal_Control_Variates_with_ReSTIR_for_Real-Time_Rendering` | resampled many-light direct illumination | the technical alternative to a budgeted pool (every light, sampled), as `s2025-advances-materials-9` and `-12` are |
| `sig2024_Scintilla_Simulating_Combustible_Vegetation_for_Wildfires` | fire spreading through vegetation | forest-fire spread is `bevy_flora`'s, not this crate's (`AGENTS.md`, ownership map); listed so it is not re-read |
| `s2024-advances-pdf-slides-notes-6mb` (Iwanicki, Activision), `s2024-advances-pdf-slides-notes-5mb`, `s2024-advances-pdf-12-5-mb`, `s2017-advances-pdf-slides-*` (clouds, planar reflections), the rest of `s2006`-`s2025-advances-*` | neural-network, terrain, cloud and reflection talks | **not lighting** (`torch` in the Iwanicki talk is PyTorch) |
| the `sig2024_*` and `sig2026_*` geometry, simulation, diffusion, cloth, knit, differentiable-rendering and optimisation papers; `transvoxel_*`, `tetgen_*`, `vowels2021`, `thompson2020`, the `schoeller_*` papers, `sexes-05-00047`, `slatkin_effective_python_3rd` | geometry processing, ML, medical and psychology | **not lighting** |

Three `EmbedSkip` rows (`ssrn_1601785`, `sm2_58`, `slatkin_effective_python_3rd`) failed on `503 ... myelin observe` and are unrelated to lighting.

## Added by the 2026-10-31 re-scan (iteration 279)

`catalog_recent` (200 rows, newest 05:01 UTC on the 31st) is the same bulk re-embedding of the SIGGRAPH library that iteration 278 scanned (`sig2024_*`, `sig2026_*`, the `s2007`-`s2025` *Advances* courses), plus three `EmbedSkip` rows that failed on `gpu busy: 1012 MB free < 5000 MB required` and an unrelated forestry paper. Every stem was already on disk. Nothing in it is cinematography, photography or fire physics. Two stems that bear on this crate were not listed before:

| stem | what it is | use |
|---|---|---|
| `s2008-advances-chapter-1` | Chen & Liu (Bungie), *Lighting and Material of Halo 3* (SIGGRAPH 2008 *Advances*) | a shipped game's lighting rig and its material model: production context for how a studio budgets the lights a scene carries |
| `s2025-advances-materials` (the opening, lines 850-920) | Tatarchuk, *Advances in Real-Time Rendering in Games, celebrating 20 years*: the list of games that shaped the field | names *It Takes Two* ("narrative co-operative gameplay with forced split screen") and *Split Fiction* ("expanded upon the forced split-screen co-op model"): evidence that a pool with one focus is not every shipped game's camera |

`book_real_time_rendering_4th_akenine` chapter 21 (virtual and augmented reality) is the technical reference for a frame drawn from more than one viewpoint at once ("each eye has a separate display"). The corpus has no document on lighting a scene for several cameras at once (searched: split-screen, multi-camera, per-view, local multiplayer); the craft source for that is on the web and is listed under `docs/research/pool-foci.md`'s `Pending index`.

## Added by the 2026-10-31 re-scan (iteration 281)

`catalog_recent` (200 rows, newest 07:21 UTC) is the same SIGGRAPH re-embedding iterations 278-280 scanned, plus unrelated clinical-psychology and software papers and the two candle-flame papers (`10.1038_s41598-018-36754-w`, Chen et al. 2019, *Frequency and phase characteristics of candle flame oscillation*; `10.1038_s41598-020-78229-x`, *Flickering candle flames and their collective behavior*), which are in the library and indexed but not yet in the on-disk corpus. They bear on the flicker's rate and phase, not on a new system. No `Research: corpus` commit beyond this note. Smoke (`flame-smoke.md`) read four stems that were already on disk and were not listed for it:

| stem | what it is | use |
|---|---|---|
| `s2011-advances-pdf-slides` (Fire/Flamethrowers/Thrusters, lines 1290-1462) | SIGGRAPH 2011 *Advances*, shipping fire | "Particle comp pass converts hotness to a blackbody color texture, adds it on top of smokey particles renderered with usual alpha blending": the additive heat layer over a smoke layer |
| `s2025-advances-materials-2` (lines 2333-2385) | SIGGRAPH 2025 *Advances*, the transparency (OIT) talk | why blended VFX smoke sorts badly: "manual sorting can result in a lot of side effects ... smoke tendrils look buggy" |
| `PBR3_11_Volume_Scattering` | *Physically Based Rendering* 3rd ed., ch. 11 | "fog and smoke attenuate and scatter light": smoke is lit by the light that reaches it |
| `10.13140_marples.nav.lighting.thesis` (9.5.3, lines 6152 and 8060-8206) | Marples 2017 (already listed above for navigation) | a torch sconce "with an animated flame and smoke effect" against a static glowing torch: a measured preference for the animated one (χ²(1, N = 76) = 19.000) |

Not in the corpus: Morton, Taylor & Turner 1956 on plumes (cited second-hand through `10.5194/npg-21-269-2014`, Matulka et al. 2014, found with `paper_search`; `paper_download` and `distill_index` pending) and Briggs' bent-over plume rise law (textbook, no source on disk).

## Added by the 2026-10-31 re-scan (iteration 283)

`catalog_recent` (200 rows, newest 09:07 UTC) is psychology re-embeds, the SIGGRAPH re-embed iterations 278-282 scanned and Matulka's plume paper (already listed): no new lighting stem. The wide-fire note found one source in the catalog by a semantic search (`distill_search`), not in the local mirror:

| stem | what it is | use |
|---|---|---|
| `10.3210_fst.26.1` | Delichatsios 2007, *A Correlation for the Flame Height in "Group" Fires* (Fire Sci. Technol. 26, 1) | flames of burners `S` apart merge above `Z_m = 3 S` (eqs. 6b and 7b, "L_f / S = 3 - 3.3"): the clamp on a side tongue's height in `docs/research/flame-tongues.md` |
| `10.13140_marples.nav.lighting.thesis` (lines 3426-3440) | Marples 2017 (already listed) | "the motion of flame, this can be in the form of lanterns, flaming barrels, or burning objects" draws the player's gaze: a flame is looked at |
| GPU Gems 1, ch. 6.2 (`GP/GPU Gems- Programming Techniques ...md`, lines 2925-2935) | Nguyen, *Fire in the "Vulcan" Demo* | "We started with a single animation, and it worked even better with two or more", a random difference in "size, position, rate of decay, animation start frame": a fire is several flames that differ |
