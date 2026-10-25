# Captures and demos

How `docs/media/` is made, so two captures a month apart are comparable. Every capture is taken from
the same build, the same frame count and the same camera as the one before it.

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
