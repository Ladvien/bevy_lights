//! Where the tongues of a wide fire stand, and how tall each is.
//!
//! A flame is one teardrop, `0.4` of its height wide: a candle's and a torch's shape, and the wrong one for a
//! bed two to four times as wide as the flame is tall. A campfire or a bonfire is a heap of tongues of
//! different heights that lean and part, the tallest in the middle and a skirt of shorter ones round it
//! (`docs/research/flame-tongues.md`). [`flame_tongue`] places them, as a pure function of the index, the count,
//! the bed's width, the flame's height and a seed, so the same fire is the same on every machine and two fires
//! are not the same as each other.
//!
//! The one physical rule in it is the merge height of flames that burn side by side: two flames a distance
//! `S` apart are one flame above `3 S` (Delichatsios 2007, eqs. 6b and 7b, "L_f / S = 3 - 3.3"). A side tongue
//! is the part of the fire below that height, so it never stands taller than [`MERGE_RATIO`] times its
//! distance from the middle one.

use core::f32::consts::TAU;

/// The most tongues one fire has: the middle one and eight round it. Design default: more is overdraw.
pub const MAX_TONGUES: u8 = 9;

/// How many times its distance from its neighbour a flame stands before the two are one flame: Delichatsios
/// 2007, `Z_m / S ≈ 3` (eq. 6b) and `L_f / S = 3 - 3.3` (eq. 7b).
pub const MERGE_RATIO: f32 = 3.0;

/// The golden angle, in radians: successive tongues turn by it, so any number of them fills the bed evenly.
const GOLDEN_ANGLE: f32 = 2.399_963_2;

/// The innermost foot, as a share of the bed's radius: the first tongue is well away from the middle one.
const INNER_FOOT: f32 = 0.35;

/// How much shorter than the middle tongue the outermost tongue's mound is: `1 - MOUND_DROP` at the rim.
const MOUND_DROP: f32 = 0.4;

/// The sine of the tilt, towards the middle, of a tongue standing at the rim of the bed: a fire draws the air in at its foot, so its outer
/// tongues lean in and the fire converges on a tip instead of standing as a row. Less in step with how far from the middle the foot is.
/// Design default, judged in captures.
pub const INWARD_LEAN: f32 = 0.30;

/// The tallest a side tongue is, as a share of the middle one's height: the middle is always the tallest.
const SIDE_CEILING: f32 = 0.92;

/// One tongue of a fire: where its foot stands on the bed, how tall it is and how far its clock is turned.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tongue {
    /// Where the foot stands, `[x, z]`, in the host's units from the middle of the bed.
    pub foot: [f32; 2],
    /// The tongue's height, in the host's units.
    pub height: f32,
    /// Radians added to the fire's own clock phase, so no two tongues move together.
    pub phase: f32,
    /// The sine of the tilt towards the middle tongue, times the unit direction to it, `[x, z]` in the same frame as `foot`: written the
    /// way a wind's lean is, so the tongue is laid over the way a wind lays a flame. The middle tongue's is zero.
    pub lean: [f32; 2],
}

/// A value in `[0, 1)` that depends on the seed and the salt and nothing else.
fn unit(seed: f32, salt: u32) -> f32 {
    let mut x = seed.to_bits() ^ salt.wrapping_mul(0x9e37_79b1);
    x = x.wrapping_mul(0x85eb_ca6b);
    x ^= x >> 13;
    x = x.wrapping_mul(0xc2b2_ae35);
    x ^= x >> 16;
    (x >> 8) as f32 / 16_777_216.0
}

