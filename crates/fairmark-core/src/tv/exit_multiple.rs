//! `tv.exit_multiple`: the business sold at a multiple of a metric at N (SPEC-0006 R4).

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    check_growth_cap, checked, domain, invalid, on, Checked, GrowthCapCheck, TerminalBasis,
    NOT_DISCOUNTED,
};
use crate::error::Result;
use crate::explain::Explain;
use crate::method::Method;
use crate::num::{div, mul, sub};

/// The metric a multiple applies to. Enterprise multiples (EV/EBITDA, EV/EBIT, EV/Sales) give
/// an enterprise value; P/E and P/BV give an equity value. The two are never mixed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MultipleMetric {
    /// EBITDA at N; multiple EV/EBITDA, enterprise basis.
    Ebitda,
    /// EBIT at N; multiple EV/EBIT, enterprise basis.
    Ebit,
    /// Revenue at N; multiple EV/Sales, enterprise basis.
    Revenue,
    /// Net earnings at N; multiple P/E, equity basis.
    Earnings,
    /// Book value of equity at N; multiple P/BV (price to book value), equity basis
    /// (SPEC-0006 decision of 2026-11-08).
    BookValue,
}

impl MultipleMetric {
    pub fn basis(self) -> TerminalBasis {
        match self {
            Self::Earnings | Self::BookValue => TerminalBasis::Equity,
            Self::Ebitda | Self::Ebit | Self::Revenue => TerminalBasis::Enterprise,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Ebitda => "ebitda",
            Self::Ebit => "ebit",
            Self::Revenue => "revenue",
            Self::Earnings => "earnings",
            Self::BookValue => "book_value",
        }
    }

    pub(crate) fn multiple_name(self) -> &'static str {
        match self {
            Self::Ebitda => "EV/EBITDA",
            Self::Ebit => "EV/EBIT",
            Self::Revenue => "EV/Sales",
            Self::Earnings => "P/E",
            Self::BookValue => "P/BV",
        }
    }
}

/// Input of `tv.exit_multiple`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExitMultipleInput {
    /// The multiple, a pure number greater than 0 (for example 8 for 8 × EBITDA).
    pub multiple: Decimal,
    /// `ebitda`, `ebit`, `revenue` (enterprise basis), `earnings` or `book_value` (equity
    /// basis).
    pub metric: MultipleMetric,
    /// The metric at the horizon N, in currency units, greater than 0.
    pub metric_value: Decimal,
    /// Discount rate of the terminal phase, decimal fraction per period, on the basis of the
    /// metric (WACC for enterprise, cost of equity for equity). Optional, with `next_flow`:
    /// both or neither. Used only for the implied growth.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate: Option<Decimal>,
    /// Flow at N + 1 on the basis of the metric (FCFF for enterprise, FCFE for equity), in
    /// currency units. Optional, with `rate`: both or neither.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_flow: Option<Decimal>,
    /// Upper bound for the implied growth, decimal fraction per period. Default: none,
    /// reported as absent. Needs `rate` and `next_flow`. Exceeding it is reported, not refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub growth_cap: Option<Decimal>,
}

/// The terms of `tv.exit_multiple` inside `tv.compare`: the rate and the growth cap are shared
/// and used when `next_flow` is given.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExitMultipleTerms {
    /// The multiple, greater than 0.
    pub multiple: Decimal,
    /// `ebitda`, `ebit`, `revenue` (enterprise basis), `earnings` or `book_value` (equity
    /// basis); must match the basis of the comparison.
    pub metric: MultipleMetric,
    /// The metric at N, in currency units, greater than 0.
    pub metric_value: Decimal,
    /// Flow at N + 1, in currency units. Optional: when given, the implied growth is computed
    /// at the shared rate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_flow: Option<Decimal>,
}

impl ExitMultipleTerms {
    pub(crate) fn with(&self, rate: Decimal, growth_cap: Option<Decimal>) -> ExitMultipleInput {
        let implied = self.next_flow.is_some();
        ExitMultipleInput {
            multiple: self.multiple,
            metric: self.metric,
            metric_value: self.metric_value,
            rate: implied.then_some(rate),
            next_flow: self.next_flow,
            growth_cap: growth_cap.filter(|_| implied),
        }
    }
}

