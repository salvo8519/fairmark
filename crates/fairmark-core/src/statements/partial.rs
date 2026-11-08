//! Best effort (SPEC-0005 R6): an amount, or the set of input pointers whose absence prevents
//! it. Missing data is never an error and never a silent zero: a sum with a missing term is
//! missing, and carries the pointers of every missing term, so `not_computed` can name them.

use std::collections::BTreeSet;

use rust_decimal::Decimal;

use crate::error::Result;
use crate::explain::Explain;
use crate::num;

/// An amount in currency units, or the JSON pointers of the inputs it needs and lacks.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Partial {
    Known(Decimal),
    Missing(BTreeSet<String>),
}

/// The sign of a term in a sum.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Sign {
    Plus,
    Minus,
}

impl Partial {
    pub(crate) const ZERO: Partial = Partial::Known(Decimal::ZERO);

    /// Missing for lack of the input at `pointer`.
    pub(crate) fn lacking(pointer: impl Into<String>) -> Self {
        Self::Missing(BTreeSet::from([pointer.into()]))
    }

    pub(crate) fn of(value: Option<Decimal>, pointer: impl FnOnce() -> String) -> Self {
        match value {
            Some(v) => Self::Known(v),
            None => Self::lacking(pointer()),
        }
    }

    pub(crate) fn value(&self) -> Option<Decimal> {
        match self {
            Self::Known(v) => Some(*v),
            Self::Missing(_) => None,
        }
    }

    pub(crate) fn is_known(&self) -> bool {
        matches!(self, Self::Known(_))
    }

    /// The pointers of the missing inputs; empty for a known amount.
    pub(crate) fn missing(&self) -> BTreeSet<String> {
        match self {
            Self::Known(_) => BTreeSet::new(),
            Self::Missing(m) => m.clone(),
        }
    }

    /// The signed sum of `terms`: missing when a term is missing, with the pointers of every
    /// missing term (checked arithmetic, `FM-E008` on overflow).
    pub(crate) fn sum(terms: &[(Sign, &Partial)]) -> Result<Partial> {
        let mut missing = BTreeSet::new();
        let mut total = Decimal::ZERO;
        for (sign, term) in terms {
            match term {
                Self::Missing(m) => missing.extend(m.iter().cloned()),
                Self::Known(v) if missing.is_empty() => {
                    total = match sign {
                        Sign::Plus => num::add(total, *v)?,
                        Sign::Minus => num::sub(total, *v)?,
                    };
                }
                Self::Known(_) => {}
            }
        }
        Ok(if missing.is_empty() {
            Self::Known(total)
        } else {
            Self::Missing(missing)
        })
    }

    /// Σ `terms`, all added.
    pub(crate) fn plus_all<'a>(terms: impl IntoIterator<Item = &'a Partial>) -> Result<Partial> {
        let terms: Vec<(Sign, &Partial)> = terms.into_iter().map(|t| (Sign::Plus, t)).collect();
        Self::sum(&terms)
    }

    /// `−self`.
    pub(crate) fn negated(&self) -> Result<Partial> {
        Self::sum(&[(Sign::Minus, self)])
    }
}

/// The signed sum of named terms with its explain step: "label: a 10 + b 20 − c 5", value 25.
/// The step is written only when the total is known; a missing total is reported in
/// `not_computed` instead.
pub(crate) fn total(
    ex: &mut Explain,
    label: &str,
    terms: &[(Sign, &str, &Partial)],
) -> Result<Partial> {
    let signed: Vec<(Sign, &Partial)> = terms.iter().map(|(s, _, p)| (*s, *p)).collect();
    let result = Partial::sum(&signed)?;
    if let Partial::Known(value) = result {
        let mut formula = String::new();
        for (k, (sign, name, term)) in terms.iter().enumerate() {
            let v = term.value().unwrap_or_default();
            match (k, sign) {
                (0, Sign::Plus) => formula.push_str(&format!("{name} {v}")),
                (0, Sign::Minus) => formula.push_str(&format!("− {name} {v}")),
                (_, Sign::Plus) => formula.push_str(&format!(" + {name} {v}")),
                (_, Sign::Minus) => formula.push_str(&format!(" − {name} {v}")),
            }
        }
        if terms.is_empty() {
            formula.push_str("none");
        }
        ex.step(label, formula, value);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(v: i64) -> Partial {
        Partial::Known(Decimal::from(v))
    }

    #[test]
    fn a_missing_term_makes_the_sum_missing_with_every_pointer() {
        let a = Partial::lacking("/b");
        let b = Partial::lacking("/a");
        let s = Partial::sum(&[(Sign::Plus, &k(1)), (Sign::Minus, &a), (Sign::Plus, &b)]).unwrap();
        assert_eq!(
            s.missing().into_iter().collect::<Vec<_>>(),
            ["/a".to_owned(), "/b".to_owned()]
        );
        assert_eq!(s.value(), None);
    }

    #[test]
    fn known_terms_are_added_exactly_with_their_signs() {
        let s = Partial::sum(&[(Sign::Plus, &k(10)), (Sign::Minus, &k(3))]).unwrap();
        assert_eq!(s, k(7));
        assert_eq!(Partial::plus_all([]).unwrap(), Partial::ZERO);
        assert_eq!(k(4).negated().unwrap(), k(-4));
    }

    #[test]
    fn the_step_substitutes_every_term() {
        let mut ex = Explain::new("t", "t");
        let t = total(
            &mut ex,
            "Invested capital",
            &[
                (Sign::Plus, "operating fixed assets", &k(100)),
                (Sign::Plus, "operating working capital", &k(50)),
                (Sign::Minus, "operating provisions", &k(30)),
            ],
        )
        .unwrap();
        assert_eq!(t, k(120));
        assert_eq!(
            ex.steps[0].formula,
            "operating fixed assets 100 + operating working capital 50 − operating provisions 30"
        );
        let missing = total(&mut ex, "x", &[(Sign::Plus, "a", &Partial::lacking("/a"))]).unwrap();
        assert!(!missing.is_known());
        assert_eq!(ex.steps.len(), 1);
    }

    #[test]
    fn overflow_is_fm_e008() {
        let max = Partial::Known(Decimal::MAX);
        let e = Partial::sum(&[(Sign::Plus, &max), (Sign::Plus, &max)]).unwrap_err();
        assert_eq!(e.code(), "FM-E008");
    }
}
