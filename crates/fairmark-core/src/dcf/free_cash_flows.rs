//! `dcf.free_cash_flows`: free cash flow build-up from reclassified line items (SPEC-0001 R2 to
//! R5).
//!
//! FCFF_t = EBIT_t − taxes on EBIT_t + D&A_t − capex_t − ΔNWC_t + Σ adjustments_t and, with the
//! financing lines, FCFE_t = FCFF_t − interest_t × (1 − t) + net borrowing_t. Amounts are in
//! currency units per period and are not discounted.
//!
//! The types and [`free_cash_flows`] are public so that the shared DCF engine reuses the
//! build-up for a `given` stage through a typed call (ADR 0009).

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::{FairmarkError, Result};
use crate::explain::Explain;
use crate::method::Method;
use crate::num::{add, mul, sub};

/// A signed, labelled amount added to the free cash flow of a period.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Adjustment {
    /// What the amount is, for example "Disposal of a warehouse". Not empty.
    pub label: String,
    /// Amount in currency units: positive for an inflow (for example proceeds from a disposal),
    /// negative for an outflow.
    pub value: Decimal,
}

/// The line items of one period, after reclassification (SPEC-0005).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FreeCashFlowLines {
    /// EBIT (earnings before interest and taxes) of the period, in currency units; any sign.
    pub ebit: Decimal,
    /// Taxes on EBIT of the period, in currency units, positive for a tax paid (amounts form).
    /// Give it for every period and omit `notional_tax_rate`, or omit it everywhere and give
    /// `notional_tax_rate`. Default: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operating_taxes: Option<Decimal>,
    /// Depreciation and amortisation of the period, in currency units, zero or more; added back.
    pub depreciation_and_amortisation: Decimal,
    /// Capital expenditure of the period, in currency units, zero or more; subtracted.
    /// Disposals go in `other_adjustments` with a positive sign.
    pub capital_expenditure: Decimal,
    /// Change in net working capital of the period, in currency units: positive for an
    /// increase, which absorbs cash and is subtracted; negative for a decrease.
    pub change_in_net_working_capital: Decimal,
    /// Other labelled adjustments, each positive for an inflow and negative for an outflow.
    /// Default: none.
    #[serde(default)]
    pub other_adjustments: Vec<Adjustment>,
    /// Interest expense of the period, in currency units, zero or more. Give it with
    /// `net_borrowing`, for every period or for none, to obtain the FCFE. Default: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interest_expense: Option<Decimal>,
    /// Net borrowing of the period, in currency units: positive for new borrowing, negative for
    /// net repayment. Give it with `interest_expense`. Default: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub net_borrowing: Option<Decimal>,
}

/// Tax on a negative EBIT when the taxes are computed from a rate (SPEC-0001 R4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NegativeEbit {
    /// Taxes = rate × EBIT also when EBIT is negative: a tax saving that raises the flow.
    Credit,
    /// No tax when EBIT is negative.
    Zero,
}

impl NegativeEbit {
    fn name(self) -> &'static str {
        match self {
            Self::Credit => "credit",
            Self::Zero => "zero",
        }
    }
}

/// How the taxes on EBIT were given (SPEC-0001 Decision 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TaxForm {
    /// `notional_tax_rate` applied to the EBIT of every period.
    RateOnEbit,
    /// `operating_taxes` given for every period.
    Amounts,
}

/// Input of `dcf.free_cash_flows`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FreeCashFlowsInput {
    /// Line items of periods 1..n, in order; at least one period.
    pub periods: Vec<FreeCashFlowLines>,
    /// Tax rate applied to EBIT, decimal fraction from 0 to below 1 (0.24 = 24%) (rate form).
    /// Give this or `operating_taxes` in every period, not both. Default: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notional_tax_rate: Option<Decimal>,
    /// Treatment of a negative EBIT in the rate form: `credit` (taxes = rate × EBIT) or `zero`
    /// (no tax). Required with `notional_tax_rate`, there is no default; not accepted with
    /// `operating_taxes`. Tax loss carry-forwards are not modelled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub negative_ebit: Option<NegativeEbit>,
    /// Tax rate t of the interest tax shield in the FCFE, decimal fraction from 0 to below 1.
    /// Required when the taxes are given as `operating_taxes` and the financing lines are
    /// given; not accepted otherwise (with `notional_tax_rate` the shield uses that rate).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interest_tax_rate: Option<Decimal>,
}

