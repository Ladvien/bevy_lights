//! [`FireKind`]: the numbers a fire is made of, by name.

use bevy::prelude::*;

use crate::model::{
    CALM_FLICKER_DEPTH, FLICKER_DEPTH, WHIPPING_FLICKER_DEPTH, fire_lumens, flame_light_offset, flicker_rate,
};
use crate::{
    FlameEmbers, FlameSite, FlameSmoke, FlameTongues, Flicker, FuelBed, MAX_EMBER_RATE, PoolCandidate,
};

/// A named kind of fire: its fuel-bed diameter, flicker depth and flame height together, so a
/// picker (or a project file) stores one word instead of three loose numbers.
///
/// The diameter sizes the flicker through the puffing law, the depth is how far the fire dips, and
/// the height is the drawn flame's. [`flicker`](Self::flicker) and [`site`](Self::site) build the
/// light's and the flame's settings **from the same three numbers**, so the two breathe together.
/// Only the brazier row is a number the crate shipped before; the rest are design defaults
/// (`docs/research/fire-kinds.md`). Spell the rest of a fire with struct update:
/// `FlameSite { phase: 1.3, ..FireKind::Torch.site() }`. A kind carries no colour (all six are soot-incandescent
/// fuels, the scene's orange): a fire of another hue sets [`FlameSite::color`] and [`PoolCandidate::color`] the
/// same way, `FlameSite { color: Some(blue), ..FireKind::Lantern.site() }`.
///
/// ```
/// # use bevy_lights::*;
/// let fire = FireKind::Brazier.flicker(1200.0);
/// assert!((fire.mean() - 1200.0).abs() < 1e-2);
/// assert_eq!(fire.diameter, FireKind::Brazier.site().diameter);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FireKind {
    /// A candle: a 1.3 cm bed, a fast fine shimmer.
    Candle,
    /// A lantern's wick: calm, steady.
    Lantern,
    /// A hand-held torch: small, fast and gusting.
    Torch,
    /// A coal-bed brazier: the courtyard's showcase fire.
    Brazier,
    /// A campfire.
    Campfire,
    /// A bonfire: a slow heave.
    Bonfire,
}

impl FireKind {
    /// Every kind, smallest fire first, in the order a picker lists them.
    pub const ALL: [FireKind; 6] = [
        FireKind::Candle,
        FireKind::Lantern,
        FireKind::Torch,
        FireKind::Brazier,
        FireKind::Campfire,
        FireKind::Bonfire,
    ];

    /// The fuel bed's diameter in metres: [`Flicker::diameter`] and [`FlameSite::diameter`].
    pub const fn diameter(self) -> f32 {
        match self {
            FireKind::Candle => 0.013,
            FireKind::Lantern => 0.04,
            FireKind::Torch => 0.10,
            FireKind::Brazier => 0.34,
            FireKind::Campfire => 0.8,
            FireKind::Bonfire => 3.0,
        }
    }

    /// How far the fire dips: [`Flicker::depth`] and [`FlameSite::depth`].
    pub const fn depth(self) -> f32 {
        match self {
            FireKind::Candle => 0.10,
            FireKind::Lantern => CALM_FLICKER_DEPTH,
            FireKind::Torch => WHIPPING_FLICKER_DEPTH,
            FireKind::Brazier => FLICKER_DEPTH,
            FireKind::Campfire => 0.20,
            FireKind::Bonfire => 0.25,
        }
    }

    /// The drawn flame's height in metres: [`FlameSite::height`].
    pub const fn height(self) -> f32 {
        match self {
            FireKind::Candle => 0.04,
            FireKind::Lantern => 0.12,
            FireKind::Torch => 0.35,
            FireKind::Brazier => 1.1,
            FireKind::Campfire => 0.7,
            FireKind::Bonfire => 2.0,
        }
    }

    /// How far the fire lights the ground, in metres, to the edge of its pool: the size a host
    /// budgets its light from. Brazier and Torch are the courtyard's own pools; the rest are design
    /// defaults (`docs/research/fire-reach.md`). Never under `fire_lumens`' 1 m floor.
    pub const fn reach(self) -> f32 {
        match self {
            FireKind::Candle => 1.0,
            FireKind::Lantern => 1.5,
            FireKind::Torch => 2.0,
            FireKind::Brazier => 4.0,
            FireKind::Campfire => 5.0,
            FireKind::Bonfire => 9.0,
        }
    }

