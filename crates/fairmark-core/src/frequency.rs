//! Named frequencies and period lengths (SPEC-0010 R3, R20).
//!
//! The one definition of the named frequencies of the engine: each name has a number of periods
//! a year and a tenor in months, as the QuantLib `Frequency` values map to `Period` tenors. The
//! compounding list (`rates::Compounding`), the loan payment frequency
//! (`loans::PaymentFrequency`) and the schedules of `dates.schedule` and `series.project` take
//! their periodic values from here, so the lists cannot drift.
//!
//! A period length is given either as a named frequency or as a whole number of months
//! (`period_months`), which also covers tenors without a common name (5, 18 or 24 months).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::{FairmarkError, Reason, Result};

/// Largest number of periods of a schedule, a series or an annuity (bounds every loop over
/// periods). Defined here, with the frequencies, so that the time model does not depend on a
/// method theme.
pub const MAX_PERIODS: u32 = 10_000;

/// `FM-E001` at `pointer` unless `1 ≤ n ≤ MAX_PERIODS`; `purpose` completes the fix ("give the
/// number of {purpose}").
pub(crate) fn check_number_of_periods(n: u32, pointer: &str, purpose: &str) -> Result<()> {
    if n == 0 || n > MAX_PERIODS {
        return Err(FairmarkError::InvalidInputAt {
            pointer: pointer.to_owned(),
            detail: format!(
                "must be between 1 and {MAX_PERIODS} (got {n}); give the number of {purpose}"
            ),
        }
        .because(Reason::Range)
        .expected([format!("a whole number from 1 to {MAX_PERIODS}")]));
    }
    Ok(())
}

/// A named frequency: periods a year and months per period.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Frequency {
    /// Once a year: 1 period a year, 12 months per period.
    Annual,
    /// Twice a year: 2 periods a year, 6 months per period.
    SemiAnnual,
    /// Every four months: 3 periods a year, 4 months per period.
    FourMonthly,
    /// Every three months: 4 periods a year, 3 months per period.
    Quarterly,
    /// Every two months (not twice a month): 6 periods a year, 2 months per period.
    Bimonthly,
    /// Every month: 12 periods a year, 1 month per period.
    Monthly,
}

impl Frequency {
    /// Every named frequency, from the longest period to the shortest.
    pub const ALL: [Frequency; 6] = [
        Self::Annual,
        Self::SemiAnnual,
        Self::FourMonthly,
        Self::Quarterly,
        Self::Bimonthly,
        Self::Monthly,
    ];

    /// Periods a year.
    pub const fn per_year(self) -> u32 {
        match self {
            Self::Annual => 1,
            Self::SemiAnnual => 2,
            Self::FourMonthly => 3,
            Self::Quarterly => 4,
            Self::Bimonthly => 6,
            Self::Monthly => 12,
        }
    }

    /// Months per period (the tenor).
    pub const fn months(self) -> u32 {
        12 / self.per_year()
    }

    /// The name used in inputs, for example `four_monthly`.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Annual => "annual",
            Self::SemiAnnual => "semi_annual",
            Self::FourMonthly => "four_monthly",
            Self::Quarterly => "quarterly",
            Self::Bimonthly => "bimonthly",
            Self::Monthly => "monthly",
        }
    }

    /// The named frequency whose tenor is `months`, if there is one.
    pub fn from_months(months: u32) -> Option<Frequency> {
        Self::ALL.into_iter().find(|f| f.months() == months)
    }
}

/// The length of the periods of a schedule (R3): a named frequency or a whole number of months.
/// Built by validation from the two input fields, never deserialised. A named frequency and the
/// equal number of months behave identically; only the description of the form differs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeriodLength {
    /// Given as `frequency`.
    Named(Frequency),
    /// Given as `period_months`, a positive whole number.
    Months(u32),
}

impl PeriodLength {
    /// Months per period.
    pub fn months(self) -> u32 {
        match self {
            Self::Named(f) => f.months(),
            Self::Months(m) => m,
        }
    }

    /// Periods a year, only when the months per period divide 12.
    pub fn per_year(self) -> Option<u32> {
        let m = self.months();
        (m > 0 && 12u32.is_multiple_of(m)).then_some(12 / m)
    }

    /// Years per period, only when the months per period are a multiple of 12.
    pub fn years(self) -> Option<u32> {
        let m = self.months();
        (m > 0 && m.is_multiple_of(12)).then_some(m / 12)
    }

