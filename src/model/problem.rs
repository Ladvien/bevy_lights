//! A setting that cannot work, named: what an editor shows beside a field.
//!
//! Every settings type (`Flicker`, `FlameSite`, `FlameLook`, `LightPool`) has a `problems()` method
//! returning these. The crate itself never refuses a value (a negative `base` or a `NaN` phase still
//! builds and runs); `problems()` is for a host that reads settings from a file or a panel and wants a
//! useful error before the value reaches a light.

use std::fmt;

/// Why a field's value is unusable.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ProblemKind {
    /// `NaN` or infinite.
    NotFinite,
    /// Below zero where zero is the lowest meaningful value.
    Negative,
    /// Zero or below where the value must be strictly positive.
    NotPositive,
    /// Outside the range the crate honours (it is clamped or ignored there).
    OutOfRange {
        /// The lowest value used.
        min: f32,
        /// The highest value used.
        max: f32,
    },
}

/// One unusable field: its name as written in the struct, and why.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Problem {
    /// The field, e.g. `"depth"` or `"light.range"`.
    pub field: &'static str,
    /// What is wrong with it.
    pub kind: ProblemKind,
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            ProblemKind::NotFinite => write!(f, "`{}` must be a finite number", self.field),
            ProblemKind::Negative => write!(f, "`{}` must not be negative", self.field),
            ProblemKind::NotPositive => write!(f, "`{}` must be greater than zero", self.field),
            ProblemKind::OutOfRange { min, max } => {
                write!(f, "`{}` must be between {min} and {max}", self.field)
            }
        }
    }
}

impl std::error::Error for Problem {}

/// A finite value `>= 0`.
pub fn non_negative(out: &mut Vec<Problem>, field: &'static str, v: f32) {
    if !v.is_finite() {
        out.push(Problem { field, kind: ProblemKind::NotFinite });
    } else if v < 0.0 {
        out.push(Problem { field, kind: ProblemKind::Negative });
    }
}

/// A finite value `> 0`.
pub fn positive(out: &mut Vec<Problem>, field: &'static str, v: f32) {
    if !v.is_finite() {
        out.push(Problem { field, kind: ProblemKind::NotFinite });
    } else if v <= 0.0 {
        out.push(Problem { field, kind: ProblemKind::NotPositive });
    }
}

/// A finite value, any sign.
pub fn finite(out: &mut Vec<Problem>, field: &'static str, v: f32) {
    if !v.is_finite() {
        out.push(Problem { field, kind: ProblemKind::NotFinite });
    }
}

/// A finite value in `min..=max`.
pub fn within(out: &mut Vec<Problem>, field: &'static str, v: f32, min: f32, max: f32) {
    if !v.is_finite() {
        out.push(Problem { field, kind: ProblemKind::NotFinite });
    } else if v < min || v > max {
        out.push(Problem { field, kind: ProblemKind::OutOfRange { min, max } });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nan_is_not_finite_and_the_message_names_the_field() {
        let mut out = Vec::new();
        positive(&mut out, "reach", f32::NAN);
        assert_eq!(out, [Problem { field: "reach", kind: ProblemKind::NotFinite }]);
        assert_eq!(out[0].to_string(), "`reach` must be a finite number");
    }
}
