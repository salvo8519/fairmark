//! `ratios.debt_service`: can the company service its debt? (SPEC-0007)

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    both_positive, defined_amount, given, nfp_totals, other_items_defaults, over_positive,
    require_definition, term, union, warn_defaults, BalanceBasis, Balances, Given, Missing,
    NfpComposition, NotComputedList, Ratio, RatioValue, MISSING_CONVENTION, NFP_CONVENTION,
};
use crate::error::{rate_field, FairmarkError, Found, Reason, Result};
use crate::explain::{DefaultApplied, Explain};
use crate::method::Method;
use crate::not_computed::NotComputedEntry;
use crate::num;
use crate::tvm::discount_factor;

/// Flows of the period, in currency units. Every amount is optional: absent or null means not
/// given, and the ratios that need it are not computed (never taken as zero).
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DebtServiceIncome {
    /// EBITDA of the period, in currency units, as defined in `ebitda_definition` (the ratios
    /// on EBITDA need both).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ebitda: Option<Decimal>,
    /// EBIT (operating result) of the period, in currency units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ebit: Option<Decimal>,
    /// Interest expense of the period, in currency units, as defined in `interest_definition`
    /// (gross, or net of interest income); the interest covers need both.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interest: Option<Decimal>,
}

/// Balance-sheet amounts at one date.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DebtServiceBalances {
    /// Net financial position with its composition, in currency units. Absent or null: not
    /// given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub net_financial_position: Option<NfpComposition>,
}

/// One period of the loan schedule.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DebtPeriod {
    /// Cash flow available for debt service in the period, in currency units. Absent or null:
    /// not given; the DSCR of the period, the present value of CFADS and the LLCR are not
    /// computed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cfads: Option<Decimal>,
    /// Debt service (principal and interest) due in the period, in currency units. Absent or
    /// null: not given; the DSCR of the period is not computed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub debt_service: Option<Decimal>,
}

/// The loan schedule from the calculation date to the final maturity.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DebtSchedule {
    /// Discount rate for the LLCR: the cost of debt per period of the schedule chosen by the
    /// analyst, as a decimal fraction (0.06 = 6% per period). Absent or null: not given; the
    /// present value of CFADS and the LLCR are not computed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discount_rate: Option<Decimal>,
    /// Debt outstanding at the start of the first period, in currency units. Absent or null:
    /// not given; the LLCR is not computed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub debt_outstanding: Option<Decimal>,
    /// Balance of the debt service reserve account at the valuation date, in currency units,
    /// added undiscounted to the LLCR numerator. Absent: no reserve account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reserve_balance: Option<Decimal>,
    /// Periods 1..n, in order, up to the final maturity of the loan. At least one.
    pub periods: Vec<DebtPeriod>,
}

/// Input of `ratios.debt_service` (version 2). Only `balances` is required; each ratio is
/// computed when its own amounts are given (SPEC-0007 R7).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DebtServiceInput {
    /// Balances used by the ratios: `closing`, or `average` of opening and closing.
    pub balances: Balances,
    /// How EBITDA was measured, in words (for example "reported EBITDA, no adjustments");
    /// echoed in the conventions. Absent or null: the ratios on EBITDA are not computed; given
    /// blank: refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ebitda_definition: Option<String>,
    /// How interest was measured, in words (for example "gross interest expense" or "interest
    /// expense net of interest income"); echoed in the conventions. Absent or null: the interest
    /// covers are not computed; given blank: refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interest_definition: Option<String>,
    /// Flows of the period. Absent or null: no flow given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub income_statement: Option<DebtServiceIncome>,
    /// Balance sheet at the end of the period. Absent or null: no amount given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closing: Option<DebtServiceBalances>,
    /// Balance sheet at the start of the period, used with average balances; not used with
    /// closing balances. Absent with average balances: the net financial position is not
    /// computed.
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
    /// Net financial position used by the ratios, in currency units (positive for net debt);
    /// null when not computed.
    pub net_financial_position: Option<Decimal>,
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
    /// null when no debt schedule is given or when it is not computed.
    pub pv_cfads: Option<Decimal>,
    /// (Present value of CFADS + reserve balance) / debt outstanding, multiple.
    pub llcr: RatioValue,
    /// Outputs not computed because an input is missing, in the order of the fields above,
    /// each with the JSON pointers of the inputs it lacks (SPEC-0007 R7).
    pub not_computed: Vec<NotComputedEntry>,
}

