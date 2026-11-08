//! `dcf.free_cash_flows`: free cash flow build-up from reclassified line items (SPEC-0001 R2 to
//! R5).
//!
//! FCFF_t = EBIT_t − taxes on EBIT_t + D&A_t − capex_t − ΔNWC_t + Σ adjustments_t and, with the
//! financing lines, FCFE_t = FCFF_t − interest_t + interest tax shield_t + net borrowing_t, the
//! shield being t × interest_t, limited to t × min(interest_t, max(EBIT_t, 0)) under the `zero`
//! treatment (SPEC-0001 Decision 22). Amounts are in currency units per period and are not
//! discounted.
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
    /// (no tax; with the financing lines the interest tax shield is limited to the taxable
    /// income available, rate × min(interest, max(EBIT, 0)), so the taxes never fall below
    /// zero). Required with `notional_tax_rate`, there is no default; not accepted with
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
    /// Rate t of the interest tax shield applied in this period: 0 where the `zero` treatment
    /// of a negative EBIT applies (SPEC-0001 amendment 14); `null` without financing lines.
    pub interest_tax_rate: Option<Decimal>,
    /// Interest tax shield of the period: t × interest expense; under the `zero` treatment
    /// t × min(interest expense, max(EBIT, 0)), the deduction limited to the taxable income
    /// available (SPEC-0001 Decision 22); `null` without financing lines.
    pub interest_tax_shield: Option<Decimal>,
    /// Interest expense − interest tax shield; `null` without financing lines.
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
    /// Rate t of the interest tax shield of the tax form: the tax rate on EBIT, or the given
    /// `interest_tax_rate`; `null` without financing lines. The rate applied in each period is
    /// `periods[].interest_tax_rate`, 0 where the `zero` treatment applies.
    pub interest_tax_rate: Option<Decimal>,
}

/// The tax choice of the build-up (Decision 7, R3, R4): the three tax fields of
/// [`FreeCashFlowsInput`], which the shared DCF engine carries in its own input.
#[derive(Debug, Clone, Copy, Default)]
pub struct TaxChoice {
    /// Tax rate on EBIT (rate form), or `None` when the taxes are given as amounts.
    pub notional_tax_rate: Option<Decimal>,
    /// Treatment of a negative EBIT; required with the rate form only.
    pub negative_ebit: Option<NegativeEbit>,
    /// Rate of the interest tax shield; required with the amounts form and financing lines.
    pub interest_tax_rate: Option<Decimal>,
}

/// Where the lines sit in the caller's input, for error messages and explain labels: the JSON
/// pointer of the list of lines and the period number of its first element.
#[derive(Debug, Clone, Copy)]
pub struct Location<'a> {
    /// JSON pointer of the list of lines, for example `/periods` or `/stages/0/lines`.
    pub pointer_prefix: &'a str,
    /// Period number of the first line, from 1.
    pub first_period: u32,
}

impl Location<'_> {
    fn pointer(&self, index: usize, field: &str) -> String {
        format!("{}/{index}/{field}", self.pointer_prefix)
    }

    fn period(&self, index: usize) -> Result<u32> {
        u32::try_from(index)
            .ok()
            .and_then(|i| self.first_period.checked_add(i))
            .ok_or(FairmarkError::Overflow("period number"))
    }

    /// "`<pointer>` (period n)".
    fn field(&self, index: usize, field: &str) -> Result<String> {
        Ok(format!(
            "`{}` (period {})",
            self.pointer(index, field),
            self.period(index)?
        ))
    }
}

fn invalid(msg: impl Into<String>) -> FairmarkError {
    FairmarkError::InvalidInput(msg.into())
}

