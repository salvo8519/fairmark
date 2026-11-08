//! `ratios.debt_service`: can the company service its debt? (SPEC-0007)

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    quotient, term, BalanceBasis, Balances, NetFinancialPosition, Ratio, RatioValue, NFP_CONVENTION,
};
use crate::error::{FairmarkError, Result};
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
    /// Interest expense of the period, in currency units, as the analyst measures it (gross, or
    /// net of interest income).
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
    /// Discount rate for the LLCR, per period of the schedule, as a decimal fraction (0.06 =
    /// 6% per period). Usually the interest rate of the loan.
    pub discount_rate: Decimal,
    /// Debt outstanding at the start of the first period, in currency units.
    pub debt_outstanding: Decimal,
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
    /// Flows of the period.
    pub income_statement: DebtServiceIncome,
    /// Balance sheet at the end of the period.
    pub closing: DebtServiceBalances,
    /// Balance sheet at the start of the period. Required with average balances; not used
    /// with closing balances.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opening: Option<DebtServiceBalances>,
    /// Loan schedule for DSCR and LLCR.
    pub debt_schedule: DebtSchedule,
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
    /// CFADS / debt service of each period, multiple, in the order of the schedule.
    pub dscr: Vec<RatioValue>,
    /// Present value of the CFADS of every period at the calculation date, in currency units.
    pub pv_cfads: Decimal,
    /// Present value of CFADS / debt outstanding, multiple.
    pub llcr: RatioValue,
}

pub fn debt_service(input: &DebtServiceInput) -> Result<(DebtServiceOutput, Explain)> {
    let schedule = &input.debt_schedule;
    if schedule.periods.is_empty() {
        return Err(FairmarkError::EmptyCashFlows);
    }
    let mut ex = Explain::new(
        DebtService::ID,
        "interest cover = EBIT (or EBITDA) / interest; NFP / EBITDA; DSCR_t = CFADS_t / debt \
         service_t; LLCR = Σ CFADS_t / (1 + r)^t / debt outstanding",
    );
    let basis = BalanceBasis::new(input.balances, input.opening.is_some(), &mut ex)?;
    ex.convention(format!("EBITDA as given: {}", input.ebitda_definition));
    ex.convention(NFP_CONVENTION);
    ex.convention("interest and debt service as given");
    let r = schedule.discount_rate;
    let n = schedule.periods.len();
    ex.convention(format!(
        "CFADS discounted at the end of each period at {r} per period"
    ));
    ex.convention(format!(
        "LLCR over the {n} periods of the schedule, debt outstanding at the start of the first \
         period"
    ));
    ex.convention("a ratio with a zero denominator is not defined");

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
    let interest_cover_ebit = quotient(
        "Interest cover, EBIT",
        "EBIT / interest",
        term("EBIT", is.ebit),
        interest,
    )
    .evaluate(&mut ex)?;
    let interest_cover_ebitda = quotient(
        "Interest cover, EBITDA",
        "EBITDA / interest",
        ebitda,
        interest,
    )
    .evaluate(&mut ex)?;
    let nfp_to_ebitda = quotient(
        "NFP / EBITDA",
        "net financial position / EBITDA",
        nfp,
        ebitda,
    )
    .evaluate(&mut ex)?;

    let mut dscr = Vec::with_capacity(n);
    let mut pv = Decimal::ZERO;
    for (i, p) in schedule.periods.iter().enumerate() {
        let t = i + 1;
        let label = format!("DSCR, period {t}");
        let ratio = quotient(
            "DSCR",
            "CFADS / debt service",
            term("CFADS", p.cfads),
            term("debt service", p.debt_service),
        );
        dscr.push(Labelled(&label, &ratio).evaluate(&mut ex)?);
        let df = discount_factor(r, Decimal::from(t as u64))?;
        let pv_t = num::mul(p.cfads, df)?;
        ex.step(
            format!("Present value of CFADS, period {t}"),
            format!("{} / (1 + {r})^{t}", p.cfads),
            pv_t,
        );
        pv = num::add(pv, pv_t)?;
    }
    ex.step("Present value of CFADS", "Σ PV CFADS_t", pv);
    let llcr = quotient(
        "LLCR",
        "present value of CFADS / debt outstanding",
        term("present value of CFADS", pv),
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
            pv_cfads: pv,
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