pub(crate) const NO_SCHEDULE: &str = "no debt schedule given";

const RATE_POINTER: &str = "/debt_schedule/discount_rate";
const RESERVE_POINTER: &str = "/debt_schedule/reserve_balance";

/// The documented defaults of an input (R13): `other_items` of each net financial position read,
/// and `reserve_balance` when the LLCR has every other input (rate, every CFADS, debt
/// outstanding). The reserve default is reported, not written into the input, so the hash is
/// unchanged and a reserve given as zero stays distinct (SPEC-0001 Decision 25).
fn defaults(input: &DebtServiceInput) -> Vec<DefaultApplied> {
    fn nfp(b: &Option<DebtServiceBalances>) -> Option<&NfpComposition> {
        b.as_ref().and_then(|b| b.net_financial_position.as_ref())
    }
    let opening = match input.balances {
        Balances::Average => nfp(&input.opening),
        Balances::Closing => None,
    };
    let mut out = other_items_defaults([("closing", nfp(&input.closing)), ("opening", opening)]);
    if let Some(s) = &input.debt_schedule {
        if s.reserve_balance.is_none()
            && s.discount_rate.is_some()
            && s.debt_outstanding.is_some()
            && s.periods.iter().all(|p| p.cfads.is_some())
        {
            out.push(DefaultApplied::new(
                RESERVE_POINTER,
                serde_json::json!("0"),
                "no reserve balance given: no debt service reserve account, nothing is added to \
                 the LLCR numerator",
            ));
        }
    }
    out
}