/// Output of `tv.exit_multiple`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ExitMultipleOutput {
    /// Terminal value at N, not discounted, in currency units.
    pub terminal_value: Decimal,
    /// `enterprise` (EV/EBITDA, EV/EBIT, EV/Sales) or `equity` (P/E, P/BV).
    pub basis: TerminalBasis,
    /// Perpetual growth implied by the multiple, r − CF_{N+1} / TV_N, decimal fraction per
    /// period; `null` when `rate` and `next_flow` are not given.
    pub implied_growth: Option<Decimal>,
    /// Implied growth compared with the cap; `null` when no cap is given.
    pub growth_cap: Option<GrowthCapCheck>,
}

/// Validates the input; each error names its field.
pub(crate) fn check(input: &ExitMultipleInput) -> Checked {
    let (m, x, metric) = (input.multiple, input.metric_value, input.metric);
    let mut errors = Vec::new();
    if m <= Decimal::ZERO {
        errors.push(on("multiple")(domain(format!(
            "the multiple must be greater than 0 (got {m})"
        ))));
    }
    if x <= Decimal::ZERO {
        errors.push(on("metric_value")(domain(format!(
            "the metric {} must be greater than 0 to carry a multiple (got {x})",
            metric.name()
        ))));
    }
    if input.rate.is_some() != input.next_flow.is_some() {
        errors.push(on("next_flow")(
            invalid("give both rate and next_flow, or neither")
                .because(crate::error::Reason::Requires)
                .between(["/next_flow", "/rate"]),
        ));
    }
    if input.next_flow.is_none() && input.growth_cap.is_some() {
        errors.push(on("growth_cap")(
            invalid(
                "growth_cap needs rate and next_flow: the cap is checked on the implied growth",
            )
            .because(crate::error::Reason::Requires),
        ));
    }
    checked(errors)
}

/// TV_N = multiple × metric_N; implied growth g = r − CF_{N+1} / TV_N.
pub fn exit_multiple(input: &ExitMultipleInput) -> Result<(ExitMultipleOutput, Explain)> {
    check(input).map_err(super::into_error)?;
    let (m, x, metric) = (input.multiple, input.metric_value, input.metric);
    let pair = input.rate.zip(input.next_flow);
    let basis = metric.basis();
    let mut ex = Explain::new(
        "tv.exit_multiple",
        "TV_N = multiple × metric_N; implied g = r − CF_{N+1} / TV_N",
    );
    ex.convention(NOT_DISCOUNTED);
    ex.convention(match basis {
        TerminalBasis::Enterprise => format!(
            "metric {}: enterprise basis ({}); the terminal value is an enterprise value",
            metric.name(),
            metric.multiple_name()
        ),
        TerminalBasis::Equity => format!(
            "metric {}: equity basis ({}); the terminal value is an equity value, never mixed \
            with enterprise measures",
            metric.name(),
            metric.multiple_name()
        ),
    });
    ex.convention(format!(
        "metric at N (trailing multiple): {} × {} of period N",
        metric.multiple_name(),
        metric.name()
    ));
    let tv = mul(m, x)?;
    ex.step(
        format!("Terminal value at N ({})", metric.multiple_name()),
        format!("{m} × {x}"),
        tv,
    );
    let (implied, cap) = match pair {
        Some((r, cf)) => {
            ex.convention(match basis {
                TerminalBasis::Enterprise => {
                    "implied growth on the enterprise basis: the rate is a WACC and the next flow \
                    a free cash flow to the firm"
                }
                TerminalBasis::Equity => {
                    "implied growth on the equity basis: the rate is a cost of equity and the \
                    next flow a free cash flow to equity"
                }
            });
            let yield_next = div(cf, tv)?;
            ex.step("Flow yield at N + 1", format!("{cf} / {tv}"), yield_next);
            let g = sub(r, yield_next)?;
            ex.step("Implied perpetual growth", format!("{r} − {yield_next}"), g);
            if g >= r {
                // Reported, not refused (open question in plan.md).
                ex.convention(format!(
                    "implied growth {g} is not below the rate {r}: the multiple is inconsistent \
                    with a stable-growth value"
                ));
            }
            let cap = check_growth_cap(&mut ex, "implied growth", g, input.growth_cap);
            (Some(g), cap)
        }
        None => {
            ex.convention(
                "implied growth: not computed (rate and next_flow not given); growth cap: not \
                checked",
            );
            (None, None)
        }
    };
    Ok((
        ExitMultipleOutput {
            terminal_value: tv,
            basis,
            implied_growth: implied,
            growth_cap: cap,
        },
        ex,
    ))
}

