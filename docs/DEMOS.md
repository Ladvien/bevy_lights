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

## The still

```sh
LIGHTS_FRAMES=600 LIGHTS_SHOT=docs/media/courtyard.png cargo run --release --example courtyard
```

`LIGHTS_EYE` and `LIGHTS_LOOK` move the camera (`x,y,z`). The default eye is `(0, 9, 16)` looking at
the origin; the GIF's angle is the default.

## The GIF

```sh
FONT=/System/Library/Fonts/Supplemental/Arial.ttf \
BOLD="/System/Library/Fonts/Supplemental/Arial Bold.ttf" \
FPS=30 WIDTH=640 \
  tools/gif.sh /tmp/lights-frames docs/media/courtyard.gif \
  "Four braziers, 48 torches, an 8-light pool"
```

**No default font path, deliberately.** `/System/Library/Fonts/...` exists on exactly one of the
machines this repo is built on and fails on the others — and it fails in `magick`, after ffmpeg has
already spent the whole two-pass encode. Name both files at the call site. The paths above are
macOS's; on Linux use Liberation Sans, which is metrically compatible, so substituting one for the
other does not move the caption:

```sh
FONT=/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf \
BOLD=/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf \
  tools/gif.sh ...
```

`STRIDE=2` with `FPS` left alone halves the file and plays at half speed; pair it with a doubled `FPS`
to keep real-time speed. A GIF stores whole frames, so its size is nearly linear in frame count.
