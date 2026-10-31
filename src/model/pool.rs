//! The pool's hand-off: which candidate holds a light, and how a light that changes hands fades.
//!
//! The pool hands its real lights to the nearest candidates, and when the ordering changes a slot
//! teleports from one prop to another. Two plain functions decide that, so the rule can be tested
//! without an app: [`hand_off`] keeps a slot on the candidate it holds until a challenger is
//! meaningfully nearer, and [`fade_factor`] dips the light's brightness through the change so the
//! teleport reads as a change of intensity rather than a jump.
//!
//! The craft behind both is in `docs/research/pool-hand-off.md`: overlapping keys, and the eye's
//! finite integration time.

/// The default hysteresis: a challenger must be at least this much nearer than the candidate a slot
/// holds before it takes the light.
///
/// `1.0` is "nearest wins outright", the behaviour before this module existed. `1.15` means a
/// challenger has to be about 13 % closer, which stops two props at nearly the same distance from
/// trading a light every frame while still letting a clearly nearer one win at once.
pub const HYSTERESIS: f32 = 1.15;

/// How long a light takes to come back to full brightness after it changes owner, in seconds.
///
/// A third of a second is 20 frames at 60 Hz: well below the eye's flicker-fusion threshold, so the
/// change is followed rather than seen as a step, and long enough to be read as a dimming.
pub const FADE_SECONDS: f32 = 0.35;

/// The candidate a pool slot should hold, given the one it holds now and its distance against the
/// nearest challenger's.
///
/// `held_distance` is the squared distance from the focus to the held candidate, and
/// `challenger_distance` the same for the nearest candidate that is not held. Distances are squared
/// because that is what the pool sorts on; the hysteresis is applied to the square roots, which is
/// the same comparison on the distances themselves.
///
/// Returns `true` when the slot should take the challenger:
///
/// - with no held candidate (a slot newly filled), `true` — the nearest wins;
/// - when the held candidate is still in reach and the challenger is not nearer by `hysteresis`
///   times, `false` — "overlapping keys": the held light stays;
/// - when the challenger is nearer by more than that, `true`;
/// - `hysteresis` at or below 1.0 makes every nearer challenger win, which is the old rule.
///
/// A held candidate that has left reach or gone is the caller's to drop: this function only compares
/// distances, so a caller that has no held candidate passes [`f32::INFINITY`] and takes the challenger
/// immediately.
///
/// ```
/// # use bevy_lights::model::pool::*;
/// // Held at 4 m, challenged at 3.9 m with 1.15 hysteresis: the held light stays.
/// assert!(!hand_off(16.0, 15.21, HYSTERESIS));
/// // Challenged at 2 m: the challenger wins.
/// assert!(hand_off(16.0, 4.0, HYSTERESIS));
/// // Nothing held: the challenger always wins.
/// assert!(hand_off(f32::INFINITY, 100.0, HYSTERESIS));
/// ```
pub fn hand_off(held_distance: f32, challenger_distance: f32, hysteresis: f32) -> bool {
    // Both `NaN`s are answered **before** the clamp: `f32::max` returns the other operand when one is
    // `NaN`, so a broken distance would silently become a zero — "nothing held", or "at the focus".
    if challenger_distance.is_nan() {
        return false;
    }
    if held_distance.is_nan() {
        return true;
    }
    // Compare on the distances, not their squares: the hysteresis is a ratio of distances.
    preferred(held_distance.max(0.0).sqrt(), challenger_distance.max(0.0).sqrt(), hysteresis)
}

/// The overlap rule itself, on any "smaller is better" quantity: `true` when `challenger` beats `held`
/// by the `hysteresis` margin. [`hand_off`] is this on distances.
///
/// A pool that ranks its candidates by something other than distance — the light each puts on the
/// subject, say — compares that quantity here, so ranking and holding are one rule rather than two. The
/// quantity is not square-rooted: the caller states it in the units the margin applies to, which for
/// [`hand_off`] is the distance and not its square.
///
/// ```
/// # use bevy_lights::model::pool::*;
/// // The light arriving at the subject, as its reciprocal: a value 15 % smaller wins.
/// assert!(preferred(1.0, 1.0 / 1.16, HYSTERESIS));
/// assert!(!preferred(1.0, 1.0 / 1.10, HYSTERESIS));
/// ```
pub fn preferred(held: f32, challenger: f32, hysteresis: f32) -> bool {
    // A challenge that is not a number cannot be compared; keep what is held rather than chatter.
    if challenger.is_nan() {
        return false;
    }
    if !held.is_finite() {
        return true;
    }
    if held.is_nan() {
        return true;
    }
    let margin = if hysteresis.is_nan() { 1.0 } else { hysteresis.max(1.0) };
    // `is_nan` on the challenger already returned; the held value is clamped, which is right for a
    // distance and for an illuminance reciprocal alike.
    challenger.max(0.0) * margin < held.max(0.0)
}

