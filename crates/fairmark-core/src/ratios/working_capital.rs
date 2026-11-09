//! `ratios.working_capital`: how much does working capital tie up? (SPEC-0007)

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    composite, given, BalanceBasis, Balances, DayBasis, DayRatio, NotComputedList, Ratio,
    RatioValue, MISSING_CONVENTION,
};
use crate::error::Result;
use crate::explain::Explain;
use crate::method::Method;
use crate::not_computed::NotComputedEntry;
use crate::num;

/// Flows of the period, in currency units. Every amount is optional: absent or null means not
/// given, and the ratios that need it are not computed (never taken as zero).
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkingCapitalIncome {
    /// Revenue of the period, in currency units, on the same VAT basis as trade receivables.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revenue: Option<Decimal>,
    /// Cost of sales of the period, in currency units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_of_sales: Option<Decimal>,
    /// Purchases of goods and services of the period, in currency units, on the same VAT basis
    /// as trade payables.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purchases: Option<Decimal>,
}

/// Balance-sheet amounts at one date, in currency units. Every amount is optional: absent or
/// null means not given.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkingCapitalBalances {
    /// Trade receivables, in currency units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trade_receivables: Option<Decimal>,
    /// Inventories, in currency units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inventories: Option<Decimal>,
    /// Trade payables, in currency units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trade_payables: Option<Decimal>,
}

/// Input of `ratios.working_capital` (version 2). Only `balances` and `day_basis` are
/// required; each ratio is computed when its own amounts are given (SPEC-0007 R7).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkingCapitalInput {
    /// Balances used by the ratios: `closing`, or `average` of opening and closing.
    pub balances: Balances,
    /// Days in the year: "360" or "365".
    pub day_basis: DayBasis,
    /// Flows of the period (one year). Absent or null: no flow given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub income_statement: Option<WorkingCapitalIncome>,
    /// Balance sheet at the end of the period. Absent or null: no amount given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closing: Option<WorkingCapitalBalances>,
    /// Balance sheet at the start of the period, used with average balances; not used with
    /// closing balances. Absent with average balances: no ratio is computed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opening: Option<WorkingCapitalBalances>,
}

/// Output of `ratios.working_capital`, in days.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct WorkingCapitalOutput {
    /// Trade receivables × days / revenue, days.
    pub days_sales_outstanding: RatioValue,
    /// Inventories × days / cost of sales, days.
    pub days_inventory_outstanding: RatioValue,
    /// Trade payables × days / purchases, days.
    pub days_payable_outstanding: RatioValue,
    /// DSO + DIO − DPO, days.
    pub cash_conversion_cycle: RatioValue,
    /// Outputs not computed because an input is missing, in the order of the fields above,
    /// each with the JSON pointers of the inputs it lacks (SPEC-0007 R7).
    pub not_computed: Vec<NotComputedEntry>,
}

