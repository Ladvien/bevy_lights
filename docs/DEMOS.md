# Captures and demos

How `docs/media/` is made, so two captures a month apart are comparable. Every capture is taken from
the same build, the same frame count and the same camera as the one before it.

## Before you capture: `/tmp`

**Check `df -h /tmp` first.** On big `/tmp` is a 16 GB tmpfs that other agents fill to 100 %. A frame
sequence written into it comes out as truncated PNGs (`LIGHTS_SHOT_DIR` does not notice), and
`tools/gif.sh`, which `mktemp`s there, dies with ENOSPC or encodes a GIF of the frames that survived.
Send both to disk: `export TMPDIR=/home/ladvien/tmp-lights` (it must exist) and put
`LIGHTS_SHOT_DIR` under it. A 241-frame sequence is ~180 MB at 1280 x 720.

## The frame sequence

`examples/courtyard.rs` writes PNGs itself, at a fixed 1/60 s step, and exits when it reaches
`LIGHTS_FRAMES`:

```sh
LIGHTS_FRAMES=300 LIGHTS_SHOT_DIR=/tmp/lights-frames LIGHTS_SHOT_FROM=20 \
  cargo run --release --example courtyard
```

`LIGHTS_SHOT_FROM=20` skips the first 19 frames: the flame material's pipeline compiles and the pool
spawns its lights in the first frames, so an early frame is not the steady state. On a machine with no
display (big), put it under Xvfb:

```sh
xvfb-run -a -s "-screen 0 1920x1080x24" env LIGHTS_FRAMES=300 \
  LIGHTS_SHOT_DIR=/tmp/lights-frames LIGHTS_SHOT_FROM=20 ./target/release/examples/courtyard
```

## Seeing a fire's size

A brazier's coal bed is 0.34 m, so the puffing law gives its light 2.6 Hz; the crate's inherited rate
is 1.16/2.08 Hz. The example takes the bed and can be told to ignore it:

```sh
# sized (the default): a brazier breathes at the law's rate
LIGHTS_FRAMES=200 LIGHTS_SHOT_DIR=/tmp/lights-sized LIGHTS_SHOT_FROM=20 \
  ./target/release/examples/courtyard

# unsized: the same fires at the crate's inherited rate
LIGHTS_UNSIZED=1 LIGHTS_FRAMES=200 LIGHTS_SHOT_DIR=/tmp/lights-plain LIGHTS_SHOT_FROM=20 \
  ./target/release/examples/courtyard
```

The run prints `flicker frame N: <lumens>` every 30 frames, which is the honest way to read the rate:
a frame's mean luminance cannot see a ±15 % flicker on four lights. If the two runs are identical,
check that the switch reached the process — a shell that passes `LIGHTS_UNSIZED=` on both sides
captures the same thing twice.

## The README showcase

`docs/media/showcase.gif` is the README's hero: two seconds, 30 fps, 400 px, 1.9 MB. `TMPDIR` and
`LIGHTS_SHOT_DIR` both go to disk, not `/tmp` (see the top of this file). Its recipe, and the two
things that made the first attempts look wrong:

```sh
# 1. Capture at 2 saved frames per simulated second => 30 fps of content (60 fps sim, EVERY=2).
LIGHTS_FRAMES=260 LIGHTS_SHOT_DIR=/tmp/show LIGHTS_SHOT_FROM=9 LIGHTS_SHOT_EVERY=2 \
LIGHTS_FOCUS_WALK=18 ./target/release/examples/courtyard
# 2. Take the first 60 saved frames (2 s: the lights still kindling, then the fires breathing) and
#    encode at FPS=30. The flames are already full size here — see "Seeing a flame catch".
FPS=30 WIDTH=400 FONT=… BOLD=… tools/gif.sh /tmp/show/short docs/media/showcase.gif "…"
```

**Trap one: the encoder's frame rate must match the capture's.** `LIGHTS_SHOT_EVERY=N` saves one frame per
N simulated frames, so the content is `60/N` fps. Encoding that at any other `FPS` time-warps it — the first
attempt showed 24 fps of content at 48/2 and ran 1.2× fast, which made a 2.6 Hz flicker read as 3.1 Hz and
had the user ask whether the *flicker* was wrong. It was the GIF.

**Trap two: `dither=bayer` on a dark scene crawls.** A per-frame ordered dither pattern on a slow luminance
gradient moves every frame, so it reads as noise laid over the flicker — the very thing the demo shows.
`tools/gif.sh` now uses 256 colours with `dither=none`; it bands slightly and is stable, which is the right
trade for this subject.

**Verified:** one brazier's pool in the encoded GIF reads **2.50 Hz** against the law's 2.57, peak-to-peak
74 % of its mean. Measure a *single* pool, not the frame: four pools at different phases cancel and the
number reads 1.5 Hz whatever the truth is.