/// Tongue number `index` of a fire of `count` tongues, standing on a bed `spread` wide under a middle flame
/// `height` tall; `seed` (the fire's own phase is the natural one) tells two fires apart.
///
/// Index `0` is the middle tongue: at the origin, at `height`, with no phase of its own. It is also what a
/// one-tongue fire, and any fire whose `spread` is not a positive number, is made of entirely. The others lie on
/// a golden-angle spiral over the disc, between 0.4 of the radius and all of it, each a hash of the
/// seed and the index off its place so two fires differ; a tongue's height falls from the middle's with its
/// distance from it (a mound) with up to a third of jitter either way, it leans in towards the middle ([`INWARD_LEAN`] at the rim), is never above `0.92` of the middle's, and never above
/// [`MERGE_RATIO`] times that distance. An `index` past the last is the last, and a `count` over
/// [`MAX_TONGUES`] is [`MAX_TONGUES`].
pub fn flame_tongue(index: u8, count: u8, spread: f32, height: f32, seed: f32) -> Tongue {
    let count = count.clamp(1, MAX_TONGUES);
    let middle = Tongue { foot: [0.0, 0.0], height, phase: 0.0, lean: [0.0, 0.0] };
    if index == 0 || count == 1 || !(spread.is_finite() && spread > 0.0) {
        return middle;
    }
    let index = index.min(count - 1);
    let outer = f32::from(count - 1);
    let place = f32::from(index - 1);
    let salt = u32::from(index) * 5;
    let radius = 0.5 * spread;
    let reach = radius
        * (INNER_FOOT + (1.0 - INNER_FOOT) * ((place + 0.5) / outer).sqrt())
        * (0.92 + 0.16 * unit(seed, salt + 1));
    let reach = reach.min(radius);
    let turn = place * GOLDEN_ANGLE + TAU * unit(seed, 0) + 1.6 * (unit(seed, salt + 2) - 0.5);
    let mound = height * (1.0 - MOUND_DROP * reach / radius) * (0.70 + 0.45 * unit(seed, salt + 3));
    let tilt = INWARD_LEAN * reach / radius;
    Tongue {
        foot: [reach * turn.cos(), reach * turn.sin()],
        height: mound.min(MERGE_RATIO * reach).min(SIDE_CEILING * height),
        phase: TAU * unit(seed, salt + 4),
        lean: [-tilt * turn.cos(), -tilt * turn.sin()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEEDS: [f32; 6] = [0.0, 0.4, 1.3, 2.9, 5.1, 6.2];

    fn all(count: u8, spread: f32, height: f32, seed: f32) -> Vec<Tongue> {
        (0..count).map(|index| flame_tongue(index, count, spread, height, seed)).collect()
    }

    #[test]
    fn the_same_inputs_are_the_same_fire() {
        assert_eq!(all(7, 1.8, 2.0, 1.3), all(7, 1.8, 2.0, 1.3));
        assert_ne!(all(7, 1.8, 2.0, 1.3), all(7, 1.8, 2.0, 1.4), "the seed tells two fires apart");
    }

    #[test]
    fn the_middle_tongue_is_the_fires_own_flame_and_the_tallest() {
        for count in 1..=MAX_TONGUES {
            for seed in SEEDS {
                let fire = all(count, 0.5, 0.7, seed);
                assert_eq!(fire[0], Tongue { foot: [0.0, 0.0], height: 0.7, phase: 0.0, lean: [0.0, 0.0] });
                for side in &fire[1..] {
                    assert!(side.height < fire[0].height, "count {count} seed {seed}: {side:?}");
                }
            }
        }
    }

    #[test]
    fn every_foot_is_on_the_bed_and_off_the_middle() {
        for count in 2..=MAX_TONGUES {
            for seed in SEEDS {
                for side in &all(count, 0.5, 0.7, seed)[1..] {
                    let from_middle = side.foot[0].hypot(side.foot[1]);
                    assert!(from_middle <= 0.25 + 1e-6, "count {count} seed {seed}: {from_middle}");
                    assert!(from_middle >= 0.25 * INNER_FOOT * 0.92 - 1e-6, "{from_middle}");
                }
            }
        }
    }

    #[test]
    fn a_tongue_never_stands_taller_than_the_merge_height_of_its_distance_from_the_middle() {
        for count in 2..=MAX_TONGUES {
            for seed in SEEDS {
                for side in &all(count, 0.5, 1.1, seed)[1..] {
                    let distance = side.foot[0].hypot(side.foot[1]);
                    assert!(side.height <= MERGE_RATIO * distance + 1e-6, "{side:?}");
                    assert!(side.height > 0.0, "{side:?}");
                }
            }
        }
    }

    #[test]
    fn the_feet_are_apart_and_the_clocks_differ() {
        for count in 2..=MAX_TONGUES {
            for seed in SEEDS {
                let fire = all(count, 1.0, 1.0, seed);
                for (a, first) in fire.iter().enumerate() {
                    for second in &fire[a + 1..] {
                        let apart = (first.foot[0] - second.foot[0]).hypot(first.foot[1] - second.foot[1]);
                        assert!(apart > 0.06, "count {count} seed {seed}: two feet {apart} apart");
                        assert!((first.phase - second.phase).abs() > 1e-4 || a == 0, "no two clocks the same");
                    }
                }
            }
        }
    }

    #[test]
    fn a_fire_with_one_tongue_or_no_bed_is_the_plain_flame() {
        for index in 0..4 {
            for (count, spread) in [(1, 0.5), (5, 0.0), (5, -1.0), (5, f32::NAN), (5, f32::INFINITY)] {
                assert_eq!(
                    flame_tongue(index, count, spread, 0.7, 2.0),
                    Tongue { foot: [0.0, 0.0], height: 0.7, phase: 0.0, lean: [0.0, 0.0] },
                    "index {index}, count {count}, spread {spread}"
                );
            }
        }
    }

    #[test]
    fn a_count_or_index_out_of_range_is_the_last_one() {
        assert_eq!(flame_tongue(200, 4, 0.5, 0.7, 1.0), flame_tongue(3, 4, 0.5, 0.7, 1.0));
        assert_eq!(flame_tongue(5, 200, 0.5, 0.7, 1.0), flame_tongue(5, MAX_TONGUES, 0.5, 0.7, 1.0));
        assert_eq!(flame_tongue(1, 0, 0.5, 0.7, 1.0).foot, [0.0, 0.0], "a count of none is the plain flame");
    }

    #[test]
    fn a_tongue_leans_towards_the_middle_by_no_more_than_the_rim_lean() {
        for count in 2..=MAX_TONGUES {
            for seed in SEEDS {
                for side in &all(count, 0.5, 0.7, seed)[1..] {
                    let tilt = side.lean[0].hypot(side.lean[1]);
                    assert!(tilt > 0.0 && tilt <= INWARD_LEAN + 1e-6, "{side:?}");
                    let inward = -(side.foot[0] * side.lean[0] + side.foot[1] * side.lean[1]);
                    assert!(inward > 0.0, "the lean points at the middle: {side:?}");
                    // Parallel to the foot's direction, not merely on the middle's side.
                    let cross = side.foot[0] * side.lean[1] - side.foot[1] * side.lean[0];
                    assert!(cross.abs() < 1e-5, "{side:?}");
                }
            }
        }
    }

    #[test]
    fn a_wider_bed_spreads_the_same_fire_wider_and_does_not_reshape_it() {
        let narrow = all(5, 0.5, 0.7, 1.3);
        let wide = all(5, 1.0, 0.7, 1.3);
        for (near, far) in narrow.iter().zip(&wide).skip(1) {
            assert!((far.foot[0] - 2.0 * near.foot[0]).abs() < 1e-5 && (far.foot[1] - 2.0 * near.foot[1]).abs() < 1e-5);
            assert_eq!(near.phase, far.phase);
        }
    }
}