/// The brightness multiplier of a light that is **leaving** a hand-off `since` seconds after it: 1 at
/// the instant of the hand-off, falling to exactly 0 over `fade_seconds`, and 0 from then on.
///
/// The complement of [`fade_factor`], so a cross-fade's departing and arriving lights sum to exactly
/// one at every point: the subject neither brightens nor darkens as the light changes hands, which is
/// what "overlapping key lights" buys.
///
/// ```
/// # use bevy_lights::model::pool::*;
/// for since in [0.0, 0.05, 0.1, 0.2, 0.35] {
///     assert!((fade_out_factor(since, FADE_SECONDS) + fade_factor(since, FADE_SECONDS) - 1.0).abs() < 1e-6);
/// }
/// ```
pub fn fade_out_factor(since: f32, fade_seconds: f32) -> f32 {
    1.0 - fade_factor(since, fade_seconds)
}

/// The brightness multiplier of a light `since` seconds after it changed owner: 0 at the instant of
/// the hand-off, rising to exactly 1 over [`FADE_SECONDS`], and exactly 1 from then on.
///
/// Smoothstep, so the multiplier's derivative is zero at both ends and the fade has no edge to catch
/// the eye at either end. A negative or `NaN` time is 0 (the fade has just begun); a time past the fade
/// is 1.
///
/// ```
/// # use bevy_lights::model::pool::*;
/// assert_eq!(fade_factor(0.0, FADE_SECONDS), 0.0);
/// assert_eq!(fade_factor(FADE_SECONDS, FADE_SECONDS), 1.0);
/// assert_eq!(fade_factor(9.0, FADE_SECONDS), 1.0);
/// assert!(fade_factor(0.1, FADE_SECONDS) < fade_factor(0.2, FADE_SECONDS));
/// ```
pub fn fade_factor(since: f32, fade_seconds: f32) -> f32 {
    if since.is_nan() || fade_seconds <= 0.0 {
        return if since.is_nan() { 0.0 } else { 1.0 };
    }
    let s = (since / fade_seconds).clamp(0.0, 1.0);
    s * s * (3.0 - 2.0 * s)
}

/// Which of the pool's held lights cast shadows: the best-ranked, up to `budget`, held with the pool's
/// own overlap rule so two lights at nearly the same rank never trade a shadow frame after frame.
///
/// `holders` are the lights the pool holds, **best-ranked first**, each as its rank value ("smaller is
/// better", the quantity [`preferred`] compares) and whether it casts a shadow now. The answer is, in the
/// same order, whether each should:
///
/// 1. more shadows than `budget` (the budget was lowered) drops the worst-ranked ones;
/// 2. a place left in the budget goes to the best-ranked light without one, at once;
/// 3. a light without a shadow takes the worst-ranked shadow only when it is better by `hysteresis`.
///
/// The rule is Real-Time Rendering's for many lights (§20.4): shadow "the closest and brightest lights"
/// and let the lesser ones leak. `docs/research/pool-shadows.md` has the craft.
///
/// ```
/// # use bevy_lights::model::pool::*;
/// // Three lights, one shadow, nobody shadowed yet: the best one takes it.
/// assert_eq!(shadow_set(&[(1.0, false), (2.0, false), (3.0, false)], 1, HYSTERESIS), [true, false, false]);
/// // The second is now only 10 % better than the shadowed third: the shadow stays where it is.
/// assert_eq!(shadow_set(&[(1.0, true), (2.0, false), (2.2, true)], 2, HYSTERESIS), [true, false, true]);
/// ```
pub fn shadow_set(holders: &[(f32, bool)], budget: usize, hysteresis: f32) -> Vec<bool> {
    let mut out: Vec<bool> = holders.iter().map(|(_, shadowed)| *shadowed).collect();
    // 1. Over budget: the worst-ranked shadows go first.
    let mut count = out.iter().filter(|s| **s).count();
    for shadowed in out.iter_mut().rev() {
        if count <= budget {
            break;
        }
        if *shadowed {
            *shadowed = false;
            count -= 1;
        }
    }
    // 2. Free places: the best-ranked light without a shadow takes one, no margin needed.
    for shadowed in out.iter_mut() {
        if count >= budget {
            break;
        }
        if !*shadowed {
            *shadowed = true;
            count += 1;
        }
    }
    // 3. The overlap rule: the best light without a shadow against the worst light with one.
    loop {
        let best_without = out.iter().position(|s| !*s);
        let worst_with = out.iter().rposition(|s| *s);
        let (Some(challenger), Some(held)) = (best_without, worst_with) else { break };
        if challenger > held || !preferred(holders[held].0, holders[challenger].0, hysteresis) {
            break;
        }
        out[challenger] = true;
        out[held] = false;
    }
    out
}