pub fn working_capital(input: &WorkingCapitalInput) -> Result<(WorkingCapitalOutput, Explain)> {
    let mut ex = Explain::new(
        WorkingCapital::ID,
        "DSO = trade receivables × days / revenue; DIO = inventories × days / cost of sales; \
         DPO = trade payables × days / purchases; cash conversion cycle = DSO + DIO − DPO",
    );
    let basis = BalanceBasis::new(input.balances, input.opening.is_some(), &mut ex);
    let days = input.day_basis.days();
    ex.convention(format!("day basis: {days} days"));
    ex.convention("DSO on revenue");
    ex.convention("DIO on cost of sales");
    ex.convention("DPO on purchases");
    ex.convention(
        "balances and flows are used as given, so trade receivables and revenue must be stated \
         on the same VAT basis, and trade payables and purchases likewise",
    );
    ex.convention("a ratio with a zero denominator is not defined");
    ex.convention(MISSING_CONVENTION);

    let empty = WorkingCapitalBalances::default();
    let close = input.closing.as_ref().unwrap_or(&empty);
    let open = input.opening.as_ref();
    let amount =
        |ex: &mut Explain, name, path, f: fn(&WorkingCapitalBalances) -> Option<Decimal>| {
            basis.amount(ex, name, path, f(close), open.and_then(f))
        };
    let receivables = amount(&mut ex, "trade receivables", "trade_receivables", |b| {
        b.trade_receivables
    })?;
    let inventories = amount(&mut ex, "inventories", "inventories", |b| b.inventories)?;
    let payables = amount(&mut ex, "trade payables", "trade_payables", |b| {
        b.trade_payables
    })?;
    let is = input.income_statement.clone().unwrap_or_default();
    let flow = |name, value, field: &str| given(name, value, &format!("/income_statement/{field}"));

    let dso = DayRatio {
        label: "Days sales outstanding",
        formula: "trade receivables × days / revenue",
        numerator: receivables,
        denominator: flow("revenue", is.revenue, "revenue"),
        basis: input.day_basis,
    }
    .evaluate(&mut ex)?;
    let dio = DayRatio {
        label: "Days inventory outstanding",
        formula: "inventories × days / cost of sales",
        numerator: inventories,
        denominator: flow("cost of sales", is.cost_of_sales, "cost_of_sales"),
        basis: input.day_basis,
    }
    .evaluate(&mut ex)?;
    let dpo = DayRatio {
        label: "Days payable outstanding",
        formula: "trade payables × days / purchases",
        numerator: payables,
        denominator: flow("purchases", is.purchases, "purchases"),
        basis: input.day_basis,
    }
    .evaluate(&mut ex)?;
    let ccc = composite(
        &mut ex,
        "Cash conversion cycle",
        &[
            ("days sales outstanding", &dso),
            ("days inventory outstanding", &dio),
            ("days payable outstanding", &dpo),
        ],
        |ex, v| {
            let value = num::sub(num::add(v[0], v[1])?, v[2])?;
            ex.step(
                "Cash conversion cycle",
                format!("DSO + DIO − DPO = {} + {} − {}", v[0], v[1], v[2]),
                value,
            );
            Ok(value)
        },
    )?;

    let mut nc = NotComputedList::default();
    let output = WorkingCapitalOutput {
        days_sales_outstanding: nc.ratio("/days_sales_outstanding", dso),
        days_inventory_outstanding: nc.ratio("/days_inventory_outstanding", dio),
        days_payable_outstanding: nc.ratio("/days_payable_outstanding", dpo),
        cash_conversion_cycle: nc.ratio("/cash_conversion_cycle", ccc),
        not_computed: Vec::new(),
    };
    Ok((
        WorkingCapitalOutput {
            not_computed: nc.finish(),
            ..output
        },
        ex,
    ))
}

/// `ratios.working_capital`: see the methodology page.
pub struct WorkingCapital;

impl Method for WorkingCapital {
    const ID: &'static str = "ratios.working_capital";
    const TITLE: &'static str = "Ratios — working capital";
    const DESCRIPTION: &'static str =
        "Days sales outstanding, days inventory outstanding, days payable outstanding and the \
        cash conversion cycle, on closing or average balances and a 360 or 365-day basis. Every \
        amount is optional: a ratio whose amounts are not all given is null and listed in \
        not_computed with the inputs it lacks; a ratio with a zero denominator is reported as \
        not defined with the reason.";
    const VERSION: &'static str = "2";
    type Input = WorkingCapitalInput;
    type Output = WorkingCapitalOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        working_capital(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(v: i64) -> Option<Decimal> {
        Some(Decimal::from(v))
    }

    fn input(purchases: Option<Decimal>) -> WorkingCapitalInput {
        WorkingCapitalInput {
            balances: Balances::Closing,
            day_basis: DayBasis::Days365,
            income_statement: Some(WorkingCapitalIncome {
                revenue: d(5000),
                cost_of_sales: d(3000),
                purchases,
            }),
            closing: Some(WorkingCapitalBalances {
                trade_receivables: d(1200),
                inventories: d(600),
                trade_payables: d(700),
            }),
            opening: None,
        }
    }

    #[test]
    fn cash_conversion_cycle_is_dso_plus_dio_minus_dpo() {
        let (out, _) = working_capital(&input(d(2800))).unwrap();
        assert_eq!(out.cash_conversion_cycle.value, Some(Decimal::new(6935, 2)));
    }

    #[test]
    fn one_undefined_part_leaves_the_cycle_not_defined() {
        let (out, _) = working_capital(&input(d(0))).unwrap();
        assert_eq!(
            out.days_payable_outstanding.not_defined.as_deref(),
            Some("purchases is zero")
        );
        assert_eq!(
            out.cash_conversion_cycle.not_defined.as_deref(),
            Some("days payable outstanding is not defined")
        );
        assert!(out.days_sales_outstanding.value.is_some());
    }

    #[test]
    fn one_missing_part_leaves_the_cycle_not_computed() {
        let (out, _) = working_capital(&input(None)).unwrap();
        assert_eq!(out.cash_conversion_cycle.not_defined, None);
        assert_eq!(out.not_computed.len(), 2);
        assert_eq!(out.not_computed[1].output, "/cash_conversion_cycle");
        assert_eq!(out.not_computed[1].missing, ["/income_statement/purchases"]);
    }
}