Re-checked against the committed `showcase.gif` on 2026-10-27, on 20 x 20 px windows of lit ground beside
a brazier that never clip (a clipped window is a flame's own edge and says nothing about the light): the
strongest component above 2 Hz is **2.27 Hz** in the brightest of them, 6.6 % of that window's mean, and
2.38 Hz in the widest swing (34 % peak to peak). A whole-frame mean at 2.57 Hz is 0.35 % of its mean,
which is the point of the paragraph above. The spread from 2.50 to 2.27 is the crop's own noise floor
over a two-second file, not a different rate: at 30 fps and 60 frames the periodogram's bins are 0.5 Hz
apart, and every number here comes from reading between them.

## Seeing a flame catch

A fire that is being lit grows over `KINDLE_SECONDS` (0.6 s), light and flame together. The courtyard's
braziers start with `Kindle::default()`, so its first half-second is the whole courtyard catching — which
means the frames to look at are the first dozen, not the steady state:

```sh
LIGHTS_FRAMES=60 LIGHTS_SHOT_DIR=/tmp/kindle LIGHTS_SHOT_FROM=1 LIGHTS_SHOT_EVERY=1 \
  ./target/release/examples/courtyard
```

Measured on those frames, the count of saturated flame pixels runs **0 → 4 430 → 15 145 → 19 891** at
frames 5, 10, 20 and 40, then holds at about 19 000: the flame grows into its fire rather than appearing
whole.

**A trap: `LIGHTS_SHOT_FROM=1` does not actually reach the catch on big.** The catch runs on
`Time<Real>` (`update_flames` advances `Flame::kindled` by `Real` delta), and the first frames of a run
are pipeline compilation — which is *real* seconds, not the fixed 1/60 s the harness steps `Time` by. On
big the flame material's pipeline takes longer than `KINDLE_SECONDS` to compile, so by the first captured
frame the catch is already over: measured on the courtyard at `LIGHTS_SHOT_FROM=1`, the frame mean goes
`0.00 → 5.33 → 36.03 → 53.53` over four frames with the flames already full size. That is why
`docs/media/showcase.gif` has never shown a catch, in this build or the committed one. To see a catch,
spawn the `FlameSite` **after** the run has warmed up (a site added at frame 40 renders through the
already-compiled pipeline, and its kindle starts then), which is how iteration 18 measured it:
`/home/ladvien/l18/kindle2/` (before) and `kindle2_after/` (after).

### The relight clip (`docs/media/relight.gif`)

A re-light after the pipeline is warm is the catch the trap above cannot reach. The courtyard's
`LIGHTS_SNUFF` and `LIGHTS_RELIGHT` do it unattended; the frame mean (`magick f.png -colorspace Gray
-format '%[fx:mean*255]' info:`) is the honest clock, 53.7 at the start, 27.2 at the darkest (saved frame 37-41), back to
54.6 by frame 53:

```sh
LIGHTS_SNUFF=1.0 LIGHTS_RELIGHT=2.3 LIGHTS_EMBERS=4 LIGHTS_FRAMES=200 LIGHTS_SHOT_DIR=$HOME/tmp-lights/relight \
  LIGHTS_SHOT_FROM=57 LIGHTS_SHOT_EVERY=2 LIGHTS_FOCUS_WALK=18 \
  xvfb-run -n 151 -s "-screen 0 1920x1080x24" ./target/release/examples/courtyard
# keep saved frames 0-63 (2.1 s at 30 fps; all 72 is 2.0 MB), then tools/gif.sh as for the showcase, WIDTH=400
```

A crop of one brazier at saved frames 40, 43, 45, 47, 49, 53 shows the flame growing out of the coals
while the light flares (a tongue at 45, full height by 49).

## The bench's two lines

`bench_lights` prints two lines now. The first is the whole frame; the second separates the crate's two hot
system sets, because a total tells you a change cost something and not *what*:

```text
bench_lights candidates=5000 lights=2000 pool=16 snuffed=0 frames=600 mean_us=… p50_us=… min_us=… max_us=…
bench_lights_systems flicker_p50_us=170 pool_p50_us=306
```

The per-system pass re-runs the same app with a resource switch (`Only`) that leaves one of
`LightsSystems::{Flicker, Pool}` enabled. `the_switch_really_isolates_the_two_sets` in the example's tests
proves the switch works — with `Only::Flicker` the pool spawns no slots, with `Only::Pool` the flicker writes
no intensity — because a per-system line from a switch that does nothing would just be the frame twice.

At 5 000 candidates and 2 000 lights the split is roughly **170 µs flicker, 310 µs pool**: the pool is the
expensive half, which is what a change to the pool's sort should move.

`BENCH_SNUFFED=<n>` puts the first `n` candidates out, each with a finished `Snuff`: the pool at dawn, or a saved scene
whose torches were put out. Run it at 0 and at `BENCH_CANDIDATES` alternately; the two `pool_p50_us` should agree
(the pool must meet a fire that is out at no more than the cost of a lit one).

## The bench, and the host

`cargo run --release --example bench_lights` is CPU-only, so it measures the host as much as the crate.
On a machine with other work on it (this Mac has read load averages near 20 from other agents) the same
binary reports p50 495, then 856, then 865 µs — that is the load, not the code. A change is compared by
building the base commit and the change into **separate target directories** and running the two
binaries **interleaved**, three rounds each; a reading whose repeat moves by more than 10 % is thrown
away.

## The flame bench: thousands of fires, a few on screen

`bench_lights` has no renderer, so it cannot see a flame. `bench_flames` can: it spawns `BENCH_FLAMES` torches (default 5000) on a 3 m grid, points one camera at a corner of the grid so that about twenty are in view, renders 400 frames to an off-screen image (the first 100 are not counted: the flame material's pipeline compiles in them) and prints

```text
bench_flames flames=5000 visible=15 embers=0 embers_visible=0 smoke=0 smoke_visible=0 wind=0,0 frames=300 mean_ms=… p50_ms=… min_ms=…
```

`visible` is the number of flames the renderer counted as in view at the last frame, which is the proof that the others were culled; `BENCH_WIND=3,0` makes every flame lean, so each one's scale and culling box are rewritten every frame; `BENCH_EMBERS=1` gives every torch `FlameEmbers` (a second quad a flame) and `embers_visible` counts those quads the renderer drew. Each `*_ms` is one `App::update()`, the main world and the render world's CPU work together, and the bench needs a GPU adapter. It runs without a display or a window (`DefaultPlugins` with `WinitPlugin` disabled and the camera on an `Image`), so it works over ssh. A GPU busy with other jobs makes `create_render` panic at start-up: check `nvidia-smi` and run it again.

Compare two builds the way `bench_lights` is compared: separate target directories (or a copied binary), interleaved, three rounds. Iteration 277's numbers, best of three on a loaded machine, in ms a frame: 0 flames 1.3 and 1.7 (noise: the order reversed reads the other way), 1 000 flames 2.2 → 1.9, 5 000 7.8 → 2.2, 20 000 30.0 → 6.4, and with `BENCH_WIND=3,0` 5 000 8.0 → 3.1 and 20 000 28.1 → 5.9, before → after culling. What is left, about 0.25 µs a flame, is `update_flames`' own per-flame work and Bevy's visibility pass over the boxes.

Iteration 278's numbers for the ember quads (`BENCH_EMBERS=1`, best of three interleaved runs on a machine at load average 15-26, in ms a frame), before → after the quad got a box of its own: 5 000 flames 11.5 → 4.2, 20 000 37.2 → 9.2, and with `BENCH_WIND=3,0` 5 000 8.8 → 3.6; `embers_visible` fell from every quad in the world to 18-21. A quiet-machine pair before the change (load 18) was 3.7 → 9.5 ms at 5 000 torches without and with sparks, and 9.4 → 46.3 at 20 000. Build the old side as the new `bench_flames.rs` over the old `src/` (`git checkout <old> -- src`), because the old bench has no `BENCH_EMBERS`.

Iteration 281's numbers for the smoke quads (`BENCH_SMOKE=1`: `smoke_visible` counts the quads the renderer drew, 18 of 5 000, 21 in a 3 m/s wind), the old side being the build before `FlameSmoke` (a baseline worktree built to its own target dir, `bench_flames` with no smoke fields), three interleaved rounds on a machine at load average about 30, best `p50_ms` of the three, in ms a frame: **a scene with no smoke is the same** (5 000 flames 2.23 → 2.23, 20 000 flames 4.22 → 4.27); with every torch smoking, 5 000 flames 2.23 → 3.01 and 20 000 4.22 → 7.57, with a 3 m/s wind 5 000 2.18 → 3.12, and a second quad on top of `BENCH_EMBERS=1` 3.07 → 4.03. That is about 0.16 µs a smoking fire a frame, the same class as the sparks' quad (a second entity a fire for Bevy's visibility pass and extraction), paid only by a fire that has the component, and the smoke is culled by the sparks' box when it is off the screen. On the GPU (`LIGHTS_GPU_TIMING=1`, 1280 x 720, the 3 m close-up with 52 smoking fires, four runs alternating) the transparent pass took 79.0 and 69.9 µs without smoke and 85.3 and 85.7 µs with it.

## Seeing the pool serve two viewers

`LIGHTS_FOCI=n` gives the courtyard's pool `n` foci (`FocusRule::Shared`) over the one camera, a cyan marker on each, so an overview shows who the viewers are; `LIGHTS_SPLIT=1` is two viewers with a camera each on the halves of the window (viewers a quarter of the orbit apart, each camera behind its own focus). `LIGHTS_FOCUS_RULE=one` keeps the default rule over the same foci, which leaves the pool dark: the comparison. `LIGHTS_FOCUS_WALK=8` (or 12) sends the foci out so each stands in its own part of the court.

```sh
LIGHTS_FRAMES=300 LIGHTS_SPLIT=1 LIGHTS_FOCUS_WALK=8 LIGHTS_FOCUS_RULE=one    LIGHTS_SHOT=/tmp/split_one.png    cargo run --release --example courtyard
LIGHTS_FRAMES=300 LIGHTS_SPLIT=1 LIGHTS_FOCUS_WALK=8                          LIGHTS_SHOT=/tmp/split_shared.png cargo run --release --example courtyard
magick /tmp/split_shared.png /tmp/split_one.png -compose difference -composite -evaluate Multiply 6 /tmp/split_diff.png
```

Judge each half separately (`magick f.png -crop 640x720+0+0 +repage -colorspace Gray -format "%[fx:mean*255]" info:`), and the difference image for where the pool lights are. A **moving** camera is a different test from a still one: record a sequence (`LIGHTS_SHOT_DIR`, `LIGHTS_SHOT_FROM=150`) and plot each half's mean luminance per frame; a frame that departs from the mean of its two neighbours by about a DN is a light drawn at a stale intensity (iteration 279 found one at 5.6 DN, in the frame a light entered a camera's view, and fixed it: after, none departs by more than 0.9). A static camera cannot show this. `BENCH_FOCI=n` on `bench_lights` prices the shared pool.

## Seeing a fire's own colour

`LIGHTS_COLOURS=1` gives some of the courtyard's fires a colour of their own, on the flame, its sparks and its light together: braziers 1 and 2 burn a gas-flame blue and a witch-fire green, and every fifth torch a blue or a violet; unset or `0` the courtyard is the scene's orange and draws the same pixels it always did (a capture of it differs from the previous build's by **zero** pixels, which is the check to run after any change to what a flame's colours are made of: `magick compare -metric AE old.png new.png null:`).

```sh
LIGHTS_FRAMES=300 LIGHTS_COLOURS=1 LIGHTS_SEED=7 LIGHTS_SHOT=/tmp/colours.png cargo run --release --example courtyard      # docs/media/colours.png
LIGHTS_FRAMES=300 LIGHTS_COLOURS=1 LIGHTS_EMBERS=4 LIGHTS_EYE=-5,2.6,8.4 LIGHTS_LOOK=-5,0.9,5 LIGHTS_SHOT=/tmp/blue.png cargo run --release --example courtyard   # the blue brazier from 3.5 m
```

A coloured fire is judged by whether the light it throws and the flame it is drawn on are one colour: the stone beside the brazier should lean to the flame's hue and the nearest torch-bearer's near side take it, and the rim and core of the flame should read as the orange flame's do (a darker, more saturated rim, a paler core). A pool light that changes owner between two colours changes colour while it is fading in from nothing, so no frame should depart from the mean of its neighbours by more than the pool's usual pop test in any channel: record a sequence (`LIGHTS_FOCUS_WALK=18 LIGHTS_POOL=4 LIGHTS_HYSTERESIS=1.0 LIGHTS_SHOT_DIR=... LIGHTS_SHOT_EVERY=1 LIGHTS_SHOT_FROM=100`) and plot each channel's mean per frame.

## Seeing a cross-faded hand-off

The pool's `reserve` light is what makes a hand-off cross-fade rather than dip. The example takes
`LIGHTS_RESERVE` (`0` turns it off), and its log prints the pool's total every 30 frames — which is the
number to read, because a hand-off on four lights in a 1280 x 720 frame moves the frame's mean by far
less than a DN:

```sh
LIGHTS_RESERVE=1 LIGHTS_HYSTERESIS=1.0 LIGHTS_FRAMES=150 LIGHTS_POOL=4 ./target/release/examples/courtyard
```

**Making it fire.** A cross-fade needs a slot to change owner. In the courtyard's default 6 m focus
orbit the same four torches stay nearest for the whole run, so nothing changes hands. Send the focus out
past the pool's reach instead — `LIGHTS_FOCUS_WALK=18` — and the torches stream across the boundary as
it goes round:

```sh
LIGHTS_FOCUS_WALK=18 LIGHTS_POOL_LOG=1 LIGHTS_RESERVE=1 LIGHTS_HYSTERESIS=1.0 \
LIGHTS_FRAMES=400 LIGHTS_POOL=4 LIGHTS_SEED=3 ./target/release/examples/courtyard > cross.log
LIGHTS_FOCUS_WALK=18 LIGHTS_POOL_LOG=1 LIGHTS_RESERVE=0 LIGHTS_HYSTERESIS=1.0 \
LIGHTS_FRAMES=400 LIGHTS_POOL=4 LIGHTS_SEED=3 ./target/release/examples/courtyard > dip.log
```

`LIGHTS_POOL_LOG=1` prints `pool <frame> <lights> <total>` **every frame**: a 0.35 s fade is invisible in
a 30-frame sample and far below a frame's mean luminance. The two logs differ on the frames where a
hand-off happens — at the first one the reserve run carries 12 507 lm more light than the dip, decaying
to 6 330 nine frames later as the departing light fades out.

## Seeing a hand-off

The pool hands a real light to a nearer torch when one is `hysteresis` times nearer than the torch
that holds it, and a light that does change owner fades back up over `LIGHTS_FADE` seconds. Both are
this example's own switches:

```sh
# The pop the crate used to have: nearest wins, no fade.
LIGHTS_HYSTERESIS=1.0 LIGHTS_FADE=0.0 LIGHTS_POOL=4 \
LIGHTS_FRAMES=240 LIGHTS_SHOT_DIR=/tmp/lights-handoff LIGHTS_SHOT_FROM=20 \
  ./target/release/examples/courtyard
```

Then the same run with the defaults and compare the frame pairs, **skipping the first few**: the pool
fills in on the opening frames, and that is a larger change than any hand-off. On the steady-state
tail the worst pair falls 0.697 -> 0.441 in mean luminance and the pairs over 0.45 go from two to
none, with the median unchanged. If the two runs come out *identical*, the switches did not reach the
pool: a switch that turns a feature off has to change what you are measuring.
`docs/research/pool-hand-off.md` has the craft and the measurements.

## Seeing a reshuffle — the pool's own total, every frame

A hand-off that goes wrong is not visible in a still and barely visible in a frame's mean luminance: it is
a **set** of lights being re-sorted, and the number that shows it is the pool's own total, which
`LIGHTS_POOL_LOG=1` prints every frame.

```sh
LIGHTS_FRAMES=400 LIGHTS_POOL_LOG=1 ./target/release/examples/courtyard > pool.log
```

Read it with the first 15 frames skipped (the pool spawns and the braziers kindle there, and that is a
bigger change than any hand-off):

```sh
awk '$1=="pool" && $2>=25 {print $2, $4}' pool.log   # frame, pool total
```

Two things to check, both measured on the courtyard's 400-frame default:

- **No single frame may lose a light's worth of the pool.** Before iteration 14 the total fell **19 % in one
  frame** (frame 303, 972 505 → 787 358 lm) with **8 of 13 lights reading zero** and faded back over
  twenty frames. With the set rules the worst steady-state frame moves **1.2 %** and no frame exceeds 2 %.
- **The pool's per-slot line must not show a mass extinction.** The bracket carries every light's
  intensity; `0` seven times in one frame is the reshuffle. A single `0` is a hand-off's own fade.

The frame rate in that log counts from 1 while `LIGHTS_SHOT_DIR`'s numbering starts at its `FROM`, so a
screenshot's `frame0006` is rarely the log's frame 6 — measure the log for the artefact and the images for
its look, and do not try to line the two up by index.

## Seeing a put-out fire's smoke trail

`FlameSmoke` leaves a trail when the light its flame is linked to gets a `Snuff` (`docs/research/flame-smoke-tail.md`). The courtyard's `LIGHTS_SNUFF=<seconds>` puts every brazier and torch out at that time, and `LIGHTS_SMOKE=0.8 LIGHTS_EMBERS=4` give every flame its smoke and sparks, so a sequence from a close camera is one command:

```sh
LIGHTS_FRAMES=420 LIGHTS_SNUFF=2.0 LIGHTS_SMOKE=0.8 LIGHTS_EMBERS=4 LIGHTS_EYE=2.4,1.5,7.2 LIGHTS_LOOK=5,1.6,5 \
  LIGHTS_SHOT_DIR=/tmp/trail LIGHTS_SHOT_FROM=120 LIGHTS_SHOT_EVERY=72 cargo run --release --example courtyard
```

That is frames 120, 192, 264, 336 and 408: the douse begins at frame 120 (2.0 s), the flame is out by 192 (1.2 s later), and the trail ends, exactly dark, at 5 s (frame 420). Add `LIGHTS_WIND=3,0` to see the trail bend over downwind as it lifts.

What to look for, and what to measure:

- **The flame's end is not the smoke's.** From the second frame on, the column is still there and its base has lifted off the coals; at 2.4 s it has left the bowl and at 3.6 s only its top is left. The old build's smoke was gone with the flame, at about 0.8 s.
- **No frame pops.** The mean luminance of the column's box, frame by frame (`mean |Δ|` against the frame before, in the same box), is smooth across the douse's first frame and across the trail's last: the build before this change was the same to the DN until the douse began, and the trail's own fade reaches the sky's level (4.15 DN here) at about 4.8 s, a little before its clock stops.
- **A scene that puts nothing out is unchanged.** The default courtyard and the close camera with `LIGHTS_SMOKE` and `LIGHTS_WIND` are zero pixels different from the build before (`magick compare -metric AE`).
- **From above it is a point.** A camera looking straight down sees a vertical column end-on, so the top-down view shows nothing of the trail, as it shows nothing of the steady plume; the 3x crop of a brazier at the default 45° camera shows a thin warm wisp rising from the coals.

## Seeing a wide fire's tongues

`examples/campfire.rs` puts a campfire (0.8 m bed, 0.7 m flame) at x = -2.6 and a bonfire (3 m pyre, 2 m flame) at x = 3.4, each a `FireKind` with its light, flame, sparks, smoke and `FlameTongues` (`docs/research/flame-tongues.md`). `LIGHTS_TONGUES=0` draws the same two fires as one flame each, which is the comparison, and every frame below is `LIGHTS_FRAMES=240`:

```sh
# the campfire from 2.5 m, the bonfire from 5 m, and the same two without tongues
for t in off on; do
  E=""; [ $t = off ] && E="LIGHTS_TONGUES=0"
  env $E LIGHTS_FRAMES=240 LIGHTS_EYE=-0.1,1.1,2.3 LIGHTS_LOOK=-2.6,0.55,0 LIGHTS_SHOT=/tmp/${t}_camp.png cargo run --release --example campfire
  env $E LIGHTS_FRAMES=240 LIGHTS_EYE=1.4,1.5,5.0 LIGHTS_LOOK=3.4,1.0,-0.6 LIGHTS_SHOT=/tmp/${t}_bon.png cargo run --release --example campfire
done
# a sequence: 30 frames, one every 3 (0.05 s), from frame 60
LIGHTS_FRAMES=150 LIGHTS_SHOT_DIR=/tmp/camp LIGHTS_SHOT_FROM=60 LIGHTS_SHOT_EVERY=3 \
  LIGHTS_EYE=-0.1,1.1,2.3 LIGHTS_LOOK=-2.6,0.55,0 cargo run --release --example campfire
```

`docs/media/tongues.png` is those four frames (campfire and bonfire, off and on), each cropped to the fire and the captions burned in. `LIGHTS_WIND=1.5,0` shows the tongues laid over together (the inward lean adds to the wind's), `LIGHTS_SNUFF=1.0` puts both out and every tongue goes with its light, `LIGHTS_TONGUES=n` gives both fires `n` tongues over their kind's spread (a campfire given nine is as wide as its stones), and `LIGHTS_SPREAD=m` sets the width the tongues' feet are spread over.

What to look for, and what to measure:

- **A mound, not a row.** The first build laid the tongues parallel, which read as a crown of candles (bonfire) and a handful of separate flames (campfire); the inward lean (`INWARD_LEAN`, 0.30 at the rim) turns them into one fire that converges on a tip. A frame sequence shows the tongues parting and rejoining on clocks of their own.
- **A scene that does not use the component is unchanged.** The default courtyard in ten views (ground, edge, top and graze, two seeds with a pool of 4 and of 16, and three far views at 45 and 95 m, one with sparks and smoke) against the build before: seven are zero pixels different and three have one or two pixels off by 1 DN (the GPU's own noise, as in iteration 282); counted with numpy on the two PNGs.
- **What it costs.** CPU: `BENCH_TONGUES=6 cargo run --release --example bench_flames` (5 000 fires of 6 tongues, 30 000 flames in the world, 88 in view) reads 7.1 to 7.4 ms a frame against 2.6 to 2.9 for 5 000 plain flames: the 25 000 extra flames cost about 0.18 µs each a frame, so a campfire's five extra tongues are about 0.9 µs. GPU: `LIGHTS_GPU_TIMING=1 LIGHTS_FRAMES=420` with the bonfire 5 m from the camera (both fires in view, a third of the screen) reads the transparent pass at 340 and 302 µs without tongues and 504 and 436 µs with them (two rounds, interleaved), the opaque pass unchanged: about 0.15 ms a frame for the two fires, 13 draws more.

## Seeing a bed of coals live

`examples/campfire.rs` lays each of its two fires on its kind's `FuelBed` (`FireKind::bed()`: a campfire's 0.7 m, a bonfire's 1.9 m; `docs/research/fuel-bed-heat.md`). `LIGHTS_BED=0` leaves the coals out (the comparison) and `LIGHTS_COOLING=s` sets their cooling time constant, so a fire put out (`LIGHTS_SNUFF`) can be captured cooling in seconds instead of minutes:

```sh
# the campfire from straight above, with and without coals, and a frame every 75 frames of a fire put out at 1.0 s
E="LIGHTS_FRAMES=240 LIGHTS_EYE=-2.6,2.4,0.5 LIGHTS_LOOK=-2.6,0,0"
env $E LIGHTS_SHOT=/tmp/bed_on.png cargo run --release --example campfire
env $E LIGHTS_BED=0 LIGHTS_SHOT=/tmp/bed_off.png cargo run --release --example campfire
LIGHTS_FRAMES=1050 LIGHTS_SHOT_DIR=/tmp/bed_snuff LIGHTS_SHOT_FROM=30 LIGHTS_SHOT_EVERY=75 \
  LIGHTS_EYE=-2.6,2.4,0.5 LIGHTS_LOOK=-2.6,0,0 LIGHTS_SNUFF=1.0 LIGHTS_COOLING=6 cargo run --release --example campfire
```

`docs/media/fuel-bed.png` is frames 0, 1, 2 and 4 of that last run (frames 30, 105, 180 and 330), each cropped to 420 x 330 round the bed with the captions burned in: `magick frameNNNN.png -crop 420x330+430+180 +repage -gravity south -background black -fill white -pointsize 18 -splice 0x34 -annotate +0+8 "caption"`, then `+append` and `-append` into a 2 x 2 sheet (a truecolor PNG: 128 colours bands the dark ground).

What to look for, and what to measure:

- **A bed read against dark.** The first build was an additive glow over the ground the same fire had lit, and read as a cloud with nothing dark between the lumps; the disc is an opaque cut-out with its own ash (`ASH` in `src/bed.wesl`), so the lumps are patches of orange and deep red on near-black, with a ragged rim, not a plate. Judge it from 2.4 m straight down, at 45°, and from 1.2 m; at the viewer's eye and at a graze the logs and the stones cover most of it, as they would.
- **It goes on glowing.** The flame and the light are gone 1.2 s after the douse (`SNUFF_SECONDS`) and the coals stay, orange then red then nothing, with the logs dark against them: the strongest frame of the set is the one 2 s after the douse (frame 2 above). At the default 60 s time constant the hottest coal is under a thousandth of its burning brightness after 104 s (`bed_visible_heat`, 0.18 of the heat).
- **It breathes.** 61 consecutive frames of a burning bed (`LIGHTS_SHOT_DIR` with `LIGHTS_SHOT_EVERY=1 LIGHTS_SHOT_FROM=100`), the mean grey level of a 40 x 50 patch of bed with no flame over it: 110 to 128 (a standard deviation of 5, about 7 % of its level), rising with the flame's own swing over the same frames.
- **A scene that does not use the component is unchanged.** The default courtyard in ten views (default, ground, edge, top, graze, seed 3 with a pool of 4 and of 16, far at 45 and 95 m, and the close camera with sparks and smoke, 300 frames, `magick compare -metric AE`) against the build before: eight are zero pixels different and two (seed 3 pool 16, the sparks and smoke view) are one pixel off by 1 DN, the GPU's own noise. The courtyard does not use `FuelBed`.
- **What it costs.** CPU, interleaved against the build before it on a machine at load average 14-20: a scene with no bed is the same (`bench_lights`, seven alternating runs each: `min_us` 328-378 against 343-385, `p50_us` 534-638 against 553-642). `BENCH_BEDS=1 cargo run --release --example bench_flames` (every torch of 5 000 has a bed, 15 in view; needs a GPU adapter) reads 3.5 ms a frame at best against 2.4 without, about 0.2 µs a bed, and 6.0 and 6.8 in the other two runs of three on the same load; a bed is an entity like a flame, and `update_beds` alone is about 0.06 µs a bed a frame in a headless app (5 000 beds, 300 frames). GPU: `LIGHTS_GPU_TIMING=1 LIGHTS_FRAMES=420` with the bonfire 5 m from the camera reads the opaque pass at 303 and 280 µs before and 333 and 292 µs with two beds (two rounds, interleaved), the transparent pass unchanged within the noise (426 and 392 against 435 and 394): 12 to 30 µs for two beds. A host that never asks for a bed builds nothing: the colour table and the coal field (a 128 x 128 tile, about 40 ms in a release build) are made the first time a site has a `FuelBed`.

## Seeing a whole fire (`Fire`)

`examples/fires.rs` is one `Fire` on an empty entity and a one-light pool; `LIGHTS_KIND` names the kind, and the camera stands a fixed number of flame heights back so a candle and a bonfire fill the frame alike. `docs/media/fires.png` is six runs and a montage:

```sh
cd /path/to/bevy_lights
for k in candle lantern torch brazier campfire bonfire; do
  LIGHTS_FRAMES=240 LIGHTS_KIND=$k LIGHTS_SHOT=/tmp/$k.png cargo run --release --example fires
done
# six panels at 520 x 293 with the names under them, then an 8-bit PNG (16-bit is 2.7 MB; 128 colours bands the dark ground)
magick montage -font DejaVu-Sans -pointsize 20 -fill white -background '#050810' -label '%t' \
  /tmp/{candle,lantern,torch,brazier,campfire,bonfire}.png -tile 3x2 -geometry 520x293+3+3 /tmp/fires16.png
magick /tmp/fires16.png -depth 8 -define png:compression-level=9 PNG24:docs/media/fires.png
```

Other switches: `LIGHTS_COLOUR=0.1,0.32,1.0` (linear) is a blue fire (light, flame, sparks and smoke in one hue), `LIGHTS_SMOKE=0`, `LIGHTS_EMBERS=0`, `LIGHTS_TONGUES=0` and `LIGHTS_BED=0` are the `Fire` flags, `LIGHTS_ROW=12` lays twelve side by side (a frame sequence shows them breathing apart: each has its own phase), and `LIGHTS_SNUFF=1.0 LIGHTS_COOLING=6 LIGHTS_KIND=campfire` puts the fire out and leaves its coals cooling, the light, the flame, the smoke's trail and the bed all following the one entity.

What was checked, and how:

- **The build before a `Fire` is the build after.** `examples/campfire.rs` wrote each fire's `PointLight`, `Flicker` and `FlameSite` by hand and is now one `Fire` per fire on a two-light pool (the template's colour dropped, since a `Fire` brings its own). Six still-air views of it (the default, `campfire` and `bonfire` close, from above, the tongue-less comparison, a fire put out) against the old binary at frame 240: **at most 2 DN** anywhere, 0.0017 to 0.017 DN mean, and a fire put out 1 DN over 3 039 pixels. The 2 DN is on the ground and was not isolated; the likely cause is the pool template's one range for both fires (27 m) where the hand-written campfire light had 15 m. **In a wind they differ** (`LIGHTS_WIND=1.5,0`: 430 040 pixels, mean 2.2 DN, max 237): the pooled light leans with its flame (`docs/research/flame-lean-light.md`) and the hand-written one did not, which is the intended difference, though it was not isolated either.
- **A scene that does not use `Fire` is unchanged.** The default courtyard in eight views (default, ground, edge, top, graze, seed 3 with pools of 4 and 16, and the close camera with sparks and smoke, 300 frames, `magick compare`/numpy count) against the old binary: six zero pixels, edge one pixel and close two pixels off by 1 DN (the GPU's own noise).
- **The first capture found a white pool.** `Fire { color: None }` first left the candidate's colour to the pool template, whose default is white, and the six kinds were orange flames over white ground. A `Fire` with no colour now throws a `FIRE_KELVIN` (1900 K) light; the unit test asserts it for every kind.
- **Known limits.** The coals stay coal-coloured under a coloured flame (a green fire on orange coals): turn the bed off for a magic fire. A `Fire` is pooled: a host with no `LightPoolPlugin` gets the flame and a logged error, not a light.
- **What it costs.** `BENCH_FIRE=1` against `BENCH_FIRE=2` (the same parts written by hand), 5 000 torches, 15 in view, four rounds interleaved, `p50_ms` 3.89 to 4.65 against 3.78 to 3.97 (and one by-hand outlier round at 7.87, min 3.96, on a loaded machine) and `min_ms` 2.82 to 3.11 against 2.79 to 3.01: the component's own cost is at most a tenth of a millisecond at 5 000 fires (about 0.02 µs a fire a frame, inside the noise), and the work a fire does (the linked flame, the sparks, the smoke) is the same either way. `bench_lights` base against new, three rounds, `min_us` 405, 404, 325 against 322, 335, 325, `p50_us` 687, 729, 509 against 558, 713, 542: the same (nothing it runs was touched); `bench_flames` with no `Fire` and with sparks and smoke, base against new, p50 2.47 to 3.28 against 2.40 to 3.96 and 4.30 to 6.16 against 4.68 to 6.12 over three rounds at load average 14: the same.

## Seeing a fire that moves (`FlameMotion`)

`examples/fires.rs` carries its fire with `LIGHTS_WALK=v` (metres per second along +X) for `LIGHTS_WALK_FOR=s` seconds (default 1.5), centred on where it was placed, and then stands it still; the camera stands back 3.5 m. A frame sequence shows the flame lie back while it is carried and stand up again after (`docs/media/carried.png` is six frames of it):

```sh
LIGHTS_FRAMES=190 LIGHTS_KIND=torch LIGHTS_WALK=1.4 LIGHTS_WALK_FOR=1.5 LIGHTS_EYE=0,0.65,3.2 LIGHTS_LOOK=0,0.3,0 \
  LIGHTS_SHOT_DIR=$HOME/walk LIGHTS_SHOT_FROM=20 LIGHTS_SHOT_EVERY=10 ./target/release/examples/fires
```

Measured on those frames (the principal axis of the flame's bright, warm pixels against the vertical): 57° at frame 50 and 50° at frame 90 while carried at 1.4 m/s (`flame_tilt(1.4, 0.1)` is 54.7°), 20° a third of a second after the stop and under 10° from half a second on (the filter's 0.15 s is `exp(-t / 0.15)`: 0.15 m/s left at 0.33 s, which `flame_tilt` lays over 22°). On the courtyard `LIGHTS_MOTION=0` is the courtyard as it was before a fire's own motion counted, and it is **zero pixels different** from the build before in eight views (default, ground, edge, mid, top, graze, seed 3 with pools of 4 and 16, 300 frames each; one pixel off by 1 DN in two of them, the GPU's own noise): a fire that stands still, or a gain of zero, is the picture it was. With the gain at its default the braziers (which stand still) are unchanged and the walking torch-bearers' flames and the light and shadows they throw differ.

**The hand-off is not disturbed by it**: 141 consecutive courtyard frames (`LIGHTS_SEED=3 LIGHTS_POOL=4 LIGHTS_FOCUS_WALK=18 LIGHTS_HYSTERESIS=1.0`, frames 100 to 240), each frame's mean red-green-blue departure from the average of its two neighbours, whole frame and each half: at most 0.20 DN with the lean and 0.19 before, 99th percentile 0.08 both (a pop is a DN or more).

**What it costs** (CPU, interleaved against the build before it on a machine at load average 19-34): `bench_lights` (no `FlameSite`, so nothing it runs was touched) `min_us` 337-348 against 342-360 and `p50_us` 527-543 against 533-569 over three alternating runs each; `bench_flames` with 5 000 torches that stand still, `p50_ms` 2.38-2.55 against 2.30-2.50 (a fire at rest costs the tracker one position compare, about 0.02 µs a frame); with all 5 000 walking (`BENCH_MOVING=1.4`) the lean itself (`BENCH_MOTION=1` against `0`) costs up to 0.3 ms at 5 000 moving fires, about 0.06 µs a fire, in the noise of the load.

## The still

```sh
LIGHTS_FRAMES=600 LIGHTS_SHOT=docs/media/courtyard.png cargo run --release --example courtyard
```

`LIGHTS_EMBERS=r` gives every flame `r` sparks a second (`FlameEmbers`; unset, none). `LIGHTS_SMOKE=d` gives every flame `FlameSmoke { density: d }` (unset, none; `0.8` is the default density; add `LIGHTS_WIND=3,0` to see the column bend, and a close camera, `LIGHTS_EYE=2.4,1.5,7.2 LIGHTS_LOOK=5,1.6,5`, to see it: from the default 45° camera it is foreshortened to a thin wisp). `LIGHTS_GPU_TIMING=1` with `LIGHTS_FRAMES=420` prints the mean GPU microseconds per render pass (`gpu_us <pass> <mean>`, frames 100 to 400; needs Vulkan or DX12 timestamp queries). `LIGHTS_EYE` and `LIGHTS_LOOK` move the camera (`x,y,z`). The default eye is `(0, 9, 16)` looking at
the origin; the GIF's angle is the default.

## The GIF

```sh
FONT=/System/Library/Fonts/Supplemental/Arial.ttf \
BOLD="/System/Library/Fonts/Supplemental/Arial Bold.ttf" \
FPS=30 WIDTH=640 \
  tools/gif.sh /tmp/lights-frames docs/media/courtyard.gif \
  "Four braziers, 48 torches, an 8-light pool"
```

`docs/media/courtyard.gif` is the showcase's capture at 640 px: the showcase recipe's step 1, then frames 1 to 48 of the
sequence (frame 0 is black: the pipelines are still compiling), renumbered from 0 and encoded with `WIDTH=640`. 48 frames,
3.2 MB. `docs/media/courtyard.png` is the still above. Both were re-taken on 2026-10-28 with the bowl braziers and the
ember bed (the earlier files showed box braziers and a pink bed), and on 2026-10-30 with every figure casting a shadow,
the pool's three shadowed lights and the 20 cd/m² sky fill; that capture, and the showcase and relight clips, used
`LIGHTS_EMBERS=4`.

**No default font path, deliberately.** `/System/Library/Fonts/...` exists on exactly one of the
machines this repo is built on and fails on the others — and it fails in `magick`, after ffmpeg has
already spent the whole two-pass encode. Name both files at the call site. The paths above are
macOS's; on Linux use Liberation Sans, which is metrically compatible, so substituting one for the
other does not move the caption:

```sh
FONT=/usr/share/fonts/liberation/LiberationSans-Regular.ttf \
BOLD=/usr/share/fonts/liberation/LiberationSans-Bold.ttf \
  tools/gif.sh ...
```

The `truetype/liberation/` path some distributions use does not exist on big, and
`gif.sh` fails in `magick` after the whole encode, exactly as the macOS path does. Check the file is
there before starting: `ls /usr/share/fonts/*/LiberationSans-Regular.ttf`.

`STRIDE=2` with `FPS` left alone halves the file and plays at half speed; pair it with a doubled `FPS`
to keep real-time speed. A GIF stores whole frames, so its size is nearly linear in frame count.