/// How many of a pool's `size` real lights each of `foci` viewers is entitled to when they share it:
/// `ceil(size / foci)`, never below 1 while the pool has a light at all.
///
/// An equal share, and an entitlement and not a reservation: a viewer that does not use its share leaves
/// it to the others, because [`shared_value`] only ranks, it never sets lights aside. With no foci there
/// is no one to share with, and the share is 0.
///
/// ```
/// # use bevy_lights::model::pool::*;
/// assert_eq!(focus_quota(8, 2), 4);
/// assert_eq!(focus_quota(7, 2), 4, "rounded up, so no light is left over");
/// assert_eq!(focus_quota(16, 1), 16);
/// assert_eq!(focus_quota(2, 4), 1, "never below one light while there is one");
/// assert_eq!(focus_quota(0, 3), 0);
/// assert_eq!(focus_quota(8, 0), 0);
/// ```
pub fn focus_quota(size: usize, foci: usize) -> usize {
    if size == 0 || foci == 0 { 0 } else { size.div_ceil(foci) }
}

/// The value of one viewer's `quota`-th best candidate: the number that viewer's other candidates are
/// measured against by [`shared_value`].
///
/// `values` are that viewer's candidates' rank values ("smaller is better", the quantity [`preferred`]
/// compares), in any order; they are reordered. A viewer with fewer candidates than `quota` is measured
/// against its worst, so every one of them lands at or inside its share. `None` when the viewer has no
/// candidates at all. A `NaN` value sorts last and so is never the cutoff while a real one exists.
///
/// ```
/// # use bevy_lights::model::pool::*;
/// let mut values = [9.0, 1.0, 4.0, 2.0];
/// assert_eq!(quota_cutoff(&mut values, 2), Some(2.0));
/// assert_eq!(quota_cutoff(&mut [3.0, 5.0], 8), Some(5.0), "fewer than the quota: its worst");
/// assert_eq!(quota_cutoff(&mut [], 4), None);
/// ```
pub fn quota_cutoff(values: &mut [f32], quota: usize) -> Option<f32> {
    if values.is_empty() {
        return None;
    }
    let at = quota.clamp(1, values.len()) - 1;
    let (_, cutoff, _) = values.select_nth_unstable_by(at, f32::total_cmp);
    Some(*cutoff)
}

