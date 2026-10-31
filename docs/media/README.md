# `docs/media/` — every image, with the command that made it

Every file in this folder, what produced it, and the release it shows. The capture recipes and the traps
behind them are in [`docs/DEMOS.md`](../DEMOS.md); the frame sequences and encodings are not repeated
here. Nothing in this folder is in Git LFS, and every reference to it from a markdown file is a relative
path, so crates.io rewrites it to this repository and the pictures show on the crate's page too.

The crate version below is the release the capture was committed for (`git describe --tags --abbrev=0`
of the commit that added it); `0.5.0` is the current release, and where a file is older than the code
around it the table says so.

| file | what it shows | command that regenerates it | captured | shows |
|---|---|---|---|---|
| `showcase.gif` | the hero: four brazier lights, 48 torch-bearers sharing an 8-light pool, flames leaning behind the walkers | `LIGHTS_FRAMES=260 LIGHTS_SHOT_DIR=/tmp/show LIGHTS_SHOT_FROM=9 LIGHTS_SHOT_EVERY=2 LIGHTS_EMBERS=4 LIGHTS_FOCUS_WALK=18 cargo run --release --example courtyard`, then `FPS=30 WIDTH=400 tools/gif.sh /tmp/show/short docs/media/showcase.gif "<caption>"` (`docs/DEMOS.md`, "The README showcase") | 2026-10-31 | 0.5.0 |
| `relight.gif` | the same courtyard snuffed and re-lit: light and flame fall away together, then catch together | `LIGHTS_FRAMES=260 LIGHTS_SHOT_DIR=/tmp/relight LIGHTS_SNUFF=1.0 LIGHTS_RELIGHT=2.3 LIGHTS_EMBERS=4 cargo run --release --example courtyard`, then `tools/gif.sh` at `FPS=30 WIDTH=400` (`docs/DEMOS.md`, "The relight clip") | 2026-10-30 | 0.5.0 |
| `courtyard.gif` | the courtyard at 640 px, the walkers' flames leaning behind them | the showcase recipe at `WIDTH=640`, frames 1-48 of the sequence (`docs/DEMOS.md`, "The GIF") | 2026-10-31 | 0.5.0 |
| `courtyard.png` | the still: the courtyard at 1280 × 720, frame 600 | `LIGHTS_FRAMES=600 LIGHTS_EMBERS=4 LIGHTS_SHOT=docs/media/courtyard.png cargo run --release --example courtyard` (`docs/DEMOS.md`, "The still") | 2026-10-31 | 0.5.0 |
| `fires.png` | the six fire kinds, each one `Fire` component, with its flame, its pool on the ground and its own sparks, smoke, tongues and coals | six `LIGHTS_FRAMES=240 LIGHTS_KIND=<kind> LIGHTS_SHOT=/tmp/<kind>.png cargo run --release --example fires` runs, then `magick montage … -tile 3x2 -geometry 520x293` and `magick … PNG24:docs/media/fires.png` (`docs/DEMOS.md`, "Seeing a whole fire") | 2026-10-31 | 0.5.0 |
| `colours.png` | two coloured braziers (a gas blue, a witch green) among the orange ones, and blue and violet torches | `LIGHTS_FRAMES=300 LIGHTS_COLOURS=1 LIGHTS_SEED=7 LIGHTS_SHOT=docs/media/colours.png cargo run --release --example courtyard` (`docs/DEMOS.md`, "Seeing a fire's own colour") | 2026-10-31 | 0.4.0 (the feature shipped in 0.4.0; the file was taken on its commit, which `v0.5.0` follows) |
| `tongues.png` | a campfire and a bonfire as one flame each, then as a mound of `FlameTongues` | the four stills in `docs/DEMOS.md` ("Seeing a wide fire's tongues"), cropped and captioned | 2026-10-31 | 0.5.0 |
| `smoke.png` | a brazier's lit smoke plume from 3 m, still air and a 3 m/s wind | `LIGHTS_FRAMES=300 LIGHTS_SMOKE=0.8 LIGHTS_EMBERS=4` (and the same with `LIGHTS_WIND=3,0`) on `examples/courtyard`, the close camera `LIGHTS_EYE=2.4,1.5,7.2 LIGHTS_LOOK=5,1.6,5`, each panel a 680 × 680 crop (`docs/DEMOS.md`, "Seeing a put-out fire's smoke trail" recipe, frames 300) | 2026-10-31 | 0.5.0 |
| `smoke-tail.png` | a fire put out with `Snuff`: the flame gone, the plume bent over and lifting off, the last thin remains | `LIGHTS_FRAMES=420 LIGHTS_SNUFF=2.0 LIGHTS_SMOKE=0.8 LIGHTS_EMBERS=4 LIGHTS_WIND=3,0 LIGHTS_EYE=2.4,1.5,7.2 LIGHTS_LOOK=5,1.6,5 LIGHTS_SHOT_DIR=/tmp/trail LIGHTS_SHOT_FROM=120 LIGHTS_SHOT_EVERY=72 cargo run --release --example courtyard`, then the four frames cropped to 640 × 360 (`docs/DEMOS.md`, "Seeing a put-out fire's smoke trail") | 2026-10-31 | 0.5.0 |
| `fuel-bed.png` | a campfire's coals from 2.4 m straight down: burning, 0.75 s after the douse, 2.0 s, 4.5 s | `LIGHTS_FRAMES=1050 LIGHTS_SHOT_DIR=/tmp/bed_snuff LIGHTS_SHOT_FROM=30 LIGHTS_SHOT_EVERY=75 LIGHTS_EYE=-2.6,2.4,0.5 LIGHTS_LOOK=-2.6,0,0 LIGHTS_SNUFF=1.0 LIGHTS_COOLING=6 cargo run --release --example campfire`, then the 2 × 2 sheet of 420 × 330 crops (`docs/DEMOS.md`, "Seeing a bed of coals live") | 2026-10-31 | 0.5.0 |
| `depth-fade.gif` | a torch behind a low wall: its flame goes out while the stone hides its source, and returns past the wall's end | `LIGHTS_FRAMES=272 LIGHTS_PAN=1 LIGHTS_DEPTH_FADE=1 LIGHTS_SHOT_DIR=/tmp/fade LIGHTS_SHOT_FROM=1 LIGHTS_SHOT_EVERY=2 cargo run --release --example occlusion` on big under Xvfb, then `FONT=/usr/share/fonts/liberation/LiberationSans-Regular.ttf BOLD=/usr/share/fonts/liberation/LiberationSans-Bold.ttf FPS=15 STRIDE=2 WIDTH=640 tools/gif.sh /tmp/fade docs/media/depth-fade.gif "<caption>"` (the first 8 saved frames are black: the pipeline is still compiling) | 2026-10-31 | 0.5.0 |
| `depth-fade.png` | the same torch with `LIGHTS_DEPTH_FADE=0` beside `=1`, cropped: a flame over the wall against no flame | `LIGHTS_FRAMES=200 LIGHTS_DEPTH_FADE=<0\|1> LIGHTS_EYE=-2.2,1.9,4.2 LIGHTS_LOOK=-2.5,1.9,-1.2 LIGHTS_SHOT=/tmp/depthfade_<0\|1>.png cargo run --release --example occlusion`, then `magick … -crop 360x230+470+250 -resize 200%` and `+append` with the captions | 2026-10-31 | 0.5.0 |
| `carried.png` | a torch carried at 1.4 m/s and then stopped, in six frames: the flame and its smoke streaming behind, then standing up | `LIGHTS_FRAMES=190 LIGHTS_KIND=torch LIGHTS_WALK=1.4 LIGHTS_WALK_FOR=1.5 LIGHTS_EYE=0,0.65,3.2 LIGHTS_LOOK=0,0.3,0 LIGHTS_SHOT_DIR=$HOME/walk LIGHTS_SHOT_FROM=20 LIGHTS_SHOT_EVERY=10 ./target/release/examples/fires`, then six 420 px crops (`docs/DEMOS.md`, "Seeing a fire that moves") | 2026-10-31 | 0.5.0 |

Two notes on the table:

- **`colours.png` was captured before the 0.5.0 release** and shows a scene the 0.5.0 build draws the
  same way: a `Fire`'s colour, `PoolCandidate::color` and the retinted ramp did not change in 0.5.0.
  Every other file was taken from the 0.5.0 tree.
- **`relight.gif` and `courtyard.gif` were re-encoded from captures of the tree that became 0.5.0**, not
  from the tagged commit, so a byte-for-byte rerun of their recipe on `v0.5.0` may differ in the GPU's own
  noise (the repository's captures are reproducible to a pixel or two, not exactly).

A capture is a run of an example with `LIGHTS_SHOT`/`LIGHTS_SHOT_DIR` set; both are read by
`examples/common/mod.rs`. `LIGHTS_*` switches are documented per example, and every number quoted in a
caption in the README was measured on the file it sits under.
