//! [`FireKind`]: the numbers a fire is made of, by name.

use bevy::prelude::*;

use crate::model::{CALM_FLICKER_DEPTH, FLICKER_DEPTH, WHIPPING_FLICKER_DEPTH, fire_lumens, flicker_rate};
use crate::{FlameSite, Flicker, PoolCandidate};

/// A named kind of fire: its fuel-bed diameter, flicker depth and flame height together, so a
/// picker (or a project file) stores one word instead of three loose numbers.
///
/// The diameter sizes the flicker through the puffing law, the depth is how far the fire dips, and
/// the height is the drawn flame's. [`flicker`](Self::flicker) and [`site`](Self::site) build the
/// light's and the flame's settings **from the same three numbers**, so the two breathe together.
/// Only the brazier row is a number the crate shipped before; the rest are design defaults
/// (`docs/research/fire-kinds.md`). Spell the rest of a fire with struct update:
/// `FlameSite { phase: 1.3, ..FireKind::Torch.site() }`.
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
    /// [`reach`](Self::reach) to `lux` with the light `over` metres above the ground. `offset` is
    /// zero (put the candidate on the [`FlameSite`] entity, whose origin is the flame's foot), `phase`
    /// is `None`, and `diameter` and `depth` are this kind's, so the light a torch is handed breathes
    /// at the flame's own rate and depth.
    pub fn candidate(self, lux: f32, over: f32) -> PoolCandidate {
        PoolCandidate {
            diameter: Some(self.diameter()),
            ..PoolCandidate::at_mean(self.lumens(lux, over), self.depth())
        }
    }

    /// How fast the fire flickers, in hertz: the puffing law at [`diameter`](Self::diameter).
    pub fn rate(self) -> f32 {
        flicker_rate(self.diameter())
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