/// A candidate's value for one viewer in a pool several viewers share: its rank `value` measured from
/// that viewer, as a multiple of that viewer's `cutoff` ([`quota_cutoff`]).
///
/// At or below 1 means "inside the share of the viewer it serves". A candidate's value for the pool is
/// the smallest of these over the viewers whose reach it is in, so one ordered list interleaves the
/// viewers in proportion to their shares and a fixture between two viewers is one entry, not two. Every
/// value of one viewer is divided by the same number, so that viewer's order and every ratio between
/// its values (the overlap rule's margin) are unchanged by it.
///
/// The division is done in `f64` and a value that would overflow is held at [`f32::MAX`]: a `cutoff`
/// of 0 (a fire standing on the viewer) must not turn every other value into the same infinity, which
/// would break the tie-break on entity. A `cutoff` of 0 with a `value` of 0 is 0.
///
/// ```
/// # use bevy_lights::model::pool::*;
/// assert_eq!(shared_value(2.0, 4.0), 0.5);
/// assert_eq!(shared_value(0.0, 0.0), 0.0, "on the viewer, with the viewer's share on the viewer");
/// assert!(shared_value(1.0, 0.0) > 1.0e30 && shared_value(1.0, 0.0).is_finite());
/// assert!(shared_value(1.0, 0.0) < shared_value(2.0, 0.0), "still ordered by value");
/// ```
pub fn shared_value(value: f32, cutoff: f32) -> f32 {
    let cutoff = f64::from(cutoff.max(f32::MIN_POSITIVE));
    (f64::from(value.max(0.0)) / cutoff).min(f64::from(f32::MAX)) as f32
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn a_held_light_stays_until_a_challenger_is_meaningfully_nearer() {
        // Held at 4 m. The challenger's distance at which it wins is 4 / 1.15 = 3.478 m.
        let held = 16.0;
        assert!(!hand_off(held, 15.5, HYSTERESIS), "3.937 m does not win");
        assert!(!hand_off(held, 12.25, HYSTERESIS), "3.5 m does not win");
        assert!(hand_off(held, 12.0, HYSTERESIS), "3.464 m wins");
        assert!(hand_off(held, 4.0, HYSTERESIS), "2 m wins");
    }

    #[test]
    fn no_held_candidate_takes_the_nearest_at_once() {
        assert!(hand_off(f32::INFINITY, 100.0, HYSTERESIS));
        assert!(hand_off(f32::NAN, 100.0, HYSTERESIS));
        assert!(!hand_off(1.0, f32::NAN, HYSTERESIS), "a broken distance does not displace a held light");
    }

    #[test]
    fn a_hysteresis_of_one_is_the_old_nearest_wins() {
        assert!(hand_off(16.0, 15.99, 1.0), "any nearer challenger wins at 1.0");
        assert!(!hand_off(16.0, 16.01, 1.0), "a further one never does");
        assert!(hand_off(16.0, 15.99, 0.5), "below 1.0 clamps to the nearest-wins rule");
    }

    #[test]
    fn a_cross_fade_sums_to_one_at_every_point() {
        let mut since = 0.0;
        while since <= FADE_SECONDS {
            let out = fade_out_factor(since, FADE_SECONDS);
            let incoming = fade_factor(since, FADE_SECONDS);
            assert!((out + incoming - 1.0).abs() < 1e-6, "at {since}: {out} + {incoming}");
            assert!((0.0..=1.0).contains(&out), "{out} at {since}");
            since += 0.005;
        }
        assert_eq!(fade_out_factor(0.0, FADE_SECONDS), 1.0, "a departing light starts whole");
        assert_eq!(fade_out_factor(FADE_SECONDS, FADE_SECONDS), 0.0, "and ends gone");
    }

    #[test]
    fn the_fade_runs_from_nothing_to_full_and_stays_there() {
        assert_eq!(fade_factor(0.0, FADE_SECONDS), 0.0);
        assert!((fade_factor(FADE_SECONDS / 2.0, FADE_SECONDS) - 0.5).abs() < 1e-5, "half at half time");
        assert_eq!(fade_factor(FADE_SECONDS, FADE_SECONDS), 1.0);
        assert_eq!(fade_factor(FADE_SECONDS * 3.0, FADE_SECONDS), 1.0);
        assert_eq!(fade_factor(-1.0, FADE_SECONDS), 0.0, "a negative age is the start of the fade");
        assert!(fade_factor(f32::NAN, FADE_SECONDS) == 0.0);
        // A zero-length fade is no fade at all.
        assert_eq!(fade_factor(0.0, 0.0), 1.0);

        // Monotone, and smooth at both ends.
        let mut last = 0.0;
        let mut t = 0.0;
        while t <= FADE_SECONDS {
            let f = fade_factor(t, FADE_SECONDS);
            assert!(f >= last, "the fade fell at {t}");
            assert!((0.0..=1.0).contains(&f), "{f} at {t}");
            last = f;
            t += 0.005;
        }
        assert!(fade_factor(0.001, FADE_SECONDS) < 1e-4, "the start has no step");
        assert!(fade_factor(FADE_SECONDS - 0.001, FADE_SECONDS) > 1.0 - 1e-4, "nor the end");
    }

    #[test]
    fn the_shadow_budget_goes_to_the_best_ranked_and_holds_against_a_near_tie() {
        let unshadowed = [(1.0, false), (2.0, false), (3.0, false), (4.0, false)];
        assert_eq!(shadow_set(&unshadowed, 2, HYSTERESIS), [true, true, false, false], "fill best first");
        assert_eq!(shadow_set(&unshadowed, 0, HYSTERESIS), [false; 4], "a zero budget shadows nothing");
        assert_eq!(shadow_set(&unshadowed, 9, HYSTERESIS), [true; 4], "a budget past the pool shadows all");
        // Held shadows on the 2nd and 4th; the 3rd is 10 % better than the 4th (not enough), the 1st is
        // four times better (enough).
        let held = [(1.0, false), (2.0, true), (3.6, false), (4.0, true)];
        assert_eq!(shadow_set(&held, 2, HYSTERESIS), [true, true, false, false]);
        let near_tie = [(2.0, true), (3.6, false), (4.0, true)];
        assert_eq!(shadow_set(&near_tie, 2, HYSTERESIS), [true, false, true], "a 10 % challenger waits");
        let clear = [(2.0, true), (3.3, false), (4.0, true)];
        assert_eq!(shadow_set(&clear, 2, HYSTERESIS), [true, true, false], "a 21 % challenger wins");
        // A lowered budget drops the worst-ranked shadows.
        assert_eq!(shadow_set(&[(1.0, true), (2.0, true), (3.0, true)], 1, HYSTERESIS), [true, false, false]);
        // A worse light never takes a better light's shadow.
        assert_eq!(shadow_set(&[(1.0, true), (5.0, false)], 1, 1.0), [true, false]);
    }

    /// The pool's whole use of the three functions, on plain numbers: each viewer's values against its own
    /// cutoff, a candidate's value the smallest over the viewers it is in reach of.
    fn pool_values(viewers: &[&[f32]], size: usize) -> Vec<Vec<f32>> {
        let quota = focus_quota(size, viewers.len());
        viewers
            .iter()
            .map(|values| {
                let cutoff = quota_cutoff(&mut values.to_vec(), quota).unwrap_or(1.0);
                values.iter().map(|v| shared_value(*v, cutoff)).collect()
            })
            .collect()
    }

    #[test]
    fn a_viewers_order_and_ratios_survive_the_division_by_its_cutoff() {
        let values = [0.4, 1.0, 1.15, 3.0, 9.0];
        for cutoff in [1e-6_f32, 0.5, 1.0, 7.0, 1e6] {
            let shared: Vec<f32> = values.iter().map(|v| shared_value(*v, cutoff)).collect();
            assert!(shared.windows(2).all(|w| w[0] < w[1]), "order kept at cutoff {cutoff}: {shared:?}");
            for pair in 0..values.len() - 1 {
                let before = preferred(values[pair + 1], values[pair], HYSTERESIS);
                let after = preferred(shared[pair + 1], shared[pair], HYSTERESIS);
                assert_eq!(before, after, "the overlap rule's verdict is the same at cutoff {cutoff}");
            }
        }
    }

    #[test]
    fn a_sparse_viewer_is_not_starved_by_a_dense_one() {
        // Eight lights, two viewers: four each. The first has ten fixtures all nearer than anything the
        // second has; on plain values the second would get none.
        let dense = [1.0, 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9];
        let sparse = [50.0, 60.0, 70.0];
        let shared = pool_values(&[&dense, &sparse], 8);
        let mut ranked: Vec<(f32, &str)> = shared[0].iter().map(|v| (*v, "dense")).collect();
        ranked.extend(shared[1].iter().map(|v| (*v, "sparse")));
        ranked.sort_by(|a, b| a.0.total_cmp(&b.0));
        let first_eight: Vec<&str> = ranked.iter().take(8).map(|(_, who)| *who).collect();
        assert_eq!(
            first_eight.iter().filter(|w| **w == "sparse").count(),
            3,
            "all three of the sparse viewer's: {ranked:?}"
        );
        assert_eq!(
            first_eight.iter().filter(|w| **w == "dense").count(),
            5,
            "the dense viewer's share and what is left"
        );
    }

    #[test]
    fn a_viewers_quota_is_exactly_the_candidates_at_or_inside_its_share() {
        let values = [3.0, 1.0, 2.0, 5.0, 4.0];
        let cutoff = quota_cutoff(&mut values.to_vec(), 3).unwrap();
        let at_or_inside = values.iter().filter(|v| shared_value(**v, cutoff) <= 1.0).count();
        assert_eq!(at_or_inside, 3, "exactly the quota is at or inside the share");
    }

    #[test]
    fn a_cutoff_on_the_viewer_does_not_make_every_other_value_the_same() {
        let values = [0.0, 1e-3, 1.0, 1e4];
        let shared: Vec<f32> = values.iter().map(|v| shared_value(*v, 0.0)).collect();
        assert_eq!(shared[0], 0.0);
        assert!(shared.windows(2).all(|w| w[0] < w[1]), "{shared:?}");
        assert!(shared.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn the_quota_is_an_equal_share_rounded_up() {
        for size in 1..40 {
            for foci in 1..=size.min(5) {
                let quota = focus_quota(size, foci);
                assert!(quota * foci >= size, "{foci} shares of {quota} cover {size} lights");
                assert!(quota * (foci - 1) < size, "and no share is wasted: {size}/{foci} gave {quota}");
            }
        }
    }
}
