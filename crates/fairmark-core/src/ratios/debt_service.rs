//! `ratios.debt_service`: can the company service its debt? (SPEC-0007)

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    both_positive, over_positive, require_definition, term, BalanceBasis, Balances,
    NetFinancialPosition, Ratio, RatioValue, NFP_CONVENTION,
};
use crate::error::{rate_field, FairmarkError, Result};
use crate::explain::Explain;
use crate::method::Method;
use crate::num;
use crate::tvm::discount_factor;

/// Flows of the period, in currency units.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DebtServiceIncome {
    /// EBITDA of the period, in currency units, as defined in `ebitda_definition`.
    pub ebitda: Decimal,
    /// EBIT (operating result) of the period, in currency units.
    pub ebit: Decimal,
    /// Interest expense of the period, in currency units, as defined in `interest_definition`
    /// (gross, or net of interest income).
    pub interest: Decimal,
}

/// Balance-sheet amounts at one date.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DebtServiceBalances {
    /// Net financial position with its composition, in currency units.
    pub net_financial_position: NetFinancialPosition,
}

/// One period of the loan schedule.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DebtPeriod {
    /// Cash flow available for debt service in the period, in currency units.
    pub cfads: Decimal,
    /// Debt service (principal and interest) due in the period, in currency units.
    pub debt_service: Decimal,
}

/// The loan schedule from the calculation date to the final maturity.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DebtSchedule {
    /// Discount rate for the LLCR: the cost of debt per period of the schedule chosen by the
    /// analyst, as a decimal fraction (0.06 = 6% per period).
    pub discount_rate: Decimal,
    /// Debt outstanding at the start of the first period, in currency units.
    pub debt_outstanding: Decimal,
    /// Balance of the debt service reserve account at the valuation date, in currency units,
    /// added undiscounted to the LLCR numerator. Absent: no reserve account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reserve_balance: Option<Decimal>,
    /// Periods 1..n, in order, up to the final maturity of the loan. At least one.
    pub periods: Vec<DebtPeriod>,
}

/// Input of `ratios.debt_service`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DebtServiceInput {
    /// Balances used by the ratios: `closing`, or `average` of opening and closing.
    pub balances: Balances,
    /// How EBITDA was measured, in words (for example "reported EBITDA, no adjustments");
    /// echoed in the conventions.
    pub ebitda_definition: String,
    /// How interest was measured, in words (for example "gross interest expense" or "interest
    /// expense net of interest income"); echoed in the conventions.
    pub interest_definition: String,
    /// Flows of the period.
    pub income_statement: DebtServiceIncome,
    /// Balance sheet at the end of the period.
    pub closing: DebtServiceBalances,
    /// Balance sheet at the start of the period. Required with average balances; not used
    /// with closing balances.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opening: Option<DebtServiceBalances>,
    /// Loan schedule for DSCR and LLCR. Absent: DSCR and LLCR are not defined and the other
    /// ratios are computed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub debt_schedule: Option<DebtSchedule>,
}

/// Output of `ratios.debt_service`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct DebtServiceOutput {
    /// Net financial position used by the ratios, in currency units (positive for net debt).
    pub net_financial_position: Decimal,
    /// EBIT / interest, multiple.
    pub interest_cover_ebit: RatioValue,
    /// EBITDA / interest, multiple.
    pub interest_cover_ebitda: RatioValue,
    /// Net financial position / EBITDA, multiple.
    pub nfp_to_ebitda: RatioValue,
    /// CFADS / debt service of each period, multiple, in the order of the schedule; empty when
    /// no debt schedule is given.
    pub dscr: Vec<RatioValue>,
    /// Present value of the CFADS of every period at the calculation date, in currency units;
    /// null when no debt schedule is given.
    pub pv_cfads: Option<Decimal>,
    /// (Present value of CFADS + reserve balance) / debt outstanding, multiple.
    pub llcr: RatioValue,
}

pub(crate) const NO_SCHEDULE: &str = "no debt schedule given";