/// A value for an explain formula: negative values in parentheses, so that `a − (−3)` reads
/// unambiguously.
fn term(v: Decimal) -> String {
    if v < Decimal::ZERO {
        format!("({v})")
    } else {
        v.to_string()
    }
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
fn check_not_negative(value: Decimal, at: &Location, index: usize, field: &str) -> Result<()> {
    if value < Decimal::ZERO {
        return Err(invalid(format!(
            "{} must be zero or more (got {value}); enter disposals and other inflows in `{}` \
             with a positive sign",
            at.field(index, field)?,
            at.pointer(index, "other_adjustments")
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
fn validate(
    p: &[FreeCashFlowLines],
    choice: &TaxChoice,
    at: &Location,
) -> Result<(Taxes, bool, Option<Decimal>)> {
    if p.is_empty() {
        return Err(invalid(format!(
            "`{}` must contain at least one period with its line items",
            at.pointer_prefix
        )));
    }
    let with_amounts: Vec<usize> = (0..p.len())
        .filter(|&i| p[i].operating_taxes.is_some())
        .collect();
    let taxes = match choice.notional_tax_rate {
        Some(t) => {
            if let Some(&i) = with_amounts.first() {
                return Err(invalid(format!(
                    "give the taxes on EBIT in one form only: `/notional_tax_rate` conflicts with \
                     {}; remove `notional_tax_rate`, or remove `operating_taxes` from every line \
                     of `{}`",
                    at.field(i, "operating_taxes")?,
                    at.pointer_prefix
                )));
            }
            check_tax_rate(t, "/notional_tax_rate")?;
            let Some(treatment) = choice.negative_ebit else {
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
                return Err(invalid(format!(
                    "no taxes on EBIT: give `/notional_tax_rate` (with `/negative_ebit`), or \
                     `operating_taxes` in every line of `{}`",
                    at.pointer_prefix
                )));
            }
            if let Some(i) = (0..p.len()).find(|&i| p[i].operating_taxes.is_none()) {
                return Err(invalid(format!(
                    "{} is missing: give the taxes on EBIT as amounts in every line of `{}`, or \
                     remove them and give `/notional_tax_rate`",
                    at.field(i, "operating_taxes")?,
                    at.pointer_prefix
                )));
            }
            if choice.negative_ebit.is_some() {
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
            at,
            i,
            "depreciation_and_amortisation",
        )?;
        check_not_negative(lines.capital_expenditure, at, i, "capital_expenditure")?;
        if let Some(interest) = lines.interest_expense {
            check_not_negative(interest, at, i, "interest_expense")?;
        }
        for (k, a) in lines.other_adjustments.iter().enumerate() {
            if a.label.trim().is_empty() {
                return Err(invalid(format!(
                    "{} is empty: say what the amount is",
                    at.field(i, &format!("other_adjustments/{k}/label"))?
                )));
            }
        }
        match (lines.interest_expense, lines.net_borrowing) {
            (Some(_), None) => {
                return Err(invalid(format!(
                    "{} is missing: the FCFE needs `interest_expense` and `net_borrowing` \
                     together; give it (0 for none) or remove `{}`",
                    at.field(i, "net_borrowing")?,
                    at.pointer(i, "interest_expense")
                )))
            }
            (None, Some(_)) => {
                return Err(invalid(format!(
                    "{} is missing: the FCFE needs `interest_expense` and `net_borrowing` \
                     together; give it (0 for none) or remove `{}`",
                    at.field(i, "interest_expense")?,
                    at.pointer(i, "net_borrowing")
                )))
            }
            _ => {}
        }
    }
    let financing = p[0].interest_expense.is_some();
    if let Some(i) = (0..p.len()).find(|&i| p[i].interest_expense.is_some() != financing) {
        let (given, missing) = if financing { (0, i) } else { (i, 0) };
        return Err(invalid(format!(
            "financing lines are given for period {} but not for period {}: add `{}` and `{}`, \
             or remove them from `{}` and `{}`, so that every period or none has them",
            at.period(given)?,
            at.period(missing)?,
            at.pointer(missing, "interest_expense"),
            at.pointer(missing, "net_borrowing"),
            at.pointer(given, "interest_expense"),
            at.pointer(given, "net_borrowing"),
        )));
    }

    let shield =
        match (taxes, financing, choice.interest_tax_rate) {
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
                return Err(invalid(format!(
                    "`/interest_tax_rate` is used only for the FCFE: give `interest_expense` and \
                     `net_borrowing` in every line of `{}`, or remove `interest_tax_rate`",
                    at.pointer_prefix
                )))
            }
            (_, false, None) => None,
        };
    Ok((taxes, financing, shield))
}

pub(crate) const FORMULA_FCFF: &str =
    "FCFF_t = EBIT_t − taxes on EBIT_t + D&A_t − capex_t − ΔNWC_t + Σ adjustments_t";
const FORMULA_FCFE: &str =
    "FCFE_t = FCFF_t − (interest_t − interest tax shield_t) + net borrowing_t";

/// Builds FCFF, and FCFE when the financing lines are given, for every period.
pub fn free_cash_flows(input: &FreeCashFlowsInput) -> Result<(FreeCashFlowsOutput, Explain)> {
    let mut ex = Explain::new(FreeCashFlows::ID, FORMULA_FCFF);
    let choice = TaxChoice {
        notional_tax_rate: input.notional_tax_rate,
        negative_ebit: input.negative_ebit,
        interest_tax_rate: input.interest_tax_rate,
    };
    let at = Location {
        pointer_prefix: "/periods",
        first_period: 1,
    };
    let out = build_up(&input.periods, &choice, at, &mut ex)?;
    // Reported by the standalone method only: inside a DCF the flows are discounted, and the
    // basis decides which flow is used.
    if out.free_cash_flows_to_equity.is_some() {
        ex.formula = format!("{FORMULA_FCFF};  {FORMULA_FCFE}");
    } else {
        ex.convention("FCFE: not computed (no interest expense and net borrowing given)");
    }
    ex.convention("amounts in currency units per period, not discounted");
    Ok((out, ex))
}

/// The build-up itself, for any list of lines: validates them, appends the steps and
/// conventions to `ex` and returns the flows. Error pointers and period numbers come from `at`,
/// so that the shared DCF engine reuses it for the lines of a stage (ADR 0009).
pub fn build_up(
    lines: &[FreeCashFlowLines],
    choice: &TaxChoice,
    at: Location,
    ex: &mut Explain,
) -> Result<FreeCashFlowsOutput> {
    let (taxes, financing, shield) = validate(lines, choice, &at)?;
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
            ex.convention(
                "operating taxes are positive for a tax paid and are subtracted; negative for a \
                 tax credit",
            );
        }
    }
    ex.convention("no tax loss carry-forward is modelled");
    ex.convention(
        "change in net working capital is positive for an increase, which absorbs cash and is \
         subtracted",
    );
    ex.convention("adjustments are positive for an inflow and are added");
    match (taxes, shield) {
        (Taxes::Rate(_, NegativeEbit::Credit), Some(t)) => {
            ex.convention(format!(
                "interest tax shield at the tax rate on EBIT {t}: interest after taxes = \
                 interest × (1 − {t}), whatever the sign of EBIT"
            ));
        }
        (Taxes::Rate(_, NegativeEbit::Zero), Some(t)) => {
            ex.convention(format!(
                "interest tax shield at the tax rate on EBIT {t}, limited to the taxable income \
                 available (treatment zero): shield = {t} × min(interest, max(EBIT, 0)), so the \
                 taxes never fall below zero; zero in a period with a negative EBIT, where the \
                 interest is subtracted in full, as in the net-income route"
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
    }

    let mut periods = Vec::with_capacity(lines.len());
    let mut negative = Vec::new();
    let mut no_shield_periods: Vec<String> = Vec::new();
    let mut limited_shields: Vec<String> = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        let n = at.period(i)?;
        let ebit = l.ebit;
        let tax = match (taxes, l.operating_taxes) {
            (Taxes::Rate(t, treatment), _) => {
                if ebit < Decimal::ZERO {
                    negative.push(n.to_string());
                }
                if ebit < Decimal::ZERO && treatment == NegativeEbit::Zero {
                    ex.step(
                        format!("Taxes on EBIT, period {n}"),
                        format!("0 (EBIT {} negative, treatment zero)", term(ebit)),
                        Decimal::ZERO,
                    );
                    Decimal::ZERO
                } else {
                    let tax = mul(t, ebit)?;
                    ex.step(
                        format!("Taxes on EBIT, period {n}"),
                        format!("{t} × {}", term(ebit)),
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
                    format!("{} (given)", term(tax)),
                    tax,
                );
                tax
            }
        };
        let after = sub(ebit, tax)?;
        ex.step(
            format!("EBIT after taxes, period {n}"),
            format!("{} − {}", term(ebit), term(tax)),
            after,
        );
        let mut adjustments = Decimal::ZERO;
        for a in &l.other_adjustments {
            ex.step(
                format!("Adjustment, period {n}: {}", a.label),
                format!("{} (given)", term(a.value)),
                a.value,
            );
            adjustments = add(adjustments, a.value)?;
        }
        if l.other_adjustments.len() > 1 {
            let terms: Vec<String> = l.other_adjustments.iter().map(|a| term(a.value)).collect();
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
            format!(
                "{} − {} + {} − {} − {} + {}",
                term(ebit),
                term(tax),
                term(da),
                term(capex),
                term(dnwc),
                term(adjustments)
            ),
            fcff,
        );
        let (period_shield, tax_shield, interest_after, fcfe) =
            match (l.interest_expense, l.net_borrowing, shield) {
                (Some(interest), Some(borrowing), Some(shield_rate)) => {
                    let zero = matches!(taxes, Taxes::Rate(_, NegativeEbit::Zero));
                    let (t, tax_shield) = if zero && ebit < Decimal::ZERO {
                        // Amendment 14: no shield where the `zero` treatment applies.
                        no_shield_periods.push(n.to_string());
                        ex.step(
                            format!("Interest tax shield, period {n}"),
                            format!(
                                "0 (EBIT {} negative, treatment zero: no tax shield)",
                                term(ebit)
                            ),
                            Decimal::ZERO,
                        );
                        (Decimal::ZERO, Decimal::ZERO)
                    } else if zero {
                        // Decision 22: the deduction is limited to the taxable income available.
                        let deductible = interest.min(ebit);
                        let tax_shield = mul(shield_rate, deductible)?;
                        ex.step(
                            format!("Interest tax shield, period {n}"),
                            format!(
                                "{shield_rate} × min({}, max({}, 0)) (treatment zero: shield \
                                 limited to the taxable income available)",
                                term(interest),
                                term(ebit)
                            ),
                            tax_shield,
                        );
                        if ebit < interest {
                            limited_shields.push(format!(
                                "interest tax shield limited to the taxable income in period {n} \
                                 (EBIT {} below interest {}, treatment zero): shield = \
                                 {shield_rate} × {}",
                                term(ebit),
                                term(interest),
                                term(ebit)
                            ));
                        }
                        (shield_rate, tax_shield)
                    } else {
                        let tax_shield = mul(shield_rate, interest)?;
                        ex.step(
                            format!("Interest tax shield, period {n}"),
                            format!("{shield_rate} × {}", term(interest)),
                            tax_shield,
                        );
                        (shield_rate, tax_shield)
                    };
                    let after_tax = sub(interest, tax_shield)?;
                    ex.step(
                        format!("Interest after taxes, period {n}"),
                        format!("{} − {}", term(interest), term(tax_shield)),
                        after_tax,
                    );
                    let fcfe = add(sub(fcff, after_tax)?, borrowing)?;
                    ex.step(
                        format!("FCFE, period {n}"),
                        format!("{} − {} + {}", term(fcff), term(after_tax), term(borrowing)),
                        fcfe,
                    );
                    (Some(t), Some(tax_shield), Some(after_tax), Some(fcfe))
                }
                _ => (None, None, None, None),
            };
        periods.push(FreeCashFlowPeriod {
            period: n,
            ebit,
            operating_taxes: tax,
            ebit_after_taxes: after,
            depreciation_and_amortisation: da,
            capital_expenditure: capex,
            change_in_net_working_capital: dnwc,
            other_adjustments: l.other_adjustments.clone(),
            other_adjustments_total: adjustments,
            free_cash_flow_to_firm: fcff,
            // `validate` has checked that every period or none carries the financing lines.
            interest_expense: l.interest_expense,
            interest_tax_rate: period_shield,
            interest_tax_shield: tax_shield,
            interest_after_taxes: interest_after,
            net_borrowing: l.net_borrowing,
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

    if !no_shield_periods.is_empty() {
        ex.convention(format!(
            "no interest tax shield in period {} (negative EBIT, treatment zero)",
            no_shield_periods.join(", ")
        ));
    }
    for limited in limited_shields {
        ex.convention(limited);
    }

    let fcff: Vec<Decimal> = periods.iter().map(|p| p.free_cash_flow_to_firm).collect();
    // `None` exactly when the financing lines are not given (every period or none has them).
    let fcfe: Option<Vec<Decimal>> = periods.iter().map(|p| p.free_cash_flow_to_equity).collect();
    Ok(FreeCashFlowsOutput {
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
    })
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
    // Version 2: the interest tax shield of the `zero` treatment is limited to the taxable income
    // available (SPEC-0001 Decision 22).
    const VERSION: &'static str = "2";
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
        assert_eq!(fcff.formula, "100 − 24.00 + 10 − 12 − 3 + 5");
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

    /// Amendment 14: under `zero`, a period with a negative EBIT has no interest tax shield and
    /// the FCFE equals the net-income route; `credit` keeps the shield.
    #[test]
    fn zero_treatment_removes_the_interest_tax_shield() {
        let mut l = lines("-120");
        l.interest_expense = Some(d("50"));
        l.net_borrowing = Some(d("20"));
        let (zero, ex) = free_cash_flows(&rate_form(vec![l.clone()], NegativeEbit::Zero)).unwrap();
        let p = &zero.periods[0];
        assert_eq!(p.interest_tax_rate, Some(Decimal::ZERO));
        assert_eq!(p.interest_after_taxes, Some(d("50")));
        // Net income −170 + 10 − 12 − 3 + 20.
        assert_eq!(p.free_cash_flow_to_equity, Some(d("-155")));
        assert!(ex
            .conventions
            .iter()
            .any(|c| c == "no interest tax shield in period 1 (negative EBIT, treatment zero)"));
        let (credit, _) = free_cash_flows(&rate_form(vec![l], NegativeEbit::Credit)).unwrap();
        assert_eq!(credit.periods[0].interest_tax_rate, Some(d("0.24")));
        assert_eq!(credit.periods[0].interest_tax_shield, Some(d("12.00")));
        assert_eq!(credit.periods[0].interest_after_taxes, Some(d("38.00")));
    }

    fn financed(ebit: &str, interest: &str) -> FreeCashFlowLines {
        let mut l = lines(ebit);
        l.interest_expense = Some(d(interest));
        l.net_borrowing = Some(d("0"));
        l
    }

    /// Decision 22: under `zero` the shield is t × min(interest, max(EBIT, 0)), so the total
    /// taxes are never below zero; `credit` keeps the full shield.
    #[test]
    fn zero_treatment_limits_the_shield_to_ebit() {
        let (out, ex) =
            free_cash_flows(&rate_form(vec![financed("30", "50")], NegativeEbit::Zero)).unwrap();
        let p = &out.periods[0];
        assert_eq!(p.interest_tax_rate, Some(d("0.24")));
        assert_eq!(p.interest_tax_shield, Some(d("7.20")));
        assert_eq!(p.interest_after_taxes, Some(d("42.80")));
        // 30 − 7.20 + 10 − 12 − 3 − 42.80; net income −20 + 10 − 12 − 3 = −25.
        assert_eq!(p.free_cash_flow_to_equity, Some(d("-25")));
        assert_eq!(
            p.operating_taxes - p.interest_tax_shield.unwrap(),
            Decimal::ZERO
        );
        let formula = |label: &str| {
            ex.steps
                .iter()
                .find(|s| s.label == label)
                .unwrap_or_else(|| panic!("{label}: {:?}", ex.steps))
                .formula
                .clone()
        };
        assert_eq!(
            formula("Interest tax shield, period 1"),
            "0.24 × min(50, max(30, 0)) (treatment zero: shield limited to the taxable income \
             available)"
        );
        assert_eq!(formula("Interest after taxes, period 1"), "50 − 7.20");
        assert!(
            ex.conventions.iter().any(|c| c
                == "interest tax shield limited to the taxable income in period 1 (EBIT 30 \
                    below interest 50, treatment zero): shield = 0.24 × 30"),
            "{:?}",
            ex.conventions
        );
        assert!(
            ex.conventions
                .iter()
                .any(|c| c.contains("0.24 × min(interest, max(EBIT, 0))")),
            "{:?}",
            ex.conventions
        );

        // `credit`: the full shield also when EBIT is below the interest.
        let (credit, ex) =
            free_cash_flows(&rate_form(vec![financed("30", "50")], NegativeEbit::Credit)).unwrap();
        assert_eq!(credit.periods[0].interest_tax_shield, Some(d("12.00")));
        assert_eq!(credit.periods[0].interest_after_taxes, Some(d("38.00")));
        assert!(
            !ex.conventions.iter().any(|c| c.contains("limited")),
            "{:?}",
            ex.conventions
        );

        // EBIT at or above the interest: the full shield under both treatments.
        for treatment in [NegativeEbit::Credit, NegativeEbit::Zero] {
            for ebit in ["50", "80"] {
                let (out, ex) =
                    free_cash_flows(&rate_form(vec![financed(ebit, "50")], treatment)).unwrap();
                assert_eq!(out.periods[0].interest_tax_shield, Some(d("12.00")));
                assert_eq!(out.periods[0].interest_after_taxes, Some(d("38.00")));
                assert!(!ex
                    .conventions
                    .iter()
                    .any(|c| c.contains("limited to the taxable income in")));
            }
        }

        // EBIT zero: no taxable income, no shield; the rate stays t (EBIT is not negative).
        let (out, _) =
            free_cash_flows(&rate_form(vec![financed("0", "50")], NegativeEbit::Zero)).unwrap();
        assert_eq!(out.periods[0].interest_tax_rate, Some(d("0.24")));
        assert_eq!(out.periods[0].interest_tax_shield, Some(Decimal::ZERO));
        assert_eq!(out.periods[0].interest_after_taxes, Some(d("50")));

        // Interest zero: no shield and no limitation reported.
        let (out, ex) =
            free_cash_flows(&rate_form(vec![financed("30", "0")], NegativeEbit::Zero)).unwrap();
        assert_eq!(out.periods[0].interest_tax_shield, Some(Decimal::ZERO));
        assert_eq!(out.periods[0].interest_after_taxes, Some(Decimal::ZERO));
        assert!(!ex
            .conventions
            .iter()
            .any(|c| c.contains("limited to the taxable income in")));
    }

    /// Decision 22: the total taxes are never below zero under `zero`, for any EBIT and
    /// interest.
    #[test]
    fn zero_treatment_total_taxes_never_below_zero() {
        for ebit in [
            "-80", "-0.01", "0", "0.01", "29.99", "30", "49.99", "50", "50.01", "900",
        ] {
            for interest in ["0", "0.01", "30", "50", "1000"] {
                let (out, _) = free_cash_flows(&rate_form(
                    vec![financed(ebit, interest)],
                    NegativeEbit::Zero,
                ))
                .unwrap();
                let p = &out.periods[0];
                let shield = p.interest_tax_shield.unwrap();
                assert!(
                    p.operating_taxes - shield >= Decimal::ZERO,
                    "{ebit} {interest}"
                );
                assert_eq!(
                    p.interest_after_taxes.unwrap() + shield,
                    d(interest),
                    "{ebit} {interest}"
                );
            }
        }
    }

    /// Negative values appear in parentheses in every formula.
    #[test]
    fn negative_values_are_in_parentheses() {
        let mut l = lines("-100");
        l.change_in_net_working_capital = d("-3");
        l.interest_expense = Some(d("2"));
        l.net_borrowing = Some(d("-1"));
        let (_, ex) = free_cash_flows(&rate_form(vec![l], NegativeEbit::Credit)).unwrap();
        let formula = |label: &str| {
            ex.steps
                .iter()
                .find(|s| s.label == label)
                .unwrap()
                .formula
                .clone()
        };
        assert_eq!(formula("Taxes on EBIT, period 1"), "0.24 × (-100)");
        assert_eq!(
            formula("FCFF, period 1"),
            "(-100) − (-24.00) + 10 − 12 − (-3) + 0"
        );
        assert_eq!(formula("FCFE, period 1"), "(-75.00) − 1.52 + (-1)");
    }

    /// The shared engine reuses the build-up for the lines of a stage: pointers and period
    /// numbers come from the location.
    #[test]
    fn build_up_reports_the_location_of_the_lines() {
        let choice = TaxChoice {
            notional_tax_rate: Some(d("0.24")),
            negative_ebit: Some(NegativeEbit::Credit),
            interest_tax_rate: None,
        };
        let at = Location {
            pointer_prefix: "/stages/1/lines",
            first_period: 6,
        };
        let mut ex = Explain::new("dcf.fcff", "");
        let out = build_up(&[lines("100"), lines("-5")], &choice, at, &mut ex).unwrap();
        assert_eq!(out.periods[1].period, 7);
        assert!(ex.steps.iter().any(|s| s.label == "FCFF, period 7"));
        assert!(ex
            .conventions
            .iter()
            .any(|c| c == "negative EBIT in period 7: treatment credit applied"));
        let mut bad = lines("1");
        bad.capital_expenditure = d("-1");
        let msg = build_up(&[lines("1"), bad], &choice, at, &mut ex)
            .unwrap_err()
            .to_string();
        assert!(
            msg.contains("`/stages/1/lines/1/capital_expenditure` (period 7)"),
            "{msg}"
        );
        assert!(
            msg.contains("`/stages/1/lines/1/other_adjustments`"),
            "{msg}"
        );
    }

    #[test]
    fn financing_mismatch_names_both_fields() {
        let mut first = lines("1");
        first.interest_expense = Some(d("1"));
        first.net_borrowing = Some(d("0"));
        let msg = free_cash_flows(&rate_form(vec![first, lines("2")], NegativeEbit::Credit))
            .unwrap_err()
            .to_string();
        assert!(msg.contains("`/periods/1/interest_expense`"), "{msg}");
        assert!(msg.contains("`/periods/1/net_borrowing`"), "{msg}");
    }

    /// Audit: the sign of the given tax amounts is reported.
    #[test]
    fn amounts_form_reports_the_sign_of_operating_taxes() {
        let mut l = lines("100");
        l.operating_taxes = Some(d("-4"));
        let (out, ex) = free_cash_flows(&FreeCashFlowsInput {
            periods: vec![l],
            notional_tax_rate: None,
            negative_ebit: None,
            interest_tax_rate: None,
        })
        .unwrap();
        assert_eq!(out.periods[0].ebit_after_taxes, d("104"));
        assert!(ex.conventions.iter().any(|c| c
            == "operating taxes are positive for a tax paid and are subtracted; negative for a \
                tax credit"));
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