    /// The lumens that put `lux` on the ground [`reach`](Self::reach) metres out, with the light
    /// `over` metres above it: [`fire_lumens`] at this kind's reach. `lux` is the host's own look
    /// number in its own exposure; nothing here converts units.
    pub fn lumens(self, lux: f32, over: f32) -> f32 {
        fire_lumens(lux, self.reach(), over)
    }

    /// The light's flicker, sized to light its [`reach`](Self::reach) to `lux`:
    /// [`flicker`](Self::flicker) averaging [`lumens`](Self::lumens).
    pub fn light(self, lux: f32, over: f32) -> Flicker {
        self.flicker(self.lumens(lux, over))
    }

    /// The pool candidate for this fire, sized like [`light`](Self::light): its mean lights
    /// [`reach`](Self::reach) to `lux` with the light `over` metres above the ground. `over` is the height
    /// of the light, which stands at the middle of the flame: `offset` is
    /// [`flame_light_offset`]`(`[`height`](Self::height)`)`, so a candidate on the [`FlameSite`] entity (whose
    /// origin is the flame's foot) is handed a light inside its flame, and in a [`FlameWind`](crate::FlameWind)
    /// the pool moves it with the leaned flame's middle. `phase` is `None`, and `diameter` and `depth` are this
    /// kind's, so the light a torch is handed breathes at the flame's own rate and depth.
    pub fn candidate(self, lux: f32, over: f32) -> PoolCandidate {
        self.candidate_at_mean(self.lumens(lux, over))
    }

    /// The pool candidate for this fire whose light averages `mean` lumens: [`candidate`](Self::candidate) with the
    /// budget written rather than solved from a pool's lux, as [`flicker`](Self::flicker) is to [`light`](Self::light).
    /// `offset`, `diameter` and `depth` are this kind's, `phase` and `color` are `None`; [`Fire`](crate::Fire) is
    /// the one that sets them.
    pub fn candidate_at_mean(self, mean: f32) -> PoolCandidate {
        PoolCandidate {
            offset: Vec3::from(flame_light_offset(self.height())),
            diameter: Some(self.diameter()),
            ..PoolCandidate::at_mean(mean, self.depth())
        }
    }

    /// How fast the fire flickers, in hertz: the puffing law at [`diameter`](Self::diameter).
    pub fn rate(self) -> f32 {
        flicker_rate(self.diameter())
    }

    /// The sparks this fire sheds, or `None` for one that sheds none (a candle, a lantern's wick: no
    /// solid fuel to spit). Sparks scale with the fuel burned, so the rate rises with the fire's size
    /// up to [`MAX_EMBER_RATE`]; the brazier's is [`FlameEmbers::default`]. Insert the value on the
    /// [`FlameSite`] entity. A design default (`docs/research/fire-kinds.md`).
    pub fn embers(self) -> Option<FlameEmbers> {
        let rate = match self {
            FireKind::Candle | FireKind::Lantern => return None,
            FireKind::Torch => 2.0,
            FireKind::Brazier => return Some(FlameEmbers::default()),
            FireKind::Campfire => 5.0,
            FireKind::Bonfire => MAX_EMBER_RATE,
        };
        Some(FlameEmbers { rate })
    }

    /// The smoke this fire gives off, or `None` for one that gives none worth drawing (a lantern's enclosed wick). A wood or
    /// pitch fire smokes more than a clean one: a candle a thin wisp, a charcoal brazier little, a torch, a campfire and a
    /// bonfire a column. Insert the value on the [`FlameSite`] entity, as [`embers`](Self::embers). A design default
    /// (`docs/research/flame-smoke.md`).
    pub fn smoke(self) -> Option<FlameSmoke> {
        let density = match self {
            FireKind::Lantern => return None,
            FireKind::Candle => 0.25,
            FireKind::Torch => 0.8,
            FireKind::Campfire => 0.9,
            FireKind::Brazier => 0.5,
            FireKind::Bonfire => crate::MAX_SMOKE_DENSITY,
        };
        Some(FlameSmoke { density })
    }