pub fn debt_service(input: &DebtServiceInput) -> Result<(DebtServiceOutput, Explain)> {
    if input
        .debt_schedule
        .as_ref()
        .is_some_and(|s| s.periods.is_empty())
    {
        return Err(FairmarkError::EmptyCashFlows);
    }
    let mut ex = Explain::new(
        DebtService::ID,
        "interest cover = EBIT (or EBITDA) / interest; NFP / EBITDA; DSCR_t = CFADS_t / debt \
         service_t; LLCR = (Σ CFADS_t / (1 + r)^t + reserve balance) / debt outstanding",
    );
    require_definition("ebitda_definition", &input.ebitda_definition)?;
    require_definition("interest_definition", &input.interest_definition)?;
    let basis = BalanceBasis::new(input.balances, input.opening.is_some(), &mut ex)?;
    ex.convention(format!("EBITDA as given: {}", input.ebitda_definition));
    ex.convention(format!("interest as given: {}", input.interest_definition));
    ex.convention(NFP_CONVENTION);
    ex.convention("debt service as given");
    ex.convention("a ratio with a zero denominator is not defined");
    ex.convention(
        "not meaningful, so not defined: an interest cover when interest or its numerator (EBIT \
         or EBITDA) is zero or negative; NFP / EBITDA when EBITDA is zero or negative; DSCR \
         when debt service is zero or negative; LLCR when debt outstanding is zero or negative",
    );

    let closing_nfp = input
        .closing
        .net_financial_position
        .total(&mut ex, "closing")?;
    let opening_nfp = match (input.balances, &input.opening) {
        (Balances::Average, Some(o)) => Some(o.net_financial_position.total(&mut ex, "opening")?),
        _ => None,
    };
    let nfp = basis.resolve(&mut ex, "net financial position", closing_nfp, opening_nfp)?;

    let is = &input.income_statement;
    let interest = term("interest", is.interest);
    let ebitda = term("EBITDA", is.ebitda);
    let interest_cover_ebit = both_positive(
        "Interest cover, EBIT",
        "EBIT / interest",
        term("EBIT", is.ebit),
        interest,
    )
    .evaluate(&mut ex)?;
    let interest_cover_ebitda = both_positive(
        "Interest cover, EBITDA",
        "EBITDA / interest",
        ebitda,
        interest,
    )
    .evaluate(&mut ex)?;
    let nfp_to_ebitda = over_positive(
        "NFP / EBITDA",
        "net financial position / EBITDA",
        nfp,
        ebitda,
    )
    .evaluate(&mut ex)?;

    let Some(schedule) = &input.debt_schedule else {
        ex.convention(format!("DSCR and LLCR not defined: {NO_SCHEDULE}"));
        return Ok((
            DebtServiceOutput {
                net_financial_position: nfp.value,
                interest_cover_ebit,
                interest_cover_ebitda,
                nfp_to_ebitda,
                dscr: Vec::new(),
                pv_cfads: None,
                llcr: RatioValue {
                    value: None,
                    not_defined: Some(NO_SCHEDULE.to_owned()),
                },
            },
            ex,
        ));
    };
    let r = schedule.discount_rate;
    let n = schedule.periods.len();
    ex.convention(format!(
        "CFADS discounted at the end of each period at {r} per period, the cost of debt chosen \
         by the analyst"
    ));
    ex.convention(format!(
        "LLCR over the {n} periods of the schedule, debt outstanding at the start of the first \
         period"
    ));
    match schedule.reserve_balance {
        Some(reserve) => ex.convention(format!(
            "reserve balance {reserve} added undiscounted to the LLCR numerator"
        )),
        None => ex.convention("no reserve balance given: LLCR on CFADS only"),
    }

    let mut dscr = Vec::with_capacity(n);
    let mut pv = Decimal::ZERO;
    let mut present_values = Vec::with_capacity(n);
    for (i, p) in schedule.periods.iter().enumerate() {
        let t = i + 1;
        let label = format!("DSCR, period {t}");
        let ratio = over_positive(
            "DSCR",
            "CFADS / debt service",
            term("CFADS", p.cfads),
            term("debt service", p.debt_service),
        );
        dscr.push(Labelled(&label, &ratio).evaluate(&mut ex)?);
        let df = discount_factor(r, Decimal::from(t as u64))
            .map_err(rate_field("/debt_schedule/discount_rate", None))?;
        let pv_t = num::mul(p.cfads, df)?;
        ex.step(
            format!("Present value of CFADS, period {t}"),
            format!(
                "CFADS × discount factor = {} × {df} (discount factor = 1 / (1 + {r})^{t})",
                p.cfads
            ),
            pv_t,
        );
        pv = num::add(pv, pv_t)?;
        present_values.push(pv_t.to_string());
    }
    ex.step(
        "Present value of CFADS",
        format!("Σ PV CFADS_t = {}", present_values.join(" + ")),
        pv,
    );
    let (numerator, formula) = match schedule.reserve_balance {
        Some(reserve) => {
            let total = num::add(pv, reserve)?;
            ex.step(
                "Present value of CFADS and reserve balance",
                format!("present value of CFADS + reserve balance = {pv} + {reserve}"),
                total,
            );
            (
                term("present value of CFADS and reserve balance", total),
                "(present value of CFADS + reserve balance) / debt outstanding",
            )
        }
        None => (
            term("present value of CFADS", pv),
            "present value of CFADS / debt outstanding",
        ),
    };
    let llcr = over_positive(
        "LLCR",
        formula,
        numerator,
        term("debt outstanding", schedule.debt_outstanding),
    )
    .evaluate(&mut ex)?;

    Ok((
        DebtServiceOutput {
            net_financial_position: nfp.value,
            interest_cover_ebit,
            interest_cover_ebitda,
            nfp_to_ebitda,
            dscr,
            pv_cfads: Some(pv),
            llcr,
        },
        ex,
    ))
}