/// `tv.exit_multiple`: see the methodology page.
pub struct ExitMultiple;

impl Method for ExitMultiple {
    const ID: &'static str = "tv.exit_multiple";
    const TITLE: &'static str = "Terminal value, exit multiple";
    const DESCRIPTION: &'static str =
        "Terminal value at the horizon N as a multiple of EBITDA, EBIT, revenue (enterprise \
        basis), earnings or book value of equity (equity basis); with a rate and the next flow, \
        reports the implied perpetual growth.";
    const VERSION: &'static str = "1";
    type Input = ExitMultipleInput;
    type Output = ExitMultipleOutput;

    fn validate(input: &Self::Input, _defaults: &[crate::explain::DefaultApplied]) -> Result<()> {
        check(input).map_err(super::into_error)
    }

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        exit_multiple(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tv::{d, gordon, GordonInput};

    fn input(m: &str, x: &str, r: Option<&str>, cf: Option<&str>) -> ExitMultipleInput {
        ExitMultipleInput {
            multiple: d(m),
            metric: MultipleMetric::Ebitda,
            metric_value: d(x),
            rate: r.map(d),
            next_flow: cf.map(d),
            growth_cap: None,
        }
    }

    /// AC4: the implied growth fed back into `tv.gordon` returns the same terminal value.
    #[test]
    fn ac4_implied_growth_fed_back_into_gordon() {
        for (m, x, r, cf) in [
            ("8", "125", "0.10", "60"),
            ("7.5", "180", "0.09", "95"),
            ("11.3", "47.9", "0.083", "21.7"),
            ("5", "300", "0.12", "100"),
        ] {
            let (exit, _) = exit_multiple(&input(m, x, Some(r), Some(cf))).unwrap();
            let (g, _) = gordon(&GordonInput {
                rate: d(r),
                growth: exit.implied_growth.unwrap(),
                next_flow: Some(d(cf)),
                last_flow: None,
                growth_cap: None,
            })
            .unwrap();
            let diff = (g.terminal_value - exit.terminal_value).abs();
            assert!(diff <= d("0.000000000000000001"), "{m} × {x}: {diff}");
        }
    }

    #[test]
    fn implied_growth_not_below_the_rate_is_reported() {
        let (out, ex) = exit_multiple(&input("8", "125", Some("0.10"), Some("-5"))).unwrap();
        assert!(out.implied_growth.unwrap() >= d("0.10"));
        assert!(ex
            .conventions
            .iter()
            .any(|c| c.ends_with("inconsistent with a stable-growth value")));
        assert!(ex
            .conventions
            .iter()
            .any(|c| c.contains("trailing multiple")));
    }

    #[test]
    fn basis_follows_the_metric() {
        assert_eq!(MultipleMetric::Earnings.basis(), TerminalBasis::Equity);
        assert_eq!(MultipleMetric::BookValue.basis(), TerminalBasis::Equity);
        for m in [
            MultipleMetric::Ebitda,
            MultipleMetric::Ebit,
            MultipleMetric::Revenue,
        ] {
            assert_eq!(m.basis(), TerminalBasis::Enterprise);
        }
    }

    #[test]
    fn invalid_inputs_are_refused() {
        let code = |i: ExitMultipleInput| exit_multiple(&i).unwrap_err().code();
        assert_eq!(code(input("0", "100", None, None)), "FM-E018");
        assert_eq!(code(input("6", "0", None, None)), "FM-E018");
        assert_eq!(code(input("6", "100", Some("0.1"), None)), "FM-E001");
        let mut i = input("6", "100", None, None);
        i.growth_cap = Some(d("0.02"));
        assert_eq!(code(i), "FM-E001");
    }
}