    /// The tongues this fire burns as, or `None` for one whose bed is no wider than its one flame can fill (a candle, a lantern's wick, a
    /// torch and a brazier: a bed under a third of the flame's height). A campfire's 0.8 m bed under a 0.7 m flame is six tongues over
    /// 0.55 m, and a bonfire's 3 m pyre under a 2 m flame nine over 1.6 m: drawn as the one slender flame they were, both read as a very
    /// large candle. Insert the value on the [`FlameSite`] entity, as [`embers`](Self::embers). A design default
    /// (`docs/research/flame-tongues.md`).
    pub fn tongues(self) -> Option<FlameTongues> {
        match self {
            FireKind::Campfire => Some(FlameTongues { count: 6, spread: 0.55 }),
            FireKind::Bonfire => Some(FlameTongues { count: 9, spread: 1.6 }),
            FireKind::Candle | FireKind::Lantern | FireKind::Torch | FireKind::Brazier => None,
        }
    }

    /// The bed of coals this fire burns on, or `None` for one with no bed worth drawing (a candle's wick, a lantern's, a torch's wrapped
    /// head). A campfire's is its 0.8 m ring less the stones, a bonfire's the pyre's glowing floor under its tongues, a brazier's the
    /// coal bed in its bowl. Insert the value on the [`FlameSite`] entity, as [`embers`](Self::embers); the coals follow the site's fire
    /// (`FlameSite::fire`), breathing with its flicker and glowing on after it is put out. A design default
    /// (`docs/research/fuel-bed-heat.md`).
    ///
    /// ```
    /// # use bevy_lights::*;
    /// assert!(FireKind::Torch.bed().is_none());
    /// for kind in FireKind::ALL {
    ///     if let Some(bed) = kind.bed() {
    ///         assert!(bed.problems().is_empty());
    ///         // The bed is a little narrower than the fuel it sits in.
    ///         assert!(bed.diameter <= kind.diameter());
    ///     }
    /// }
    /// ```
    pub fn bed(self) -> Option<FuelBed> {
        let diameter = match self {
            FireKind::Candle | FireKind::Lantern | FireKind::Torch => return None,
            FireKind::Brazier => 0.30,
            FireKind::Campfire => 0.7,
            FireKind::Bonfire => 1.9,
        };
        Some(FuelBed { diameter, ..FuelBed::default() })
    }

    /// The fire's name, as a picker shows it.
    pub const fn name(self) -> &'static str {
        match self {
            FireKind::Candle => "Candle",
            FireKind::Lantern => "Lantern",
            FireKind::Torch => "Torch",
            FireKind::Brazier => "Brazier",
            FireKind::Campfire => "Campfire",
            FireKind::Bonfire => "Bonfire",
        }
    }

    /// The light's flicker, averaging `mean` lumens: [`Flicker::at_mean`] with this kind's depth and
    /// diameter. `phase` is `0.0`.
    pub fn flicker(self, mean: f32) -> Flicker {
        Flicker { diameter: Some(self.diameter()), ..Flicker::at_mean(mean, self.depth()) }
    }

    /// The flame's settings: this kind's height, diameter and depth, burning already (`kindle` 0), no
    /// depth fade, no linked fire.
    pub fn site(self) -> FlameSite {
        FlameSite {
            color: None,
            height: self.height(),
            phase: 0.0,
            kindle: 0.0,
            diameter: Some(self.diameter()),
            depth: self.depth(),
            depth_fade: false,
            fire: None,
        }
    }
}

impl core::fmt::Display for FireKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.name())
    }
}

/// A name that is not a [`FireKind`]: [`FireKind::from_str`](core::str::FromStr) rejects it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownFireKind(pub String);

impl core::fmt::Display for UnknownFireKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "unknown fire kind {:?}; expected one of ", self.0)?;
        for (i, kind) in FireKind::ALL.iter().enumerate() {
            f.write_str(if i == 0 { "" } else { ", " })?;
            f.write_str(kind.name())?;
        }
        Ok(())
    }
}

impl core::error::Error for UnknownFireKind {}

/// Parses a [`name`](FireKind::name), ignoring case and surrounding whitespace, so a text field, a
/// command line or an environment variable can pick a kind. [`Display`](core::fmt::Display) is the
/// inverse.
///
/// ```
/// # use bevy_lights::*;
/// assert_eq!(" torch ".parse(), Ok(FireKind::Torch));
/// assert!("pyre".parse::<FireKind>().is_err());
/// ```
impl core::str::FromStr for FireKind {
    type Err = UnknownFireKind;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        FireKind::ALL
            .into_iter()
            .find(|k| k.name().eq_ignore_ascii_case(s))
            .ok_or_else(|| UnknownFireKind(s.to_owned()))
    }
}
