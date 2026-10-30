# Flame gusts: wind that is never steady

`FlameWind` (iteration 196) leans every flame downwind, but the host has to write its velocity each frame, and a constant velocity is a flame frozen at an angle: the picture reads as a fan, not weather. Outdoor wind is a mean plus a turbulent fluctuation. This note adds the fluctuation as a deterministic function of time, so a host that knows only its mean wind (or nothing but "it is breezy") gets a living wind in one line.

## Sources

| stem / location | kind | what it gives |
|---|---|---|
| `10.7456_tojdac.1674124` (Doğaner 2025, *The narrational role of lighting design in The Lights of Dark City*) | craft, in the corpus | what a wavering flame is *for* in a frame |
| Hannesdóttir, Kelly & Dimitrov (2019), *Extreme wind fluctuations: joint statistics, extreme turbulence, and impact on wind turbine loads*, `10.5194/wes-4-325-2019` (open access, read at https://wes.copernicus.org/articles/4/325/2019/; `paper_download` fetched the PDF, `scribe_convert` not run) | technical | turbulent velocity fluctuation is **Gaussian, stationary**; turbulence intensity is defined against the mean and classed (IEC class C: `I_ref` 0.12); coherent *lateral* fluctuation accompanies speed ramps |
| Mann (1994, 1998), via the same paper | technical, **not read** | the IEC's spectral turbulence model; the shape this note replaces with a one-pole spectrum |

No shipping-game or production source on procedural gusting is in the corpus (`distill_search` "wind gust turbulence spectrum von Karman Davenport turbulence intensity" returned only an evapotranspiration paper; `paper_search` found no games paper whose PDF could be downloaded: Wind turbulence modeling for real-time simulation, `10.1007/s13540-023-00165-0`, downloaded but is a flight-simulation fractional-order model, not read). Recorded under `## Pending index`.

## Craft principle

Doğaner:

> "Flickering light, created by the reflection of water or flickering light by the flicker of a flame, is often used to depict the fragility of perception of reality and mood swings."

A flame that leans at a constant angle is a stuck image; one that leans, eases, and leans again tells the viewer the air is alive. A gaffer's fan is never run steady for the same reason. What the viewer needs is **slow**, **correlated** motion: a gust that builds over a second or two, not a jitter at the flicker's rate. The flame's own puff is 1–3 Hz (`flicker_rate`); a gust has to live well below that or the two blur into one shimmer.

## How shipping games do it

Not cited (see Sources). The crate's own practice: the wind is a host-owned resource and the flames read it; this note adds one more *writer* of that resource, not a second path into the shader.

## Model adopted

Hannesdóttir et al.:

> "Gaussian turbulent velocity component fluctuations are synthesized via the “Mann model” spectra and assumed to be stationary and homogeneous"

> "*I*ref is the reference turbulence intensity (TI) at 15 m s−1"

So the wind speed is `U(t) = Ū · (1 + I · x(t))` with `x` a zero-mean, unit-variance, stationary Gaussian process and `I` the turbulence intensity (`σ_U / Ū`). `x` is built **statelessly**, a function of time and a seed only: a sum of `N` sinusoids with log-spaced frequencies, random (hashed) phases, and amplitudes drawn from a one-pole (Lorentzian, Ornstein–Uhlenbeck) spectrum `S(f) ∝ 1 / (1 + (2π f T)²)` with `T` the gust's correlation time. Being a pure function of `t`, it is deterministic across machines and frames, needs no per-frame state, loops nowhere inside a session, and a host can sample it at any `t` (a preview, a test, a pause). The speed is clamped at zero. The wind's heading wanders by `I · y(t)` radians, with `y` an independent draw of the same process (design default: for small angles the lateral standard deviation is `σ_v ≈ σ_U`, so the heading's standard deviation is `σ_v / Ū ≈ I`; the paper reports lateral coherence with ramps but gives no number).

Hosts own the mean wind (weather is `bevy_elements`'). A `FlameGust { mean, intensity, timescale, seed }` resource makes `FlamePlugin` write `FlameWind::velocity` each frame, on `Time<Real>` like the flames' own clock. Without a `FlameGust`, `FlameWind` is untouched: a host that drives its own wind still can.

## Parameters

| parameter | value | source |
|---|---|---|
| `intensity` default | 0.25 | design default. IEC class C is 0.12 at hub height over open water (Hannesdóttir et al., quoted above); ground level among walls is rougher, and a lean is only readable above about 0.2. `I = 0.12` is available and reads as a faint sway |
| `timescale` default | 2.0 s | design default: slow against the flame's 1–3 Hz puff (`flicker_rate`), fast enough to see a gust build and drop within a few seconds. The paper's 120 m turbulence length scale at 100 m height over a 10 m/s mean is 12 s; near the ground the eddies are smaller, so a shorter time is used |
| `N` (sinusoids) | 12, frequencies `1 / (2π T) · 0.125 · √2^k`, k = 0..11 (0.125 to 5.7 times the corner) | design default: spans the one-pole spectrum's knee; beyond 5.7 corners it holds about 3 % of the power, and cutting it keeps a frame step under 0.1 sigma |
| speed floor | 0 | a gust cannot blow upwind of the mean's heading by speed; the heading wander handles direction |
| heading wander | `I · y(t)` rad | design default (see above) |
| the unit process's variance | normalised to exactly 1 over the discrete amplitudes | construction |

## What great looks like

A row of braziers in a mean 2 m/s wind: every flame leans the same way, all together, leaning more at a gust and easing back, never twitching at the flame's own flicker rate, and sometimes swinging a few degrees off the mean heading. Two seeds give two different but equally plausible nights. With no `FlameGust` the picture is the one of iteration 196.

## Validation targets

- The unit process: mean over a long window ~ 0 (|mean| < 0.15) and variance ~ 1 (0.7 to 1.3) over 1000 s at 60 Hz, for several seeds; deterministic (same `t`, same seed, same value); different seeds differ.
- Smoothness: the largest frame-to-frame change at 60 Hz with `T = 2 s` is under 0.1σ, so the lean never jitters.
- The correlation time matches `T`: the autocorrelation at lag `T` is within 0.2 of `e⁻¹`.
- `intensity = 0` gives exactly the mean for every `t`; speed is never negative; the heading offset has zero mean and standard deviation near `I`.
- Plugin: with a `FlameGust` the `FlameWind` follows it and the flames' lean changes frame to frame; without one, a hand-written `FlameWind` is left alone.
- Capture: `courtyard` with `LIGHTS_GUST`, a frame sequence: the tip's horizontal offset varies smoothly over seconds, flames stay rooted and unclipped.

## Deviations

- The spectrum is one-pole, not Mann's: simpler, and the flame cannot show the difference. Design default; the cited paper supplies the Gaussian assumption and the intensity's meaning, not the spectral shape.
- `intensity` and `timescale` defaults are design defaults, not site data.
- No spatial coherence: every flame sees the same wind at the same instant. A real gust crosses a courtyard; at the scale of a courtyard (tens of metres, the gust's seconds) the delay is a fraction of a second, below what the flame's own flicker hides.

## Pending index

- Mann (1994, 1998) and a shipping-game source on procedural gusts: not in the corpus, not downloaded.
- `10.1007/s13540-023-00165-0` is downloaded but unread and unindexed.
- Hannesdóttir et al. 2019 was read from the publisher's HTML, not converted; `distill_index` of its stem is pending a `scribe_convert`.
