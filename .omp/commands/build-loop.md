---
description: One iteration of the bevy_lights improvement loop
argument-hint: "[optional focus, e.g. 'flicker only']"
---

One run of this command is one iteration of a long-running loop on bevy_lights, a Bevy 0.20 crate for
practical lights (fires that flicker and kindle, a budgeted pool of real lights, billboard shader
flames) that ships in a real game. Do ONE improvement per iteration, the single most valuable one
from a game developer's point of view, finish it completely, then stop.

This crate must be performant AND beautiful. Both are gates: a feature that is fast but merely
"fine" looking is not done, and a gorgeous feature that blows the frame budget is not done. Lighting
is also where a scene is *read*, so the art-director judgement here is a **cinematographer's**:
key-to-fill, motivated sources, warm against cool, silhouette, falloff.

You do all the work yourself, including looking at the images and judging them. Do not delegate
visual review.

Extra instructions for this run (a focus such as "flicker only"): $ARGUMENTS
They override the choice of item in section 4, not the gates.

State lives in `docs/loop-log.md`; rules and traps live in `AGENTS.md`. Follow them; do not
re-derive them.

## Your work is saved even if you are killed — read this before you start

**The driver saves whatever you leave uncommitted, without touching your branch.** Every ten minutes
it records the whole tree to a snapshot ref (`refs/wiggum/pass-N-...`), and again when the pass ends.
Your `git status`, your index and your branch are unchanged — so this is not a substitute for your own
commit step, which is still the only thing that puts work on `main`.
Recover a killed pass's tree with `git -C <repo> log --all --oneline --grep='WIP(pass'` then
`git show <ref>`.

1. **Commit as you go, not once at the end.** Your commit step is last, behind implement, gate and
   benchmark, so a pass killed at the ceiling never reaches it. Commit the research note alone before
   writing code, as your sections already say; then commit the implementation, then the docs and the
   log. Each commit that lands is work a killed pass cannot lose.
2. **A snapshot ref is not a commit on your branch.** `git log` will not show it. Do not look for
   one; if an earlier pass was killed, its tree is in `refs/wiggum/` and nobody's gate ran on it.
3. **Parked work is a branch, and you look for it before you pick an item.** When a pass is stopped or
   killed with its tree uncommitted, the driver's next start commits that tree to a branch
   (`wiggum/parked-pass-N-<time>`, or on the pass's own `loop/iter-N`) in a commit titled
   `WIP(pass N, parked)` and puts the clone back on `main`: you start clean and nobody tells you. So run
   `git branch --list 'wiggum/parked-*'` and `git log --all --oneline -n 5 --grep='parked'` once, after
   you have triaged the filed issues and before you commit to an item. If the newest parked tree is for
   the item you would pick, or for an issue or bug that outranks your pick, **adopt it**: it is a first
   draft that was never built, tested or gated and was cut off mid-work, so read it, bring its files into
   your own `loop/iter-N` branch (`git checkout <parked-branch> -- <paths>` or `git cherry-pick`),
   finish it, and run the real gate on the result before it ships under your name. Never merge the
   parked branch itself and never delete it. If it is stale (its files conflict with `main`, or the item
   was fixed meanwhile), say so in one line in the log and ignore it. A parked branch that exists only on
   this clone holds a lot of data: the driver keeps anything over 10 MB of new data off origin.

## 1. Orient

1. `AGENTS.md`: rules, Bevy 0.20 traps, gates.
2. `docs/loop-log.md` (create it if missing): what earlier iterations did, found and deferred, and the
   newest iteration's ranked backlog. The next iteration number is the last one plus one.
3. `git log` since the last logged iteration.
4. The `docs/research/<system>.md` note of any system you may touch, and `docs/research/corpus.md` for
   the reading list per domain.
5. `git status --short`. The driver (`scripts/bevy-build-loop.sh`) starts a pass only on `main` with no
   tracked changes. If you find tracked changes you did not make, stop and log them; do not commit or
   revert them.
6. **Re-scan the corpus for new lighting sources.** Run `catalog_recent` (limit 200) and add any
   cinematography, photography, lighting-design, rendering or fire-physics document that
   `docs/research/corpus.md` does not list, as its own commit (`Research: corpus`).

Preflight home-still now, so research is not blocked later:

- `system_status` (MCP) or `hs status`. If distill is down, run `systemd-run --user --scope --collect hs
  restart` once, never a bare `hs restart`: that starts Qdrant's podman monitors inside this loop's own
  systemd unit, so restarting the loop restarts Qdrant (it did, on 2026-10-28). If it stays down, skip
  `distill_search` and `distill_index` and list each unindexed note under `Pending index`. Record the
  live state in the log; do not copy an earlier iteration's outage note.
- Scribe: check `scribe_health`. `scribe_convert` may queue or fail; then rely on the corpus.
- Corpus: `~/.cache/bevy_lights_corpus/markdown`. If it is missing, sync it per
  `docs/research/corpus.md`.

## 2. Capture the current state

Build first (`cargo build --release --examples --all-features`; the WESL shader is embedded at build
time, so a stale build silently captures the old shader). Keep everything under `/tmp/lN/` (`N` the
iteration), the baseline under `/tmp/lN/before/`. Keep a worktree of the starting commit for later
comparisons, **detached, at a commit, and off `/tmp`**: `git worktree add --detach ~/lN_base <sha>`,
built with `CARGO_TARGET_DIR=~/lN_base-target`.

Section 9 says how to name, build and remove that baseline. A baseline left behind costs the next pass
a build cache's worth of disk, and one with `main` checked out stops anything else checking `main` out.

On **big** (Linux, RTX 3090). Check the GPU first: `gpu-tenant status`. If VRAM free is under 2 GB, log
it and wait rather than fighting another holder.

```sh
xvfb-run -a -s "-screen 0 1920x1080x24" env LIGHTS_FRAMES=600 LIGHTS_SHOT=/tmp/lN/before/x.png \
  ./target/release/examples/courtyard
```

Capture from MULTIPLE angles with `LIGHTS_EYE` / `LIGHTS_LOOK`:

- ground level (a viewer's eye, ~1.7 m), eye level at the courtyard's edge, elevated, top-down, and at
  least one **grazing** angle across the ground, where falloff and the range cut-off show worst;
- at least two `LIGHTS_SEED`s (a different arrangement of figures) and pool sizes `LIGHTS_POOL=4,8,16`;
- frame sequences for anything temporal (`LIGHTS_SHOT_DIR`, `LIGHTS_SHOT_EVERY`, `LIGHTS_SHOT_FROM`).
  Flicker, kindle and the pool's hand-offs are all temporal and a still hides every one of them. Skip
  the first ~13 frames (pipelines compile, the pool spawns on frame 1); steady state is ~frame 20.

Xvfb traps that have cost whole baselines: `/tmp` (a 16 GB tmpfs on big) may be full, so Xvfb cannot
make its socket; the display `-a` picks may be held by something invisible to this namespace. Check
`df -h /tmp`, pass `xvfb-run -n <N>` with a fresh number per run, and retry a run that leaves no
image. Keep worktrees and their `target/` off `/tmp`.

`LIGHTS_FRAMES` pins the run at a fixed 1/60 s step, so the simulated time is the same every run. GPU
output is not bit-deterministic.

## 3. Inspect closely (you judge the images yourself)

You do all the work yourself, including looking at the images and judging them. Do not delegate
visual review. Read each image with `read` (it decodes PNGs inline) and zoom in on the fires.

- Crop and upscale (nearest-neighbour, 2x-4x): `magick in.png -crop WxH+X+Y +repage -filter point -resize 400% out.png`.
- Use OpenCV or Pillow where useful (not installed system-wide; run scripts with
  `uv run --no-project --with opencv-python-headless --with pillow python script.py`): per-frame mean
  luminance plots, difference images between consecutive frames, histograms.
- Quote pixel values when a number settles it: `magick file.png -crop 40x40+X+Y +repage -format "%[fx:mean*255]" info:`.

Always view at least one actual-resolution full frame: a crop can pass while the whole looks wrong.

Write the **craft checklist** to `/tmp/lN/findings.md`, each line with a measurement or a file and
region, not an impression:

- **key-to-fill ratio**, measured in DN over a fire's pool versus the shadow beside it;
- **falloff shape**: an inverse-square read with no visible hard cut-off at the light's `range`;
- **motivated light**: every pool has a visible flame at its source, and every flame lights something;
- **warm/cool separation** against the moon fill, and whether the frame reads as night;
- **silhouette and readability**: can a figure be read against what is behind it;
- **specular highlights**: present on the stone, and placed where a fire would put them;
- **flicker rate and amplitude** against the research note's target, with a per-frame mean luminance
  plot across the sequence;
- **pool hand-off popping**: consecutive frames, a light that moves;
- **flame size against its light's reach**: is the fire's size believable for the ground it lights;
- **bloom and clipping**: the flame's core and the light's hot spot, at full frame and at zoom.

## Filed issues come before new features

**A filed issue is a consumer telling you what broke, which outranks anything this pass invented.**
Before you pick an item, list the repository's open issues and triage them:

```sh
gh issue list --repo Ladvien/bevy_lights --state open --limit 100 \
  --json number,title,labels,createdAt --jq '.[] | "#\(.number) \(.title)"'
```

(If `gh` is absent, the same list is
`https://api.github.com/repos/Ladvien/bevy_lights/issues?state=open&per_page=100`.)

Then, in this order:

1. **A bug issue is the pass's item, ahead of any enhancement.** Anything reporting wrong behaviour —
   a crash, a panic, a visual artifact, a value that is implausible, a documented promise that is not
   kept — is a bug. Take the oldest such issue that is still reproducible, and say why it beats the
   others.
2. **An issue labelled `consumer` is the pass's item next, ahead of the series and your own candidates.**
   A project built on this crate needs it and is waiting. The `consumer:<name>` label says which project;
   `consumer:bevy_emerge` is the owner's editor. Its own work resumes when the issue is closed as completed
   and it moves its git pin to a `main` commit containing the fix. Take the oldest one you can work this pass.
   - **Build the general form.** The requester is the example, never a name in this crate's API, a path or a
     default: this crate serves other projects too. Where an issue asks for something shaped to one project,
     build the general form and say so on the issue.
   - **Prove it here**, with a test in this repository that fails before and passes after against the issue's
     acceptance. Never build or run the requesting project to prove it.
   - **Pushing to `main` is the delivery.** Requesters pin a git revision, so a consumer fix never needs a
     publish or a version bump.
   - **One that cannot be worked this pass** (it needs a decision the issue does not settle, or a protected-file
     change): say exactly what is needed on the issue (`gh issue comment`), write one log line, and go on.
     A blocked consumer issue never stops the pass.
   - **Read `git log` first.** While this loop is stopped, the requester's loop may fix the issue here at the
     source, in its own commits with tests; check whether that has happened so the work is not done twice.
3. **Every other open enhancement issue is the pass's item next, ahead of your own candidates** (the
   owner's rule, 2026-10-29): a filed request was asked for; a candidate this pass invents was not. The
   order of work is: **bugs first** (a bug issue, then a bug already recorded in the backlog or the log),
   then `consumer` issues, then every other open enhancement issue, oldest first. Only when each of them
   is done, blocked or a release issue does "Pick the single most valuable item" choose a feature of your
   own. A blocked issue gets a `gh issue comment` saying exactly what it needs and is skipped; it never
   stops the pass. Name the issues you skipped, and why, in the log line, so the next pass sees the list
   was worked through.
4. **Close the loop.** An issue your pass fixes is referenced in the commit body (`Fixes #N`) **and**
   in the report, so the next pass can see it was addressed. Do not close an issue you did not fix,
   and do not close one whose fix this pass did not verify. For a `consumer` issue the close is the signal
   the requester waits for: close it only through a verified fix pushed to `main`, never as not planned
   without the owner's word.
5. **Re-check a release issue, do not act on it.** An issue asking to publish or bump a version is
   the owner's call, never a loop's (see the release rule above). Record it as seen and skip it.

**When the API fallback answers `403` or `429`** (rate limit — 60 requests/hour per IP unauthenticated,
shared by everything on this network), do not treat it as "no issues": write one line saying the list
could not be read and continue. Never spend a second call to retry it in the same pass, and never infer
from silence that nothing is filed.

**An empty issue list costs nothing:** if it is empty or `gh` is unreachable, write one line saying so
and continue — never spend a pass on the lookup.

## 4. Pick the single most valuable item

**You are here only when no bug and no open issue can be worked this pass** (see "Filed issues come
before new features"); otherwise that issue is the item. Rank candidates from your findings, the loop log's backlog and the extra instructions, always asking
what matters most to a game developer who will ship this:

1. Visible artifacts or physically wrong results (popping, a hard range cut-off, a light with no
   source, flicker that reads as a strobe, a flame detached from its fire).
2. Features that look flat or unconvincing next to what shipping games achieve. Beauty gaps rank
   here, not last.
3. API friction: missing knobs, confusing setup, a game wanting to query something the crate hides.
4. Missing features a game would expect from this domain.
5. Performance that threatens a frame budget.

Ownership is a gate: if `AGENTS.md`'s ownership map gives the item to a sibling crate, do not build it
here; add a hook instead.

State in one sentence why your pick beats the runner-up. If it is too big, carve off the most valuable
self-contained slice. Record the ranked candidate list in the log, not only the winner.

## 5. Research thoroughly (before any code)

Every feature gets researched, new or improved, with no exceptions. Lighting is art direction with
physics attached, so every iteration cites **three** kinds of source:

1. a **craft** source — cinematography, photography or a gaffer's writing (`docs/research/corpus.md`
   has the list: *Cinematography Beyond Technique*, the low-key and noir papers, *Light: Science and
   Magic* when it can be imported);
2. a **shipping-game or production** source — a GDC talk, a SIGGRAPH *Advances* course note, a studio
   technical write-up (`s<year>-advances-*` stems);
3. a **technical or academic** source from the corpus — the rendering or fire-physics literature.

If the corpus lacks one of the three, `paper_search` for it, and record the gap under the note's
`## Pending index`. Write the note with the headings `AGENTS.md` requires (`## Sources`,
`## Craft principle`, `## How shipping games do it`, `## Model adopted`, `## Parameters`,
`## Validation targets`, `## Deviations`, `## Pending index`) plus `## What great looks like`: the
visual qualities and reference images you are aiming for. `## Craft principle` and
`## How shipping games do it` must each **quote** the source they rest on, not paraphrase it from
memory.

Any changed default goes under `Deviations` with a citation, or is marked a design default. Commit the
note alone before writing code: `git add docs/research/<system>.md && git commit -m "Research: <system>"`.

## 6. Implement within the crate's rules

- Keep every rule in `AGENTS.md`. In particular: no game vocabulary in `src/` (`tests/leaf.rs`
  enforces it); no `unwrap()`/`expect()` outside tests; settings are `Resource + Default + Clone +
  Debug + Reflect` added with `init_resource`; messages use `add_message`; everything is re-exported
  flat from the crate root; systems go in the existing chained sets; one path per feature, no
  fallbacks or shims; delete the code the change replaces.
- `src/model/` stays engine-free: if the new maths can be a plain function, it goes there, with its own
  tests, so it is testable without an app.
- **Run `tools/wesl-check` before any capture** if you touched `src/flame.wesl`:
  `cargo run --release --manifest-path tools/wesl-check/Cargo.toml -- src`. A WESL error is silent at
  build time.
- Sibling crates are coupled by plain data only. A needed sibling change lands in that repo first, with
  its own tests and commit, and is named in the log.
- Tests: for a bug, a test that fails before and passes after. Permanent tests check consumer-visible
  behaviour, boundaries and invariants; throwaway checks go in `/tmp`.

## 7. Verify and sign off: it must look GREAT

You are grading your own work, so grade it as a hostile art director would — and as a
cinematographer, who will notice the light that has no source before they notice anything you are
proud of.

- Rebuild the examples, re-capture the same scenes, seeds, pool sizes, cameras and angles as section 2
  under `/tmp/lN/after/`, and rebuild the same zoomed evidence set.
- View before and after side by side, every angle, and the temporal sequence frame by frame.
- Fill in the craft checklist for the "after" captures and compare each line with the "before".
- Write the critique BEFORE the verdict (`/tmp/lN/critique.md`): list every flaw you can find at zoom
  and at full frame, against the research note's `## What great looks like` and `## Validation
  targets`. Look for what is wrong, not for confirmation that your change worked.
- Then give each feature exactly one verdict: GREAT, NOT YET or FAIL. GREAT means you would be proud to
  ship it in a showcase. "Fine", "acceptable" and "good" are NOT YET.
- On NOT YET or FAIL, write the specific changes that would move it to GREAT, apply them, rebuild,
  re-capture and re-judge. Do not lower the bar to pass.
- After 5 failed rounds on one feature, stop: log the blocker, the remaining flaws and your best
  hypothesis, and revert to the last GREAT state.
- **Performance gate**, equal in weight: `cargo run --release --example bench_lights` before and after
  at the default sizes, reporting the `min_us` and `p50_us` lines. No regression without a logged,
  justified trade. Compare only on the same machine and the same load; the bench is CPU-only, so it
  says nothing about GPU cost — for a shader change, compare the capture's frame time in the log.
- Run the full gate (the `AGENTS.md` set):
  1. `cargo build --release && cargo build && cargo test && cargo test --features serde`
  2. `cargo build --examples --all-features`
  3. `cargo clippy --all-targets --all-features -- -D warnings`
  4. `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`
  5. `cargo fmt --check`
  6. `cargo build --lib --target wasm32-unknown-unknown`
  7. `cargo run --release --manifest-path tools/wesl-check/Cargo.toml -- src`

## 8. Ship

Start only after section 7 passes. Never bump the version, tag or publish; releases are the owner's
call.

1. Document: rustdoc for every changed public item, the README for user-visible changes, the research
   note's `Deviations` and `Pending index`, and an entry under `## [Unreleased]` in `CHANGELOG.md`
   (`Added`, `Changed`, `Fixed`, `Removed`).
2. Regenerate the GIF and still in `docs/media/` that this change affects, from the new build
   (`docs/DEMOS.md` has the recipe).
2b. **The README is checked before every push, and `docs/media/showcase.gif` is re-encoded whenever the
   look it shows has moved.** Its hero GIF is what makes someone try the crate: two seconds, 30 fps, under
   ~2 MB, showing as many of the top features as one clip can (the catch, the flicker, a hand-off). Its
   recipe and the two encoding traps — the encoded `FPS` must match the capture's content rate, and
   `dither=bayer` crawls on a dark scene — are in `docs/DEMOS.md`. Verify the flicker survives the encode by
   measuring a *single* pool (four pools at different phases cancel and read ~1.5 Hz whatever the truth is).

3. Log: append `## Iteration N — <date> — <title>` to `docs/loop-log.md`:
   - what changed and why it ranked first (and the runner-up)
   - the sources used: the craft, production and technical stems, by stem
   - your critique and verdict per angle, with evidence filenames
   - the craft checklist's numbers, before and after
   - benchmark numbers, before and after
   - the commit hash, and any sibling-repo commits
   - the top 5 remaining items, ranked, for the next iteration
4. Branch and commit: `git checkout -b loop/iter-N` from `main`. Commit by path with messages that
   state the user-visible change: the research note (already committed in section 5), the
   implementation, the docs and changelog, then the log (`Loop log: iteration N, <title>`).
   `git status --short` must be empty afterwards.
5. Push and merge: `git push -u origin loop/iter-N`, then `git checkout main && git pull --ff-only &&
   git merge --no-ff loop/iter-N -m "Merge loop/iter-N: <title>"`. On a conflict, `git merge --abort`,
   log it, and stop.
6. Re-run the gate on the merged tree; a failure stops the pass before the push. Then
   `git push origin main`.

### README, demos and docs stay true

A README is read before anything else, so a false line there costs more than a false line anywhere else.
An audit on 2026-10-27 found one in each of the four loop repos: a `tour.gif` captioned "at noon" over a HUD
reading 10.00 h with a running clock; a `ties.gif` captioned for rivalry colours that appear in none of its 240
frames; a hero caption promising a flame catch that its file never shows; and a demo video captioned as the full
recording of the GIF above it when it shows different objects. Before you push a change a user can see:

1. **Put it in the section it belongs to.** Do not append to the introduction or to the end, and do not
   restructure the README in a feature pass. It reads in this order — what it is in one line, the demo, what
   you get, install, quick start, then the detail sections — and an item added in the wrong place undoes it.
2. **A caption describes the file as it exists.** Open the GIF or still (frame count, size, any HUD burned into
   it) and write only what you can read off it; a figure quoted from program output comes from running that
   program now, not from the log. Recipe: `tools/gif.sh` and `docs/DEMOS.md`.
3. **Media you cannot regenerate here is logged, not left lying.** With no GPU or display on this machine, name
   the stale file and its path in the report and change its caption to what it still shows.
4. **The quick-start block is compiled.** `tests/readme.rs` builds it in a scratch crate; edit it only together
   with the code it shows, then run `cargo test --release --test readme`.

## 9. Clean up when you are done

Worktrees and build directories outside the clone are not free: a `target/` for this crate is tens of GB, big's
`/tmp` is RAM, and on 2026-10-27 earlier passes had left 37 GB of baseline builds and 5 GB of scratch behind.
**After you ship and before you write the pass's last log line, whether or not you shipped,** remove what *you*
made this pass, and list what earlier passes left.

1. **A baseline is a pair that can be found.** Make it with `git worktree add --detach <path> <sha>` (never a
   branch name: a worktree with a branch checked out stops anything else checking that branch out) and build it
   into `<path>-target`. Then `git worktree list`, plus the `*-target` directory beside each worktree, is the
   whole inventory.
2. **Remove yours.** `git worktree remove <path>` (plain, never `--force`) and `rm -rf <path>-target`, then your
   scratch under `/tmp` (captures, frames, venvs, gate logs, `rustdoctest*`). Remove only what you created this
   pass, and name each path in the log. If git refuses because the worktree has a change in it, name the file
   in the log and leave it.
3. **Leftovers from earlier passes are listed, never deleted.** A killed pass cannot clean up after itself, but
   an agent does not delete what it did not create, so list them once and let a person approve the removal:

   ```sh
   git worktree list --porcelain | awk '/^worktree /{sub(/^worktree /,""); print}' | tail -n +2   # all but the clone
   du -sh <wt> <wt>-target                       # the size of each one that is not yours
   git -C <wt> status --short                    # empty: nothing is lost by removing it
   find <wt> <wt>-target -mmin -120 -print -quit # empty: untouched for two hours, so no live build is using it
   ```

   Log each with its path, size, and whether it is clean, already on `origin/main` and idle (the likeliest to be
   stale). Say so, and still leave it.
4. **Never remove** the clone itself, its own `target/` (the build cache the next pass starts from), anything you
   did not create this pass, or anything a process is using.
5. Write one log line: what you removed and the space it freed (`df -h .` before and after), and the leftovers
   you listed.

## Rules

- One improvement per iteration, fully finished: researched, implemented, GREAT-signed-off,
  benchmarked, gated, committed, documented, logged.
- No feature ships without a written critique, a GREAT verdict and a clean performance check.
- If blocked, log the blocker and pick the next item. A failed gate or a visible defect is not a
  blocker: fix it. A failed push or merge is: log the exact error and stop. Never force-push.