/// The build-up of one period, every component in currency units.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct FreeCashFlowPeriod {
    /// Period number, from 1.
    pub period: u32,
    /// EBIT as given.
    pub ebit: Decimal,
    /// Taxes on EBIT used: computed from the rate, or as given.
    pub operating_taxes: Decimal,
    /// EBIT − taxes on EBIT.
    pub ebit_after_taxes: Decimal,
    /// Depreciation and amortisation as given.
    pub depreciation_and_amortisation: Decimal,
    /// Capital expenditure as given.
    pub capital_expenditure: Decimal,
    /// Change in net working capital as given (positive for an increase).
    pub change_in_net_working_capital: Decimal,
    /// Labelled adjustments as given.
    pub other_adjustments: Vec<Adjustment>,
    /// Sum of the adjustments (0 when none is given).
    pub other_adjustments_total: Decimal,
    /// FCFF of the period.
    pub free_cash_flow_to_firm: Decimal,
    /// Interest expense as given; `null` without financing lines.
    pub interest_expense: Option<Decimal>,
    /// Interest expense × (1 − t); `null` without financing lines.
    pub interest_after_taxes: Option<Decimal>,
    /// Net borrowing as given; `null` without financing lines.
    pub net_borrowing: Option<Decimal>,
    /// FCFE of the period; `null` without financing lines.
    pub free_cash_flow_to_equity: Option<Decimal>,
}

/// Output of `dcf.free_cash_flows`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct FreeCashFlowsOutput {
    /// The build-up of every period.
    pub periods: Vec<FreeCashFlowPeriod>,
    /// FCFF of periods 1..n, in currency units, not discounted.
    pub free_cash_flows_to_firm: Vec<Decimal>,
    /// FCFE of periods 1..n, in currency units, not discounted; `null` without financing lines.
    pub free_cash_flows_to_equity: Option<Vec<Decimal>>,
    /// `rate_on_ebit` or `amounts`.
    pub tax_form: TaxForm,
    /// Negative EBIT treatment of the rate form; `null` in the amounts form.
    pub negative_ebit: Option<NegativeEbit>,
    /// Rate t of the interest tax shield used for the FCFE; `null` without financing lines.
    pub interest_tax_rate: Option<Decimal>,
}

fn invalid(msg: impl Into<String>) -> FairmarkError {
    FairmarkError::InvalidInput(msg.into())
}

fn check_tax_rate(t: Decimal, pointer: &str) -> Result<()> {
    if t < Decimal::ZERO || t >= Decimal::ONE {
        return Err(invalid(format!(
            "`{pointer}` must be at least 0 and below 1, as a decimal fraction (got {t}); \
             write 24% as 0.24"
        )));
    }
    Ok(())
}

/// R5: D&A, capex and interest are amounts of zero or more.
fn check_not_negative(value: Decimal, index: usize, field: &str) -> Result<()> {
    if value < Decimal::ZERO {
        return Err(invalid(format!(
            "`/periods/{index}/{field}` (period {}) must be zero or more (got {value}); \
             enter disposals and other inflows in `/periods/{index}/other_adjustments` with a \
             positive sign",
            index + 1
        )));
    }
    Ok(())
}

/// The tax choice after validation.
#[derive(Clone, Copy)]
enum Taxes {
    Rate(Decimal, NegativeEbit),
    Amounts,
}