    /// The form that gave the length, for `explain.conventions`.
    pub fn source(self) -> String {
        match self {
            Self::Named(f) => format!("given as `frequency` `{}`", f.name()),
            Self::Months(m) => format!("given as `period_months` {m}"),
        }
    }

    /// One line for `explain.conventions`: months per period, the form that gave them and,
    /// when defined, the periods a year.
    pub fn describe(self) -> String {
        let m = self.months();
        let unit = if m == 1 { "month" } else { "months" };
        let per_year = match self.per_year() {
            Some(p) => format!(", {p} periods a year"),
            None => match self.years() {
                Some(k) => format!(", {k} years per period (no whole number of periods a year)"),
                None => " (no whole number of periods a year)".to_owned(),
            },
        };
        format!(
            "period length: {m} {unit} per period, {}{per_year}",
            self.source()
        )
    }
}

/// The period length of an input that gives it in two optional fields, `frequency` and
/// `period_months`, at the JSON pointer `prefix` (for example `` or `/timeline/periods`).
/// With `required`, exactly one must be given (`dates.schedule`); otherwise at most one (the
/// `periods` timeline of `series.project`), and `None` means no period length. Both, or neither
/// when required, and a `period_months` of 0 are `FM-E001` naming the pointers with the fix.
pub(crate) fn period_length(
    frequency: Option<Frequency>,
    period_months: Option<u32>,
    prefix: &str,
    required: bool,
) -> Result<Option<PeriodLength>> {
    // The form first; then a number of months given is checked whatever the form.
    let mut found = crate::error::Found::new();
    match (frequency, period_months) {
        (Some(_), Some(_)) => found.push(
            FairmarkError::InvalidInput(format!(
                "`{prefix}/frequency` and `{prefix}/period_months` are both given; give exactly \
                 one: a named frequency, or the number of months per period (for example \
                 `quarterly` or 3), and remove the other"
            ))
            .at(format!("{prefix}/frequency"), Reason::Conflict)
            .between([
                format!("{prefix}/frequency"),
                format!("{prefix}/period_months"),
            ]),
        ),
        (None, None) if required => found.push(
            FairmarkError::InvalidInput(format!(
                "no period length: give exactly one of `{prefix}/frequency` (`monthly`, \
                 `bimonthly`, `quarterly`, `four_monthly`, `semi_annual` or `annual`) and \
                 `{prefix}/period_months` (a positive whole number of months per period)"
            ))
            .at(format!("{prefix}/frequency"), Reason::Missing)
            .between([
                format!("{prefix}/frequency"),
                format!("{prefix}/period_months"),
            ]),
        ),
        _ => {}
    }
    if period_months == Some(0) {
        found.push(
            FairmarkError::InvalidInputAt {
                pointer: format!("{prefix}/period_months"),
                detail: "must be a positive whole number of months per period (got 0); give \
                         for example 3 for quarterly periods or 18 for periods of eighteen \
                         months"
                    .to_owned(),
            }
            .because(Reason::Range)
            .expected(["a whole number of months, 1 or more"]),
        );
    }
    found.finish()?;
    Ok(match (frequency, period_months) {
        (Some(f), None) => Some(PeriodLength::Named(f)),
        (None, Some(m)) => Some(PeriodLength::Months(m)),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loans::PaymentFrequency;
    use crate::rates::Compounding;

    /// R3: the six names map to 1, 2, 3, 4, 6 and 12 months, and back.
    #[test]
    fn names_map_to_their_tenors() {
        let tenors: Vec<(&str, u32, u32)> = Frequency::ALL
            .iter()
            .map(|f| (f.name(), f.months(), f.per_year()))
            .collect();
        assert_eq!(
            tenors,
            [
                ("annual", 12, 1),
                ("semi_annual", 6, 2),
                ("four_monthly", 4, 3),
                ("quarterly", 3, 4),
                ("bimonthly", 2, 6),
                ("monthly", 1, 12),
            ]
        );
        for f in Frequency::ALL {
            assert_eq!(Frequency::from_months(f.months()), Some(f));
            assert_eq!(f.per_year() * f.months(), 12);
            let json = serde_json::to_value(f).unwrap();
            assert_eq!(json, f.name());
        }
        assert_eq!(Frequency::from_months(5), None);
    }

    /// R20: the compounding list and the payment frequency take their periodic values from
    /// `Frequency`, and agree with it on the periods a year, so the three lists cannot drift.
    #[test]
    fn compounding_and_payment_frequency_agree_with_the_frequencies() {
        let compounding = [
            Compounding::Annual,
            Compounding::SemiAnnual,
            Compounding::Quarterly,
            Compounding::Monthly,
            Compounding::Continuous,
            Compounding::Bimonthly,
            Compounding::FourMonthly,
        ];
        let mut periodic = Vec::new();
        for c in compounding {
            match c.frequency() {
                Some(f) => {
                    assert_eq!(c.per_year(), Some(f.per_year()), "{c:?}");
                    assert_eq!(serde_json::to_value(c).unwrap(), f.name());
                    periodic.push(f);
                }
                None => assert_eq!(c, Compounding::Continuous),
            }
        }
        periodic.sort_by_key(|f| f.per_year());
        assert_eq!(periodic, Frequency::ALL);
        for f in Frequency::ALL {
            assert_eq!(Compounding::of(f).frequency(), Some(f), "{f:?}");
        }

        let payment = [
            PaymentFrequency::Annual,
            PaymentFrequency::SemiAnnual,
            PaymentFrequency::Quarterly,
            PaymentFrequency::Monthly,
            PaymentFrequency::Bimonthly,
            PaymentFrequency::FourMonthly,
        ];
        let mut frequencies: Vec<Frequency> = payment.iter().map(|p| p.frequency()).collect();
        for p in payment {
            assert_eq!(serde_json::to_value(p).unwrap(), p.frequency().name());
            assert_eq!(p.compounding().frequency(), Some(p.frequency()));
        }
        frequencies.sort_by_key(|f| f.per_year());
        assert_eq!(frequencies, Frequency::ALL);
    }

    #[test]
    fn period_lengths_report_their_form_and_periods() {
        let q = PeriodLength::Named(Frequency::Quarterly);
        let m3 = PeriodLength::Months(3);
        assert_eq!((q.months(), q.per_year(), q.years()), (3, Some(4), None));
        assert_eq!((m3.months(), m3.per_year(), m3.years()), (3, Some(4), None));
        assert_eq!(
            q.describe(),
            "period length: 3 months per period, given as `frequency` `quarterly`, 4 periods a year"
        );
        assert_eq!(
            m3.describe(),
            "period length: 3 months per period, given as `period_months` 3, 4 periods a year"
        );
        let m24 = PeriodLength::Months(24);
        assert_eq!((m24.per_year(), m24.years()), (None, Some(2)));
        let m5 = PeriodLength::Months(5);
        assert_eq!((m5.per_year(), m5.years()), (None, None));
        assert!(m5
            .describe()
            .ends_with("(no whole number of periods a year)"));
        assert_eq!(
            PeriodLength::Months(1).describe(),
            "period length: 1 month per period, given as `period_months` 1, 12 periods a year"
        );
    }

    #[test]
    fn number_of_periods_is_checked_with_its_pointer() {
        assert_eq!(check_number_of_periods(1, "/n", "x"), Ok(()));
        assert_eq!(check_number_of_periods(MAX_PERIODS, "/n", "x"), Ok(()));
        for n in [0, MAX_PERIODS + 1] {
            let e = check_number_of_periods(n, "/a/number_of_periods", "periods to project")
                .unwrap_err();
            assert_eq!(e.code(), "FM-E001");
            assert_eq!(
                e.to_string(),
                format!(
                    "invalid input at `/a/number_of_periods`: must be between 1 and 10000 (got \
                     {n}); give the number of periods to project"
                )
            );
        }
    }

    #[test]
    fn exactly_one_or_at_most_one_form() {
        let both = period_length(Some(Frequency::Monthly), Some(1), "", true).unwrap_err();
        assert_eq!(both.code(), "FM-E001");
        let m = both.to_string();
        assert!(
            m.contains("`/frequency`") && m.contains("`/period_months`"),
            "{m}"
        );
        assert!(m.contains("give exactly one"), "{m}");
        let neither = period_length(None, None, "", true).unwrap_err();
        let m = neither.to_string();
        assert!(
            m.contains("`/frequency`") && m.contains("`/period_months`"),
            "{m}"
        );
        assert_eq!(
            period_length(None, None, "/timeline/periods", false),
            Ok(None)
        );
        let both = period_length(
            Some(Frequency::Monthly),
            Some(1),
            "/timeline/periods",
            false,
        )
        .unwrap_err()
        .to_string();
        assert!(both.contains("`/timeline/periods/frequency`"), "{both}");
        let zero = period_length(None, Some(0), "", true).unwrap_err();
        assert_eq!(zero.code(), "FM-E001");
        assert!(zero
            .to_string()
            .starts_with("invalid input at `/period_months`"));
        assert_eq!(
            period_length(None, Some(18), "", true),
            Ok(Some(PeriodLength::Months(18)))
        );
    }
}
