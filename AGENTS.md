# bevy_lights — notes for agents

A Bevy 0.20 library for the **practical lights** a scene's props actually carry: fires that flicker
and flare as they kindle, a budgeted pool of real lights handed to the nearest props over thousands
of virtual ones, and billboarded shader flames. Small on purpose: three plugins, three public
components' worth of state, an engine-free `src/model/` and one shader.

## Force pushes and deletions need the user's express permission

Never do any of the following unless the user has approved that specific action, by name, in the current conversation. A loop brief, a skill, a standing instruction such as "commit, push, PR, merge", or an earlier session's approval is not permission.

- Force-push in any form: `git push --force`, `--force-with-lease`, `--force-if-includes`, a `+` refspec, or pushing rewritten history. Deleting a remote branch or tag (`git push --delete`, `gh pr merge --delete-branch`, an API delete).
- Throwing away commits or uncommitted work: `git reset --hard`, `--merge` or `--keep`; `git checkout -- <path>` or `git restore` over changes you did not make; `git clean`; `git stash drop` or `clear`; `git branch -D`; `git worktree remove --force`; `git rebase` or `git commit --amend` on pushed commits; `git filter-repo` or `filter-branch`.
- Deleting files, directories, build caches, target dirs, models, datasets, database rows, issues, PRs, releases or cloud resources that you did not create in this session (`rm`, `rm -rf`, `git rm`, `cargo clean`, an API delete).

When one of these seems necessary, stop and ask. Name the exact command, what it destroys and whose work it touches. A commit that landed on the wrong branch is fixed with a new commit or a question, never by resetting a shared working tree or force-pushing. Files you created yourself in this session (scratch scripts, temp dirs, your own worktrees) you may delete.

## Source of truth