/// Decision 7, R3 and R4: exactly one tax form, the treatment of a negative EBIT with the rate
/// form only, the interest tax rate only where it is needed.
fn validate(input: &FreeCashFlowsInput) -> Result<(Taxes, bool, Option<Decimal>)> {
    let p = &input.periods;
    if p.is_empty() {
        return Err(invalid(
            "`/periods` must contain at least one period with its line items",
        ));
    }
    let with_amounts: Vec<usize> = (0..p.len())
        .filter(|&i| p[i].operating_taxes.is_some())
        .collect();
    let taxes = match input.notional_tax_rate {
        Some(t) => {
            if let Some(&i) = with_amounts.first() {
                return Err(invalid(format!(
                    "give the taxes on EBIT in one form only: `/notional_tax_rate` conflicts with \
                     `/periods/{i}/operating_taxes`; remove `notional_tax_rate`, or remove \
                     `operating_taxes` from every period"
                )));
            }
            check_tax_rate(t, "/notional_tax_rate")?;
            let Some(treatment) = input.negative_ebit else {
                return Err(invalid(
                    "`/negative_ebit` is required with `/notional_tax_rate`: `credit` (taxes = \
                     rate × EBIT, a saving when EBIT is negative) or `zero` (no tax when EBIT is \
                     negative); there is no default",
                ));
            };
            Taxes::Rate(t, treatment)
        }
        None => {
            if with_amounts.is_empty() {
                return Err(invalid(
                    "no taxes on EBIT: give `/notional_tax_rate` (with `/negative_ebit`), or \
                     `operating_taxes` in every period",
                ));
            }
            if let Some(i) = (0..p.len()).find(|&i| p[i].operating_taxes.is_none()) {
                return Err(invalid(format!(
                    "`/periods/{i}/operating_taxes` (period {}) is missing: give the taxes on \
                     EBIT as amounts in every period, or remove them and give \
                     `/notional_tax_rate`",
                    i + 1
                )));
            }
            if input.negative_ebit.is_some() {
                return Err(invalid(
                    "`/negative_ebit` applies only with `/notional_tax_rate`: the taxes are given \
                     as amounts in `operating_taxes`; remove `negative_ebit`",
                ));
            }
            Taxes::Amounts
        }
    };

    for (i, lines) in p.iter().enumerate() {
        check_not_negative(
            lines.depreciation_and_amortisation,
            i,
            "depreciation_and_amortisation",
        )?;
        check_not_negative(lines.capital_expenditure, i, "capital_expenditure")?;
        if let Some(interest) = lines.interest_expense {
            check_not_negative(interest, i, "interest_expense")?;
        }
        for (k, a) in lines.other_adjustments.iter().enumerate() {
            if a.label.trim().is_empty() {
                return Err(invalid(format!(
                    "`/periods/{i}/other_adjustments/{k}/label` (period {}) is empty: say what \
                     the amount is",
                    i + 1
                )));
            }
        }
        match (lines.interest_expense, lines.net_borrowing) {
            (Some(_), None) => {
                return Err(invalid(format!(
                    "`/periods/{i}/net_borrowing` (period {}) is missing: the FCFE needs \
                     `interest_expense` and `net_borrowing` together; give it (0 for none) or \
                     remove `interest_expense`",
                    i + 1
                )))
            }
            (None, Some(_)) => {
                return Err(invalid(format!(
                    "`/periods/{i}/interest_expense` (period {}) is missing: the FCFE needs \
                     `interest_expense` and `net_borrowing` together; give it (0 for none) or \
                     remove `net_borrowing`",
                    i + 1
                )))
            }
            _ => {}
        }
    }
    let financing = p[0].interest_expense.is_some();
    if let Some(i) = (0..p.len()).find(|&i| p[i].interest_expense.is_some() != financing) {
        let (given, missing) = if financing { (0, i) } else { (i, 0) };
        return Err(invalid(format!(
            "financing lines are given for period {} but not for period {}: give \
             `interest_expense` and `net_borrowing` in every period (from `/periods/{missing}`) \
             to obtain the FCFE, or in none",
            given + 1,
            missing + 1
        )));
    }

    let shield =
        match (taxes, financing, input.interest_tax_rate) {
            (Taxes::Rate(..), _, Some(_)) => return Err(invalid(
                "`/interest_tax_rate` is not accepted with `/notional_tax_rate`: the interest tax \
                 shield uses the tax rate on EBIT (SPEC-0001 R3); remove `interest_tax_rate`",
            )),
            (Taxes::Rate(t, _), true, None) => Some(t),
            (Taxes::Amounts, true, Some(t)) => {
                check_tax_rate(t, "/interest_tax_rate")?;
                Some(t)
            }
            (Taxes::Amounts, true, None) => {
                return Err(invalid(
                    "`/interest_tax_rate` is required: the taxes on EBIT are given as amounts, so \
                 the rate t of the interest tax shield in the FCFE must be given separately",
                ))
            }
            (Taxes::Amounts, false, Some(_)) => {
                return Err(invalid(
                    "`/interest_tax_rate` is used only for the FCFE: give `interest_expense` and \
                 `net_borrowing` in every period, or remove `interest_tax_rate`",
                ))
            }
            (_, false, None) => None,
        };
    Ok((taxes, financing, shield))
}

