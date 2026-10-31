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

## The still

```sh
LIGHTS_FRAMES=600 LIGHTS_SHOT=docs/media/courtyard.png cargo run --release --example courtyard
```

`LIGHTS_EMBERS=r` gives every flame `r` sparks a second (`FlameEmbers`; unset, none). `LIGHTS_GPU_TIMING=1` with `LIGHTS_FRAMES=420` prints the mean GPU microseconds per render pass (`gpu_us <pass> <mean>`, frames 100 to 400; needs Vulkan or DX12 timestamp queries). `LIGHTS_EYE` and `LIGHTS_LOOK` move the camera (`x,y,z`). The default eye is `(0, 9, 16)` looking at
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