pub fn debt_service(input: &DebtServiceInput) -> Result<(DebtServiceOutput, Explain)> {
    // Every input error is reported (ADR 0014 G5), in the order of the checks.
    let mut found = Found::new();
    if input
        .debt_schedule
        .as_ref()
        .is_some_and(|s| s.periods.is_empty())
    {
        found.push(
            FairmarkError::EmptyCashFlows
                .at("/debt_schedule/periods", Reason::Length)
                .fix("give at least one period, or leave out `debt_schedule`"),
        );
    }
    // A rate given is checked whatever else is given (ADR 0014 G10.3): present and invalid is
    // an error, also when the present value of CFADS is not computed.
    if let Some(r) = input.debt_schedule.as_ref().and_then(|s| s.discount_rate) {
        found.check(
            discount_factor(r, Decimal::ONE)
                .map(|_| ())
                .map_err(rate_field(RATE_POINTER, None)),
        );
    }
    found.check(require_definition(
        "ebitda_definition",
        input.ebitda_definition.as_deref(),
    ));
    found.check(require_definition(
        "interest_definition",
        input.interest_definition.as_deref(),
    ));
    found.finish()?;
    let defaults = defaults(input);
    let mut ex = Explain::new(
        DebtService::ID,
        "interest cover = EBIT (or EBITDA) / interest; NFP / EBITDA; DSCR_t = CFADS_t / debt \
         service_t; LLCR = (Σ CFADS_t / (1 + r)^t + reserve balance) / debt outstanding",
    );
    let ebitda_definition = input.ebitda_definition.as_deref();
    let interest_definition = input.interest_definition.as_deref();
    let basis = BalanceBasis::new(input.balances, input.opening.is_some(), &mut ex);
    let is = input.income_statement.clone().unwrap_or_default();
    for (what, definition, amount) in [
        ("EBITDA", ebitda_definition, is.ebitda),
        ("interest", interest_definition, is.interest),
    ] {
        match definition {
            Some(d) => ex.convention(format!("{what} as given: {d}")),
            None if amount.is_some() => ex.convention(format!(
                "{what} given without {}_definition: the ratios on {what} are not computed",
                what.to_lowercase()
            )),
            None => {}
        }
    }
    ex.convention(NFP_CONVENTION);
    warn_defaults(&mut ex, &defaults, "/other_items");
    ex.convention("debt service as given");
    ex.convention("a ratio with a zero denominator is not defined");
    ex.convention(
        "not meaningful, so not defined: an interest cover when interest or its numerator (EBIT \
         or EBITDA) is zero or negative; NFP / EBITDA when EBITDA is zero or negative; DSCR \
         when debt service is zero or negative; LLCR when debt outstanding is zero or negative",
    );
    ex.convention(MISSING_CONVENTION);

    let (closing_nfp, opening_nfp) = nfp_totals(
        &basis,
        &mut ex,
        input
            .closing
            .as_ref()
            .and_then(|c| c.net_financial_position.as_ref()),
        input
            .opening
            .as_ref()
            .and_then(|o| o.net_financial_position.as_ref()),
    )?;
    let nfp = basis.resolve(&mut ex, "net financial position", closing_nfp, opening_nfp)?;

    let interest = defined_amount(
        "interest",
        is.interest,
        "/income_statement/interest",
        interest_definition,
        "/interest_definition",
    );
    let ebitda = defined_amount(
        "EBITDA",
        is.ebitda,
        "/income_statement/ebitda",
        ebitda_definition,
        "/ebitda_definition",
    );
    let ebit = given("EBIT", is.ebit, "/income_statement/ebit");
    let interest_cover_ebit =
        both_positive("Interest cover, EBIT", "EBIT / interest", &ebit, &interest)
            .evaluate(&mut ex)?;
    let interest_cover_ebitda = both_positive(
        "Interest cover, EBITDA",
        "EBITDA / interest",
        &ebitda,
        &interest,
    )
    .evaluate(&mut ex)?;
    let nfp_to_ebitda = over_positive(
        "NFP / EBITDA",
        "net financial position / EBITDA",
        &nfp,
        &ebitda,
    )
    .evaluate(&mut ex)?;

    let mut nc = NotComputedList::default();
    let net_financial_position = nc.amount("/net_financial_position", &nfp);
    let interest_cover_ebit = nc.ratio("/interest_cover_ebit", interest_cover_ebit);
    let interest_cover_ebitda = nc.ratio("/interest_cover_ebitda", interest_cover_ebitda);
    let nfp_to_ebitda = nc.ratio("/nfp_to_ebitda", nfp_to_ebitda);

    let Some(schedule) = &input.debt_schedule else {
        ex.convention(format!("DSCR and LLCR not defined: {NO_SCHEDULE}"));
        let no_schedule = Missing::from(["/debt_schedule".to_owned()]);
        nc.push("/pv_cfads", &no_schedule);
        nc.observe(&no_schedule);
        let llcr = RatioValue::block_not_given(NO_SCHEDULE, "/debt_schedule");
        let llcr = nc.ratio("/llcr", llcr);
        return Ok((
            DebtServiceOutput {
                net_financial_position,
                interest_cover_ebit,
                interest_cover_ebitda,
                nfp_to_ebitda,
                dscr: Vec::new(),
                pv_cfads: None,
                llcr,
                not_computed: nc.finish(DebtService::ID)?,
            },
            ex,
        ));
    };
    let n = schedule.periods.len();
    let rate = given(
        "discount rate",
        schedule.discount_rate,
        "/debt_schedule/discount_rate",
    );
    let cfads: Vec<Given> = schedule
        .periods
        .iter()
        .enumerate()
        .map(|(i, p)| {
            given(
                "CFADS",
                p.cfads,
                &format!("/debt_schedule/periods/{i}/cfads"),
            )
        })
        .collect();
    let pv_missing = union(std::iter::once(&rate).chain(&cfads));
    match schedule.discount_rate {
        Some(r) => ex.convention(format!(
            "CFADS discounted at the end of each period at {r} per period, the cost of debt \
             chosen by the analyst"
        )),
        None => ex.convention(
            "discount rate not given: the present value of CFADS and the LLCR are not computed",
        ),
    }
    ex.convention(format!(
        "LLCR over the {n} periods of the schedule, debt outstanding at the start of the first \
         period"
    ));
    match schedule.reserve_balance {
        Some(reserve) => ex.convention(format!(
            "reserve balance {reserve} added undiscounted to the LLCR numerator"
        )),
        None if defaults.iter().any(|d| d.field == RESERVE_POINTER) => ex.convention(format!(
            "warning: no reserve balance given (`{RESERVE_POINTER}`): nothing is added to the \
             present value of CFADS, so the LLCR assumes no debt service reserve account; give \
             the reserve balance, or 0 for none"
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
            &cfads[i],
            &given(
                "debt service",
                p.debt_service,
                &format!("/debt_schedule/periods/{i}/debt_service"),
            ),
        );
        let value = Labelled(&label, &ratio).evaluate(&mut ex)?;
        dscr.push(nc.ratio(&format!("/dscr/{i}"), value));
        let (Some(r), Some(flow), true) = (schedule.discount_rate, p.cfads, pv_missing.is_empty())
        else {
            continue;
        };
        let df =
            discount_factor(r, Decimal::from(t as u64)).map_err(rate_field(RATE_POINTER, None))?;
        let pv_t = num::mul(flow, df)?;
        ex.step(
            format!("Present value of CFADS, period {t}"),
            format!(
                "CFADS × discount factor = {flow} × {df} (discount factor = 1 / (1 + {r})^{t})"
            ),
            pv_t,
        );
        pv = num::add(pv, pv_t)?;
        present_values.push(pv_t.to_string());
    }
    let pv_cfads: Given = if pv_missing.is_empty() {
        ex.step(
            "Present value of CFADS",
            format!("Σ PV CFADS_t = {}", present_values.join(" + ")),
            pv,
        );
        term("present value of CFADS", pv)
    } else {
        Err(pv_missing)
    };
    let (numerator, formula) = match (schedule.reserve_balance, &pv_cfads) {
        (Some(reserve), Ok(pv)) => {
            let total = num::add(pv.value, reserve)?;
            ex.step(
                "Present value of CFADS and reserve balance",
                format!(
                    "present value of CFADS + reserve balance = {} + {reserve}",
                    pv.value
                ),
                total,
            );
            (
                term("present value of CFADS and reserve balance", total),
                "(present value of CFADS + reserve balance) / debt outstanding",
            )
        }
        (Some(_), Err(_)) => (
            pv_cfads.clone(),
            "(present value of CFADS + reserve balance) / debt outstanding",
        ),
        (None, _) => (
            pv_cfads.clone(),
            "present value of CFADS / debt outstanding",
        ),
    };
    let llcr = over_positive(
        "LLCR",
        formula,
        &numerator,
        &given(
            "debt outstanding",
            schedule.debt_outstanding,
            "/debt_schedule/debt_outstanding",
        ),
    )
    .evaluate(&mut ex)?;
    let pv_cfads = nc.amount("/pv_cfads", &pv_cfads);
    let llcr = nc.ratio("/llcr", llcr);

    Ok((
        DebtServiceOutput {
            net_financial_position,
            interest_cover_ebit,
            interest_cover_ebitda,
            nfp_to_ebitda,
            dscr,
            pv_cfads,
            llcr,
            not_computed: nc.finish(DebtService::ID)?,
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
    fn numerator(&self) -> &Given {
        self.1.numerator()
    }
    fn denominator(&self) -> &Given {
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
        and LLCR on a loan schedule with an explicit discount rate. Every amount is optional: a \
        ratio whose amounts are not all given is null and listed in not_computed with the \
        inputs it lacks; a ratio that is not meaningful is reported as not defined with the \
        reason.";
    const VERSION: &'static str = "2";
    type Input = DebtServiceInput;
    type Output = DebtServiceOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        debt_service(input)
    }

    fn apply_defaults(input: &mut Self::Input) -> Vec<DefaultApplied> {
        defaults(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(v: i64) -> Option<Decimal> {
        Some(Decimal::from(v))
    }

    fn input() -> DebtServiceInput {
        DebtServiceInput {
            balances: Balances::Closing,
            ebitda_definition: Some("reported".into()),
            interest_definition: Some("gross interest expense".into()),
            income_statement: Some(DebtServiceIncome {
                ebitda: d(800),
                ebit: d(500),
                interest: d(100),
            }),
            closing: Some(DebtServiceBalances {
                net_financial_position: Some(NfpComposition {
                    gross_debt: d(2200),
                    cash: d(300),
                    other_items: Some(vec![]),
                }),
            }),
            opening: None,
            debt_schedule: Some(DebtSchedule {
                discount_rate: Some(Decimal::ZERO),
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
        assert_eq!(out.pv_cfads, d(270));
        assert_eq!(out.llcr.value, Some(Decimal::new(54, 2)));
    }

    #[test]
    fn zero_debt_outstanding_leaves_llcr_not_defined() {
        let mut i = input();
        schedule(&mut i).debt_outstanding = Some(Decimal::ZERO);
        let (out, _) = debt_service(&i).unwrap();
        assert_eq!(
            out.llcr.not_defined.as_deref(),
            Some("debt outstanding is zero")
        );
    }

    #[test]
    fn present_value_steps_show_the_numbers() {
        let mut i = input();
        schedule(&mut i).discount_rate = Some(Decimal::new(25, 2));
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
        i.interest_definition = Some(String::new());
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
        let listed: Vec<_> = out.not_computed.iter().map(|e| e.output.as_str()).collect();
        assert_eq!(listed, ["/pv_cfads", "/llcr"]);
    }

    #[test]
    fn a_missing_cfads_leaves_its_dscr_the_present_value_and_llcr_not_computed() {
        let mut i = input();
        schedule(&mut i).periods[1].cfads = None;
        let (out, ex) = debt_service(&i).unwrap();
        assert_eq!(out.pv_cfads, None);
        assert_eq!(out.llcr.value, None);
        let listed: Vec<_> = out.not_computed.iter().map(|e| e.output.as_str()).collect();
        assert_eq!(listed, ["/dscr/1", "/pv_cfads", "/llcr"]);
        assert!(!ex
            .steps
            .iter()
            .any(|s| s.label.starts_with("Present value")));
        assert_eq!(
            out.dscr[0].not_defined.as_deref(),
            Some("debt service is zero")
        );
    }

    #[test]
    fn interest_without_its_definition_leaves_the_covers_not_computed() {
        let mut i = input();
        i.interest_definition = None;
        let (out, _) = debt_service(&i).unwrap();
        assert_eq!(out.interest_cover_ebit.value, None);
        assert_eq!(out.not_computed[0].missing, ["/interest_definition"]);
        assert!(out.nfp_to_ebitda.value.is_some());
    }

    #[test]
    fn reserve_balance_is_added_undiscounted() {
        let mut i = input();
        schedule(&mut i).reserve_balance = d(30);
        let (out, _) = debt_service(&i).unwrap();
        assert_eq!(out.llcr.value, Some(Decimal::new(6, 1)));
    }

    #[test]
    fn not_meaningful_signs() {
        let mut i = input();
        let is = i.income_statement.as_mut().unwrap();
        is.ebit = d(-50);
        is.ebitda = Some(Decimal::ZERO);
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