/// Builds FCFF, and FCFE when the financing lines are given, for every period.
pub fn free_cash_flows(input: &FreeCashFlowsInput) -> Result<(FreeCashFlowsOutput, Explain)> {
    let (taxes, financing, shield) = validate(input)?;
    let formula = if financing {
        "FCFF_t = EBIT_t − taxes on EBIT_t + D&A_t − capex_t − ΔNWC_t + Σ adjustments_t;  \
         FCFE_t = FCFF_t − interest_t × (1 − t) + net borrowing_t"
    } else {
        "FCFF_t = EBIT_t − taxes on EBIT_t + D&A_t − capex_t − ΔNWC_t + Σ adjustments_t"
    };
    let mut ex = Explain::new("dcf.free_cash_flows", formula);
    match taxes {
        Taxes::Rate(t, treatment) => {
            ex.convention(format!(
                "tax form: rate on EBIT, taxes on EBIT_t = {t} × EBIT_t"
            ));
            ex.convention(match treatment {
                NegativeEbit::Credit => {
                    "negative EBIT: credit, taxes = rate × EBIT also when EBIT is negative (a tax \
                     saving)"
                }
                NegativeEbit::Zero => "negative EBIT: zero, no tax when EBIT is negative",
            });
        }
        Taxes::Amounts => {
            ex.convention("tax form: amounts given per period in `operating_taxes`");
        }
    }
    ex.convention("no tax loss carry-forward is modelled");
    ex.convention(
        "change in net working capital is positive for an increase, which absorbs cash and is \
         subtracted",
    );
    ex.convention("adjustments are positive for an inflow and are added");
    match (taxes, shield) {
        (Taxes::Rate(..), Some(t)) => {
            ex.convention(format!(
                "interest tax shield at the tax rate on EBIT {t}: interest after taxes = \
                 interest × (1 − {t}), whatever the sign of EBIT"
            ));
        }
        (Taxes::Amounts, Some(t)) => {
            ex.convention(format!(
                "interest tax shield at the separate interest tax rate {t}: interest after taxes \
                 = interest × (1 − {t})"
            ));
        }
        _ => {}
    }
    if financing {
        ex.convention("net borrowing is positive for new borrowing and is added");
    } else {
        ex.convention("FCFE: not computed (no interest expense and net borrowing given)");
    }
    ex.convention("amounts in currency units per period, not discounted");

    let mut periods = Vec::with_capacity(input.periods.len());
    let mut negative = Vec::new();
    for (i, l) in input.periods.iter().enumerate() {
        let n = i + 1;
        let ebit = l.ebit;
        let tax = match (taxes, l.operating_taxes) {
            (Taxes::Rate(t, treatment), _) => {
                if ebit < Decimal::ZERO {
                    negative.push(n.to_string());
                }
                if ebit < Decimal::ZERO && treatment == NegativeEbit::Zero {
                    ex.step(
                        format!("Taxes on EBIT, period {n}"),
                        format!("0 (EBIT {ebit} negative, treatment zero)"),
                        Decimal::ZERO,
                    );
                    Decimal::ZERO
                } else {
                    let tax = mul(t, ebit)?;
                    ex.step(
                        format!("Taxes on EBIT, period {n}"),
                        format!("{t} × {ebit}"),
                        tax,
                    );
                    tax
                }
            }
            // `validate` has checked that every period carries its amount.
            (Taxes::Amounts, amount) => {
                let tax = amount.ok_or(FairmarkError::Internal("operating taxes"))?;
                ex.step(
                    format!("Taxes on EBIT, period {n}"),
                    format!("{tax} (given)"),
                    tax,
                );
                tax
            }
        };
        let after = sub(ebit, tax)?;
        ex.step(
            format!("EBIT after taxes, period {n}"),
            format!("{ebit} − {tax}"),
            after,
        );
        let mut adjustments = Decimal::ZERO;
        for a in &l.other_adjustments {
            ex.step(
                format!("Adjustment, period {n}: {}", a.label),
                format!("{} (given)", a.value),
                a.value,
            );
            adjustments = add(adjustments, a.value)?;
        }
        if l.other_adjustments.len() > 1 {
            let terms: Vec<String> = l
                .other_adjustments
                .iter()
                .map(|a| a.value.to_string())
                .collect();
            ex.step(
                format!("Adjustments, period {n}"),
                terms.join(" + "),
                adjustments,
            );
        }
        let (da, capex, dnwc) = (
            l.depreciation_and_amortisation,
            l.capital_expenditure,
            l.change_in_net_working_capital,
        );
        let fcff = add(sub(sub(add(after, da)?, capex)?, dnwc)?, adjustments)?;
        ex.step(
            format!("FCFF, period {n}"),
            format!("{ebit} − {tax} + {da} − {capex} − ({dnwc}) + {adjustments}"),
            fcff,
        );
        let (interest_after, fcfe) = match (l.interest_expense, l.net_borrowing, shield) {
            (Some(interest), Some(borrowing), Some(t)) => {
                let after_tax = mul(interest, sub(Decimal::ONE, t)?)?;
                ex.step(
                    format!("Interest after taxes, period {n}"),
                    format!("{interest} × (1 − {t})"),
                    after_tax,
                );
                let fcfe = add(sub(fcff, after_tax)?, borrowing)?;
                ex.step(
                    format!("FCFE, period {n}"),
                    format!("{fcff} − {after_tax} + ({borrowing})"),
                    fcfe,
                );
                (Some(after_tax), Some(fcfe))
            }
            _ => (None, None),
        };
        periods.push(FreeCashFlowPeriod {
            period: u32::try_from(n).map_err(|_| FairmarkError::Overflow("period number"))?,
            ebit,
            operating_taxes: tax,
            ebit_after_taxes: after,
            depreciation_and_amortisation: da,
            capital_expenditure: capex,
            change_in_net_working_capital: dnwc,
            other_adjustments: l.other_adjustments.clone(),
            other_adjustments_total: adjustments,
            free_cash_flow_to_firm: fcff,
            interest_expense: l.interest_expense.filter(|_| financing),
            interest_after_taxes: interest_after,
            net_borrowing: l.net_borrowing.filter(|_| financing),
            free_cash_flow_to_equity: fcfe,
        });
    }
    if let Taxes::Rate(_, treatment) = taxes {
        if !negative.is_empty() {
            ex.convention(format!(
                "negative EBIT in period {}: treatment {} applied",
                negative.join(", "),
                treatment.name()
            ));
        }
    }

    let fcff: Vec<Decimal> = periods.iter().map(|p| p.free_cash_flow_to_firm).collect();
    let fcfe: Option<Vec<Decimal>> = periods
        .iter()
        .map(|p| p.free_cash_flow_to_equity)
        .collect::<Option<Vec<_>>>()
        .filter(|_| financing);
    Ok((
        FreeCashFlowsOutput {
            periods,
            free_cash_flows_to_firm: fcff,
            free_cash_flows_to_equity: fcfe,
            tax_form: match taxes {
                Taxes::Rate(..) => TaxForm::RateOnEbit,
                Taxes::Amounts => TaxForm::Amounts,
            },
            negative_ebit: match taxes {
                Taxes::Rate(_, treatment) => Some(treatment),
                Taxes::Amounts => None,
            },
            interest_tax_rate: shield,
        },
        ex,
    ))
}