/// A ratio under a label built at run time ("DSCR, period 3").
struct Labelled<'a, R: Ratio>(&'a str, &'a R);

impl<R: Ratio> Ratio for Labelled<'_, R> {
    fn label(&self) -> &str {
        self.0
    }
    fn formula(&self) -> &str {
        self.1.formula()
    }
    fn numerator(&self) -> super::Term {
        self.1.numerator()
    }
    fn denominator(&self) -> super::Term {
        self.1.denominator()
    }
    fn days(&self) -> Option<Decimal> {
        self.1.days()
    }
    fn needs_positive_denominator(&self) -> bool {
        self.1.needs_positive_denominator()
    }
    fn needs_positive_numerator(&self) -> bool {
        self.1.needs_positive_numerator()
    }
}

/// `ratios.debt_service`: see the methodology page.
pub struct DebtService;

impl Method for DebtService {
    const ID: &'static str = "ratios.debt_service";
    const TITLE: &'static str = "Ratios — debt service";
    const DESCRIPTION: &'static str =
        "Interest cover on EBIT and EBITDA, net financial position to EBITDA, DSCR per period \
        and LLCR on a loan schedule with an explicit discount rate. A ratio that cannot be \
        computed is reported as not defined with the reason.";
    const VERSION: &'static str = "1";
    type Input = DebtServiceInput;
    type Output = DebtServiceOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        debt_service(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(v: i64) -> Decimal {
        Decimal::from(v)
    }

    fn input() -> DebtServiceInput {
        DebtServiceInput {
            balances: Balances::Closing,
            ebitda_definition: "reported".into(),
            interest_definition: "gross interest expense".into(),
            income_statement: DebtServiceIncome {
                ebitda: d(800),
                ebit: d(500),
                interest: d(100),
            },
            closing: DebtServiceBalances {
                net_financial_position: NetFinancialPosition {
                    gross_debt: d(2200),
                    cash: d(300),
                    other_items: vec![],
                },
            },
            opening: None,
            debt_schedule: Some(DebtSchedule {
                discount_rate: Decimal::ZERO,
                debt_outstanding: d(500),
                reserve_balance: None,
                periods: vec![
                    DebtPeriod {
                        cfads: d(120),
                        debt_service: d(0),
                    },
                    DebtPeriod {
                        cfads: d(150),
                        debt_service: d(100),
                    },
                ],
            }),
        }
    }

    fn schedule(i: &mut DebtServiceInput) -> &mut DebtSchedule {
        i.debt_schedule.as_mut().unwrap()
    }

    #[test]
    fn a_period_without_debt_service_is_not_defined_and_the_others_are_computed() {
        let (out, _) = debt_service(&input()).unwrap();
        assert_eq!(
            out.dscr[0].not_defined.as_deref(),
            Some("debt service is zero")
        );
        assert_eq!(out.dscr[1].value, Some(Decimal::new(15, 1)));
        // At a zero rate the present value is the plain sum of CFADS.
        assert_eq!(out.pv_cfads, Some(d(270)));
        assert_eq!(out.llcr.value, Some(Decimal::new(54, 2)));
    }

    #[test]
    fn zero_debt_outstanding_leaves_llcr_not_defined() {
        let mut i = input();
        schedule(&mut i).debt_outstanding = Decimal::ZERO;
        let (out, _) = debt_service(&i).unwrap();
        assert_eq!(
            out.llcr.not_defined.as_deref(),
            Some("debt outstanding is zero")
        );
    }

    #[test]
    fn present_value_steps_show_the_numbers() {
        let mut i = input();
        schedule(&mut i).discount_rate = Decimal::new(25, 2);
        let (_, ex) = debt_service(&i).unwrap();
        let step = ex
            .steps
            .iter()
            .find(|s| s.label == "Present value of CFADS, period 1")
            .unwrap();
        assert_eq!(
            step.formula,
            "CFADS × discount factor = 120 × 0.8 (discount factor = 1 / (1 + 0.25)^1)"
        );
        let sum = ex
            .steps
            .iter()
            .find(|s| s.label == "Present value of CFADS")
            .unwrap();
        assert_eq!(sum.formula, "Σ PV CFADS_t = 96.0 + 96.00");
    }

    #[test]
    fn blank_interest_definition_is_refused() {
        let mut i = input();
        i.interest_definition = String::new();
        assert_eq!(debt_service(&i).unwrap_err().code(), "FM-E001");
    }

    #[test]
    fn without_a_schedule_dscr_and_llcr_are_not_defined() {
        let mut i = input();
        i.debt_schedule = None;
        let (out, _) = debt_service(&i).unwrap();
        assert!(out.dscr.is_empty());
        assert_eq!(out.pv_cfads, None);
        assert_eq!(
            out.llcr.not_defined.as_deref(),
            Some("no debt schedule given")
        );
        assert!(out.interest_cover_ebit.value.is_some());
    }

    #[test]
    fn reserve_balance_is_added_undiscounted() {
        let mut i = input();
        schedule(&mut i).reserve_balance = Some(d(30));
        let (out, _) = debt_service(&i).unwrap();
        assert_eq!(out.llcr.value, Some(Decimal::new(6, 1)));
    }

    #[test]
    fn not_meaningful_signs() {
        let mut i = input();
        i.income_statement.ebit = d(-50);
        i.income_statement.ebitda = Decimal::ZERO;
        schedule(&mut i).periods[0].debt_service = d(-10);
        schedule(&mut i).debt_outstanding = d(-1);
        let (out, _) = debt_service(&i).unwrap();
        assert_eq!(
            out.interest_cover_ebit.not_defined.as_deref(),
            Some("EBIT is negative (-50)")
        );
        assert_eq!(
            out.interest_cover_ebitda.not_defined.as_deref(),
            Some("EBITDA is zero")
        );
        assert_eq!(
            out.nfp_to_ebitda.not_defined.as_deref(),
            Some("EBITDA is zero")
        );
        assert_eq!(
            out.dscr[0].not_defined.as_deref(),
            Some("debt service is negative (-10)")
        );
        assert_eq!(
            out.llcr.not_defined.as_deref(),
            Some("debt outstanding is negative (-1)")
        );
    }
}