This repository, [`Ladvien/bevy_lights`](https://github.com/Ladvien/bevy_lights), is canonical. MIT OR Apache-2.0. Sanctuary (`/Users/ladvien/sanctuary`) consumes it **by crates.io version**. A fix or a feature lands here first, in its own commit with tests, and is never patched into the game. The ladvien dependencies are crates.io versions, and the family moves together.

## Version pin

Pinned to **Bevy 0.20.0**. Read the vendored source (`~/.cargo/registry/src/index.crates.io-*/bevy-0.20.0/`, its `examples/` and `_release-content/migration-guides/`), not bevy.org, which documents `main` and has been wrong for a pin more than once. MSRV is 1.97.1 (Bevy 0.20's). On big_mac the default toolchain is older: run every command as `cargo +1.98.1 …`.

## Layout

- `src/lib.rs` — the crate docs, the flat re-exports (no prelude), and `LightsSystems`.
- `src/model/mod.rs` — the engine-free half.
- `src/model/fire.rs` — `flicker_factor`, `kindle_gain`, `KINDLE_SECONDS`.
- `src/model/reach.rs` — `raised_light`, `RANGE_MARGIN`.
- `src/model/tag.rs` — `pack_flame_tag`, `unpack_flame_tag`.
- `src/model/fire_colour.rs` — `retint` and the shipped ramp's stops: a fire's own colour, as the orange ramp moved to its hue (`docs/research/fire-colour.md`).
- `src/flicker.rs` — `FlickerPlugin`, `Flicker`, `Kindle`, `Kindled`, and `flicker_lights` over both light types.
- `src/pool.rs` — `LightPoolPlugin<F>`, `LightPool`, `FocusRule`, `PoolCandidate`, `PoolLight`, `assign_pool::<F>`. `src/model/pool.rs` holds the hand-off rule and the shared pool's maths (`focus_quota`, `quota_cutoff`, `shared_value`).
- `src/flame.rs` — `FlamePlugin`, `FlameMaterial`, `FlameLook`, `FlameSite`, `Flame`, `dress_flames`, `update_flames`.
- `src/flame.wesl` — the flame shader, embedded with `embedded_asset!`.
- `src/embers.rs`, `src/embers.wesl` — `FlameEmbers` and the spark quad (a child of the `Flame`, registered by `FlamePlugin`).
- `src/smoke.rs`, `src/smoke.wesl` — `FlameSmoke` and the smoke quad (a child of the `Flame`, the spark quad's geometry and culling box, additive only), and the trail that quad becomes when its fire is put out (`SmokeTail`, `begin_smoke_tails`, `update_smoke_tails`; `docs/research/flame-smoke-tail.md`).
- `examples/common/mod.rs` — the `LIGHTS_*` run switches and the fly camera.
- `examples/courtyard.rs` — the showcase and capture scene: four braziers, 48 torch-bearers, an orbit focus.
- `examples/bench_lights.rs` — the CPU bench, no window and no GPU.
- `examples/bench_flames.rs` — what thousands of flames cost when few are on screen: headless, off-screen, needs a GPU adapter.
- `tests/leaf.rs` — the boundary ratchet: the dependency list, forbidden names, engine-free `src/model/`.
- `tests/flicker.rs`, `tests/pool.rs` — the two plugins' consumer-visible behaviour.
- `tools/wesl-check/` — compiles `src/flame.wesl` to WGSL and parses it with naga, without a GPU.
- `docs/research/*.md` — one note per system, plus `corpus.md`.
- `docs/loop-log.md` — what each improvement-loop iteration did.
- `.omp/commands/build-loop.md` — the loop (`/build-loop`): one command file, no separate skill.
- `scripts/bevy-build-loop.sh` — the loop's headless driver.
- `.github/workflows/ci.yml` — `lint`, `build`, `wasm`, on big's self-hosted runners.

## Build and test

```sh
cargo build --release
cargo build
cargo build --features serde
cargo test
cargo test --features serde
cargo build --examples --all-features
cargo clippy --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
cargo fmt --check
cargo build --lib --target wasm32-unknown-unknown
cargo run --release --manifest-path tools/wesl-check/Cargo.toml -- src
```

Use `CARGO_TARGET_DIR=$HOME/bevy_lights/target` on mac_air: the shared
`~/.cargo-target-shared/global` is 85 GB and the boot volume has crossed 98 % full, at which point
builds fail with `No space left on device` in the middle of a link and read as a code error.

`cargo build` (bare) comes before `cargo test` deliberately: `cargo test` compiles the full `bevy`
dev-dependency, and Cargo unifies features across a build, so under `cargo test` the crate silently
receives every Bevy feature and a hole in its own trimmed feature list cannot fail. A plain
`cargo build` compiles what a consumer actually gets.

## Research first

No system is implemented without its `docs/research/<system>.md`, committed first. A lighting default
is an art decision, so it needs the same treatment as a physics constant.

1. **Search the corpus first.** `docs/research/corpus.md` lists what the home-still library holds per
   domain (cinematography, photography, CG and animation lighting, game-lighting craft, many-light
   rendering, fire physics, perception) with the stems to read. Search by method phrase, not topic
   word.
2. Preflight with `system_status` (MCP) or `hs status`. If distill is down, research continues without
   `distill_search` and `distill_index` is skipped and listed under the note's `Pending index`.
3. Fill gaps with `paper_search` and `paper_download`; `scribe_convert` only when scribe is healthy.
   Read with `markdown_read {stem, page}`.
4. Write the note with the headings: `## Sources`, `## Craft principle` (what a cinematographer,
   gaffer or photographer does and why), `## How shipping games do it`, `## Model adopted`,
   `## Parameters`, `## Validation targets`, `## Deviations`, `## Pending index`.
5. **Every default cites a source or says "design default".** A default changes only in a commit that
   also changes its note and the CHANGELOG.
6. **DOIs come from the catalog and are never typed from memory.** A DOI in a note that the catalog
   does not know is a bug in the note.

## Plays well with the family

1. **Bevy pin parity.** Depend only on crates on Bevy 0.20. Couple to 0.19 crates (`bevy_light_grid`,
   `bevy_carnage`'s 0.19 siblings) only through plain data in game glue, documented in the README's
   `Plays well with` section.
2. **Ownership map.** The sun, the moon, the sky, exposure, weather and the lightning flash belong to
   `bevy_elements`; this crate **never meters exposure and never adds a `DirectionalLight`**. Gameplay
   illuminance grids belong to `bevy_light_grid`. Trees and forest-fire spread belong to `bevy_flora`,
   and who lights a lamp and when belongs to `bevy_society` or the game. This crate owns practical
   lights: flicker, kindle, the pool, the flames and the placement helpers.
3. **Hooks over dependencies.** An integration with a sibling is an optional feature named after it
   (`elements`, `flora`, `society`) that reads the sibling's public resources. It never makes a
   sibling depend on this crate.
4. **A needed sibling change** lands in the sibling's repo as an issue or PR. For `bevy_elements` that
   is always an issue (`gh issue create -R Ladvien/bevy_elements`), never a patch.
5. **Units.** Intensities are Bevy's: lumens for a point or spot light. Distances are the host's world
   units, and this crate converts nothing. Interop with `bevy_elements`' SI-metre
   `light_intensity`/`display_value` goes through `bevy_elements::ElementsUnits`, in the host.
6. **No game vocabulary in `src/`**, enforced by `tests/leaf.rs`.
7. **Everything public is `Reflect`**, so `bevy_fairy` (BRP) can read and drive it.

## The non-negotiables

- **No `unwrap()`, `expect()` or `panic!` outside tests.** Misconfiguration gets a loud
  `error!`/`warn_once!`, never a panic: `LightPoolPlugin::finish` reports a missing `FlickerPlugin`
  rather than adding it (a host that added it afterwards would panic on the duplicate).
- **One path per feature.** No fallbacks, shims or stubs.
- **The host owns its lights' lifetimes; the crate owns only the pool's.** A pool light is spawned and
  despawned by the crate and never by the host.
- **The pool is deterministic**: sorted by distance, ties broken on `Entity`. Same inputs, same
  assignment, on every machine and every frame.
- **`src/model/` is engine-free.** No `bevy` in it; `tests/leaf.rs` enforces it.
- **Flicker runs on virtual `Time` and flames on `Time<Real>`.** A paused world's fires hold still
  while the flames drawn on them keep moving. That split is deliberate.
- **`max_height` is set once**, before any `FlameSite` spawns: a site's height is packed as a fraction
  of it, and a tag already packed cannot be re-scaled.

## Rules

- **A missing `Res<T>` panics its system.** Take `Option<Res<T>>`, or have the plugin `init_resource`
  it. Settings are `Resource + Default + Clone + Debug + Reflect` added with `init_resource`, so a
  caller who inserts one first overrides it.
- **All run conditions are evaluated — there is no short-circuit.** A bare `Res<T>` in a
  `.run_if(..)` closure panics whenever that resource is absent.
- **`Single<..>` silently skips its system on a non-unique match.** The pool's focus is a plain
  `Query` and counts its matches instead, because two foci is a *reportable* state under the default
  `FocusRule::One` and `Single` would hide it. `FocusRule::Shared` takes any number, one per viewer, and
  shares `LightPool::size` between them (`docs/research/pool-foci.md`): `size` is the total, never per view.
- **Lifecycle observers take the event:** `On<Add<A>>`, `On<Remove<A>>`.
- Do not hard-wrap markdown.
- Close any Bevy app you launch.
- Every dependency gets a GitHub link comment in `Cargo.toml` and a row in README's `Built on` table.
- **The README's quick start is compiled by a test.** `tests/readme.rs` extracts the `## Quick start` block,
  writes it to `examples/readme_quick_start.rs` (behind the `readme-example` feature, which nothing else
  enables), formats it and builds it — so a field added to a component cannot leave the README stale. It did
  once: `FlameSite::kindle` and `Flicker::diameter` were both missing from it for four iterations.
- **The README is kept current with every commit, push and publish.** Its hero GIF
  (`docs/media/showcase.gif`) is how someone decides to try the crate: two seconds, 30 fps, under ~2 MB, and
  it must show the top features the crate actually has (the catch, the flicker at its own rate, a pool
  hand-off). Re-encode it whenever a change alters what it shows; `docs/DEMOS.md` has the recipe and the two
  encoding traps (the encoded `FPS` must match the capture's content rate, and `dither=bayer` crawls on a
  dark scene).
- Changing a default changes its research note and the CHANGELOG in the same commit.
- **Judge images yourself as a hostile art director:** critique first (file, region, severity), then
  the verdict GREAT / NOT YET / FAIL. "Fine", "acceptable" and "good" are NOT YET.

## Bevy 0.20 traps found here

- **`meta`, `active`, `patch`, `target` and `filter` are reserved WGSL words.** A struct field or a
  local named any of them fails the shader at *runtime* with "use of a reserved word", not at build
  time, and the surface renders dark.
- **A WESL error is silent at build time and loud only in a capture.** The shader is embedded with
  `embedded_asset!`, so a struct built with the wrong number of fields, an unimported name, a
  shadowing local or a `pow(x, p)` with `p` in a `let` all *build*, are embedded, and then fail in the
  GPU driver: the flame renders as nothing and the only clue is a `failed to process shader` line in
  the run's log, which a capture script throws away. **Check the shader before capturing:**
  ```sh
  cargo run --release --manifest-path tools/wesl-check/Cargo.toml -- src
  ```
- **The WESL validator's stubs must be mounted twice.** A Bevy import inside a module of this crate's
  package arrives as `PathOrigin::Package("bevy_lights/bevy_pbr")` — the sub-package form `join_path`
  produces, which no source file can spell — so each Bevy namespace needs a mount under its own name
  *and* under `bevy_lights/<name>`, and the virtual modules inside each mount are keyed by `Absolute`
  paths. A mount point that does not match fails with "no mount point", which reads as a missing
  module. (Before this was fixed, `bevy_elements`' validator reported every module that imports
  `bevy_pbr` as "not checked" — the shaders that matter most.)
- **A stub's *fields* are checked even though its *body* is not.** The `View` stub declared only
  `world_from_view` and `clip_from_world`; `view.world_position` is in the real struct
  (`bevy_render-0.20.0/src/view.wesl`) and was missing, so a shader reading the camera's position
  failed to parse. That is the safe direction — a wrong name fails loudly — but a stub that declares a
  *subset* of a real struct will reject valid code, and one that declares a *superset* will accept
  invalid code. Declare the fields a shader may read, in the real struct's order.
- **`AlphaMode::Add` needs premultiplied output**: the flame returns `(colour · mask, 0.0)`, and any
  alpha above 0 would darken what stands behind it.
- **An app with no window is not paced.** A headless bench's `App::update()` measures CPU time only;
  it says nothing about GPU cost.
- **`DebandDither::Disabled` for pixel comparisons**: tonemapping's dither depends on pixel position.
- **Bevy maps a point or spot light's lumens to Φ/4π candela** (`bevy_pbr-0.20.0/src/render/light.rs`),
  so a spot light's lumens are *not* comparable to an illuminance in lux without the factor. A wall
  sconce budgeted in lumens needs that factor to reason about the lux it puts on the ground.
- **A light whose `ViewVisibility` is false is not re-extracted**, so writing its intensity costs a
  change-detection edge for no pixels. `flicker_lights` skips the write and still advances the kindle.
  **But on the frame a light is shown, its `ViewVisibility` is still last frame's false** (it is
  recomputed in `PostUpdate`, after `Update`, and the light is extracted that same frame), so a light
  switched on by `Visibility` is written anyway: `is_hidden` exempts a `Visibility` changed (not added)
  this frame. Skipping it drew a pool reserve's first frame at the tail of its last fade, 112 lm for
  70 000. **A light the *camera* turns to is the same trap** (no `Visibility` changes, only the view
  does): `write_lights_coming_into_view` runs in `PostUpdate` after `CheckVisibility` and
  `MarkNewlyHiddenEntitiesInvisible` and writes every light whose `ViewVisibility` is `Changed` and now
  true. A static test camera never shows it; a moving one drew a pool light at 75 903 lm for one frame
  where the fade-in was at 104.
- **A flame's colour is one material per colour, held by ids.** `FlameSite::color` makes `FlameAssets` (and `EmberAssets`) a map from the colour's bits to an `AssetId`, never a `Handle`: the flames hold the material, the asset is dropped with the last of them, and `sweep` forgets the id, so a colour a host changes every frame is one live material and not one per colour ever shown (`a_colour_changed_every_frame_holds_a_bounded_number_of_materials`). A `Handle` in the map would leak every one.
- **A ramp transform that is the identity only at the shipped colour changes every scene that sets its own.** `retint` first rotated the whole ramp by the difference of hues, and the courtyard's `FlameLook::color` (a 1900 K blackbody, 16° redder than `DEFAULT_MID`) pushed the rim past red into pink on every torch: 3 514 pixels differed from the old build. The zero-pixel capture diff of the *default* courtyard against the old build is what found it, so run it for any change to what a flame's colours are made of. The fire family (red to yellow) is the identity; the rotation fades in beyond it.
- **A duplicated plugin panics.** `LightPoolPlugin` therefore cannot add `FlickerPlugin` for the host;
  it reports the missing one in `finish()`.
- **A parent of a visible entity needs the visibility components.** A `FlameSite` gained `#[require(Visibility, InheritedVisibility, ViewVisibility)]` because `dress_flames` spawns a `Flame` under it: without them Bevy warns (B0004) and the child's visibility is not inherited, so a torch's flame does not go out with the torch.
- **Replacing a component leaves its `added` tick alone.** `entity.insert(Kindle::default())` over an existing `Kindle` updates only the *changed* tick (`Column::replace`), so `Added<Kindle>` never reports a re-lit fire. `Kindled` is announced from `elapsed == 0.0` instead, and `modulate` floors the advance at `f32::MIN_POSITIVE` so a zero-delta frame (the first of a fixed-step run) cannot announce the same fire twice.
- **`MeshTag::new` is the only constructor** (`type_id` is a private debug-only field).
- **A billboarded quad's box is the crate's, not the engine's.** The CPU `Aabb` of the unit square does not cover what the
  vertex stage draws, and `NoFrustumCulling` (the old answer) draws and re-extracts every flame in the world every frame:
  1.3 µs a flame, 6.5 ms at 5 000 torches with none in view. A `Flame` carries its own `Aabb` and `NoAutoAabb`: a cube about
  the foot of side `2 · FLAME_REACH · size`, **divided by the instance scale** (`flame_cull_extents`), because the catch and
  the wind's lean are the scale's `x`, `y` and `z` and the engine multiplies a box by its entity's scale. A scale that is data
  distorts any box built once; `set_scale` rewrites the box on exactly the frames it rewrites the scale. The test
  `a_flame_any_part_of_which_is_on_screen_is_never_culled` carries a port of the vertex stage's corner formula: change that
  geometry in `src/flame.wesl` and the port and `FLAME_REACH` must change with it, or flames pop at the screen's edge.
  The ember quad (`FlameEmbers`) is the same trap with a wider reach that grows with the lean: `ember_cull_extents`
  (`ember_reach`: `sqrt((0.7 + 3.9 · |lean|)² + 2.95²)` flame heights), rewritten by `update_embers` on the frame the flame's
  scale is, *after* `update_flames`. Change `HALF_WIDTH`, `RISE_MAX`, `MARGIN` or `DRIFT` in `src/embers.wesl` and the port in
  `an_ember_quad_any_part_of_which_is_on_screen_is_never_culled` and `model/tag.rs`'s mirrors must change with them.
  The smoke quad (`FlameSmoke`) is the ember quad's geometry constant for constant and uses `ember_cull_extents`: change `HALF_WIDTH`,
  `RISE_MAX`, `MARGIN` or `LEAN_TOP` in `src/smoke.wesl` and its own port (`a_smoke_quad_any_part_of_which_is_on_screen_is_never_culled`)
  and the ember mirrors change together, or the smoke of a fire at the screen's edge pops. A smoke column is **additive only**: a blended
  smoke sorts against every other transparent surface of the host (`docs/research/flame-smoke.md`), and a column drawn thinner than a pixel
  crawls, so `smoke.wesl` floors its width at a pixel and a quarter and fades its noise out when it cannot be resolved.
- **A smoke trail rides two data channels, and a bit stolen from a field must be cleared on the normal path.** A put-out fire's smoke (`docs/research/flame-smoke-tail.md`) is the same quad in another mode: the quad's world scale `x` is `1 / (1 + seconds since the douse)` (the flame's own `x` is `0.001` by then, so `update_smoke_tails` sets the quad's *local* scale to `x / flame.x`; a world scale is the flame's times the quad's, and the culling box is divided by the world product, `tail_cull_box`), and the lowest bit of the tag's 16 height bits says "trail". Half of all packed heights have that bit set, so `steady_tag` clears it (`a_steady_quads_tag_is_never_read_as_a_trail_whatever_its_height`): the first build read every odd-height quad as a trail and the capture diff passed by luck, because a trail at `τ = 0` is the steady plume. `begin_smoke_tails` runs its quad loop only on a frame with a new `Snuff` somewhere (`Added<Snuff>` is the early out), and `update_smoke`'s clock tick must not sit behind a test for "does this system own any quad", because a trail quad is excluded from its query.
- **`cargo test` hides holes in a trimmed feature list**, because the full-`bevy` dev-dependency
  unifies features into the library graph. `cargo build` alone compiles a consumer's view.