/// `dcf.free_cash_flows`: see the methodology page.
pub struct FreeCashFlows;

impl Method for FreeCashFlows {
    const ID: &'static str = "dcf.free_cash_flows";
    const TITLE: &'static str = "Free cash flow build-up (FCFF and FCFE)";
    const DESCRIPTION: &'static str =
        "FCFF per period from EBIT, taxes on EBIT (a rate or amounts), depreciation and \
        amortisation, capital expenditure, change in net working capital and labelled \
        adjustments; FCFE when interest expense and net borrowing are given. Not discounted.";
    const VERSION: &'static str = "1";
    type Input = FreeCashFlowsInput;
    type Output = FreeCashFlowsOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        free_cash_flows(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn lines(ebit: &str) -> FreeCashFlowLines {
        FreeCashFlowLines {
            ebit: d(ebit),
            operating_taxes: None,
            depreciation_and_amortisation: d("10"),
            capital_expenditure: d("12"),
            change_in_net_working_capital: d("3"),
            other_adjustments: Vec::new(),
            interest_expense: None,
            net_borrowing: None,
        }
    }

    fn rate_form(periods: Vec<FreeCashFlowLines>, treatment: NegativeEbit) -> FreeCashFlowsInput {
        FreeCashFlowsInput {
            periods,
            notional_tax_rate: Some(d("0.24")),
            negative_ebit: Some(treatment),
            interest_tax_rate: None,
        }
    }

    #[test]
    fn fcff_formula_with_every_component() {
        let mut l = lines("100");
        l.other_adjustments = vec![
            Adjustment {
                label: "Disposal".into(),
                value: d("7"),
            },
            Adjustment {
                label: "Provision".into(),
                value: d("-2"),
            },
        ];
        let (out, ex) = free_cash_flows(&rate_form(vec![l], NegativeEbit::Credit)).unwrap();
        // 100 − 24 + 10 − 12 − 3 + 5
        assert_eq!(out.free_cash_flows_to_firm, [d("76")]);
        assert_eq!(out.periods[0].other_adjustments_total, d("5"));
        assert!(out.free_cash_flows_to_equity.is_none());
        let fcff = ex
            .steps
            .iter()
            .find(|s| s.label == "FCFF, period 1")
            .unwrap();
        assert_eq!(fcff.formula, "100 − 24.00 + 10 − 12 − (3) + 5");
    }

    /// AC2: with zero interest and zero net borrowing, FCFE equals FCFF, in both tax forms and
    /// under both treatments of a negative EBIT.
    #[test]
    fn fcfe_equals_fcff_without_interest_and_borrowing() {
        let ebits = ["-1234.5", "0", "0.01", "987654.321", "-0.07", "250"];
        for treatment in [NegativeEbit::Credit, NegativeEbit::Zero] {
            for (k, e) in ebits.iter().enumerate() {
                let mut l = lines(e);
                l.change_in_net_working_capital = Decimal::from(k as i64 - 3);
                l.interest_expense = Some(Decimal::ZERO);
                l.net_borrowing = Some(Decimal::ZERO);
                let mut amounts = l.clone();
                amounts.operating_taxes = Some(d("17.3"));
                let inputs = [
                    rate_form(vec![l], treatment),
                    FreeCashFlowsInput {
                        periods: vec![amounts],
                        notional_tax_rate: None,
                        negative_ebit: None,
                        interest_tax_rate: Some(d("0.279")),
                    },
                ];
                for input in inputs {
                    let (out, _) = free_cash_flows(&input).unwrap();
                    assert_eq!(
                        out.free_cash_flows_to_equity.as_deref(),
                        Some(out.free_cash_flows_to_firm.as_slice())
                    );
                }
            }
        }
    }

    #[test]
    fn negative_ebit_treatments() {
        let (credit, ex) =
            free_cash_flows(&rate_form(vec![lines("-100")], NegativeEbit::Credit)).unwrap();
        assert_eq!(credit.periods[0].operating_taxes, d("-24.00"));
        assert!(ex
            .conventions
            .iter()
            .any(|c| c == "negative EBIT in period 1: treatment credit applied"));
        let (zero, _) =
            free_cash_flows(&rate_form(vec![lines("-100")], NegativeEbit::Zero)).unwrap();
        assert_eq!(zero.periods[0].operating_taxes, Decimal::ZERO);
        assert_eq!(
            credit.free_cash_flows_to_firm[0] - zero.free_cash_flows_to_firm[0],
            d("24")
        );
        // A positive EBIT is taxed alike under both treatments.
        let a = free_cash_flows(&rate_form(vec![lines("100")], NegativeEbit::Credit)).unwrap();
        let b = free_cash_flows(&rate_form(vec![lines("100")], NegativeEbit::Zero)).unwrap();
        assert_eq!(a.0.free_cash_flows_to_firm, b.0.free_cash_flows_to_firm);
    }

    /// AC3 and AC14: each negative D&A, capex and interest is `FM-E001` with the JSON pointer,
    /// the period and the fix.
    #[test]
    fn negative_amounts_name_the_pointer_and_the_period() {
        for (field, pointer) in [
            (
                "depreciation_and_amortisation",
                "/periods/1/depreciation_and_amortisation",
            ),
            ("capital_expenditure", "/periods/1/capital_expenditure"),
            ("interest_expense", "/periods/1/interest_expense"),
        ] {
            let mut bad = lines("100");
            bad.interest_expense = Some(d("1"));
            bad.net_borrowing = Some(d("0"));
            match field {
                "depreciation_and_amortisation" => bad.depreciation_and_amortisation = d("-1"),
                "capital_expenditure" => bad.capital_expenditure = d("-1"),
                _ => bad.interest_expense = Some(d("-1")),
            }
            let mut good = lines("100");
            good.interest_expense = Some(d("1"));
            good.net_borrowing = Some(d("0"));
            let err =
                free_cash_flows(&rate_form(vec![good, bad], NegativeEbit::Credit)).unwrap_err();
            assert_eq!(err.code(), "FM-E001");
            let msg = err.to_string();
            assert!(msg.contains(pointer), "{msg}");
            assert!(msg.contains("period 2"), "{msg}");
            assert!(msg.contains("other_adjustments"), "{msg}");
        }
    }

    /// AC14: every refusal of the build-up names a JSON pointer.
    #[test]
    fn every_error_message_contains_a_json_pointer() {
        let base = || rate_form(vec![lines("100")], NegativeEbit::Credit);
        let mut cases: Vec<FreeCashFlowsInput> = Vec::new();
        let mut i = base();
        i.periods.clear();
        cases.push(i);
        let mut i = base();
        i.periods[0].operating_taxes = Some(d("1"));
        cases.push(i);
        let mut i = base();
        i.notional_tax_rate = None;
        cases.push(i);
        let mut i = base();
        i.notional_tax_rate = None;
        i.negative_ebit = None;
        i.periods.push(lines("1"));
        i.periods[0].operating_taxes = Some(d("1"));
        cases.push(i);
        let mut i = base();
        i.negative_ebit = None;
        cases.push(i);
        let mut i = base();
        i.notional_tax_rate = Some(d("1"));
        cases.push(i);
        let mut i = base();
        i.interest_tax_rate = Some(d("0.2"));
        cases.push(i);
        let mut i = base();
        i.periods[0].interest_expense = Some(d("1"));
        cases.push(i);
        let mut i = base();
        i.periods[0].net_borrowing = Some(d("1"));
        cases.push(i);
        let mut i = base();
        i.periods[0].interest_expense = Some(d("1"));
        i.periods[0].net_borrowing = Some(d("1"));
        i.periods.push(lines("1"));
        cases.push(i);
        let mut i = base();
        i.periods[0].other_adjustments = vec![Adjustment {
            label: String::new(),
            value: d("1"),
        }];
        cases.push(i);
        let amounts = || {
            let mut l = lines("100");
            l.operating_taxes = Some(d("24"));
            FreeCashFlowsInput {
                periods: vec![l],
                notional_tax_rate: None,
                negative_ebit: None,
                interest_tax_rate: None,
            }
        };
        let mut i = amounts();
        i.negative_ebit = Some(NegativeEbit::Zero);
        cases.push(i);
        let mut i = amounts();
        i.interest_tax_rate = Some(d("0.2"));
        cases.push(i);
        let mut i = amounts();
        i.periods[0].interest_expense = Some(d("1"));
        i.periods[0].net_borrowing = Some(d("1"));
        cases.push(i);
        let mut i = amounts();
        i.periods[0].interest_expense = Some(d("1"));
        i.periods[0].net_borrowing = Some(d("1"));
        i.interest_tax_rate = Some(d("-0.1"));
        cases.push(i);
        for (k, input) in cases.iter().enumerate() {
            let err = free_cash_flows(input).unwrap_err();
            assert_eq!(err.code(), "FM-E001", "case {k}");
            assert!(err.to_string().contains("`/"), "case {k}: {err}");
        }
    }

    /// R28: every convention of the build-up is reported.
    #[test]
    fn conventions_are_reported() {
        let mut l = lines("-5");
        l.interest_expense = Some(d("2"));
        l.net_borrowing = Some(d("1"));
        let (_, ex) = free_cash_flows(&rate_form(vec![l], NegativeEbit::Zero)).unwrap();
        for text in [
            "tax form: rate on EBIT",
            "negative EBIT: zero",
            "no tax loss carry-forward",
            "interest tax shield at the tax rate on EBIT 0.24",
            "change in net working capital is positive for an increase",
            "adjustments are positive for an inflow",
            "net borrowing is positive for new borrowing",
            "not discounted",
            "negative EBIT in period 1: treatment zero applied",
        ] {
            assert!(
                ex.conventions.iter().any(|c| c.contains(text)),
                "{text}: {:?}",
                ex.conventions
            );
        }
    }

    #[test]
    fn extreme_amounts_overflow_as_a_typed_error() {
        let mut l = lines("1");
        l.depreciation_and_amortisation = Decimal::MAX;
        l.change_in_net_working_capital = Decimal::MIN;
        let err = free_cash_flows(&rate_form(vec![l], NegativeEbit::Credit)).unwrap_err();
        assert_eq!(err.code(), "FM-E008");
    }
}
