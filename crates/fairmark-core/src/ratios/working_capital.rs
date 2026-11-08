//! `ratios.working_capital`: how much does working capital tie up? (SPEC-0007)

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{all_defined, term, BalanceBasis, Balances, DayBasis, DayRatio, Ratio, RatioValue};
use crate::error::Result;
use crate::explain::Explain;
use crate::method::Method;
use crate::num;

/// Flows of the period, in currency units.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkingCapitalIncome {
    /// Revenue of the period, in currency units, on the same VAT basis as trade receivables.
    pub revenue: Decimal,
    /// Cost of sales of the period, in currency units.
    pub cost_of_sales: Decimal,
    /// Purchases of goods and services of the period, in currency units, on the same VAT basis
    /// as trade payables.
    pub purchases: Decimal,
}

/// Balance-sheet amounts at one date, in currency units.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkingCapitalBalances {
    /// Trade receivables, in currency units.
    pub trade_receivables: Decimal,
    /// Inventories, in currency units.
    pub inventories: Decimal,
    /// Trade payables, in currency units.
    pub trade_payables: Decimal,
}

/// Input of `ratios.working_capital`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkingCapitalInput {
    /// Balances used by the ratios: `closing`, or `average` of opening and closing.
    pub balances: Balances,
    /// Days in the year: "360" or "365".
    pub day_basis: DayBasis,
    /// Flows of the period (one year).
    pub income_statement: WorkingCapitalIncome,
    /// Balance sheet at the end of the period.
    pub closing: WorkingCapitalBalances,
    /// Balance sheet at the start of the period. Required with average balances; not used
    /// with closing balances.
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
}

pub fn working_capital(input: &WorkingCapitalInput) -> Result<(WorkingCapitalOutput, Explain)> {
    let mut ex = Explain::new(
        WorkingCapital::ID,
        "DSO = trade receivables × days / revenue; DIO = inventories × days / cost of sales; \
         DPO = trade payables × days / purchases; cash conversion cycle = DSO + DIO − DPO",
    );
    let basis = BalanceBasis::new(input.balances, input.opening.is_some(), &mut ex)?;
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

    let close = &input.closing;
    let open = input.opening.as_ref();
    let receivables = basis.resolve(
        &mut ex,
        "trade receivables",
        close.trade_receivables,
        open.map(|o| o.trade_receivables),
    )?;
    let inventories = basis.resolve(
        &mut ex,
        "inventories",
        close.inventories,
        open.map(|o| o.inventories),
    )?;
    let payables = basis.resolve(
        &mut ex,
        "trade payables",
        close.trade_payables,
        open.map(|o| o.trade_payables),
    )?;
    let is = &input.income_statement;

    let dso = DayRatio {
        label: "Days sales outstanding",
        formula: "trade receivables × days / revenue",
        numerator: receivables,
        denominator: term("revenue", is.revenue),
        basis: input.day_basis,
    }
    .evaluate(&mut ex)?;
    let dio = DayRatio {
        label: "Days inventory outstanding",
        formula: "inventories × days / cost of sales",
        numerator: inventories,
        denominator: term("cost of sales", is.cost_of_sales),
        basis: input.day_basis,
    }
    .evaluate(&mut ex)?;
    let dpo = DayRatio {
        label: "Days payable outstanding",
        formula: "trade payables × days / purchases",
        numerator: payables,
        denominator: term("purchases", is.purchases),
        basis: input.day_basis,
    }
    .evaluate(&mut ex)?;
    let ccc = match all_defined(&[
        ("days sales outstanding", &dso),
        ("days inventory outstanding", &dio),
        ("days payable outstanding", &dpo),
    ]) {
        Ok(v) => {
            let value = num::sub(num::add(v[0], v[1])?, v[2])?;
            ex.step(
                "Cash conversion cycle",
                format!("DSO + DIO − DPO = {} + {} − {}", v[0], v[1], v[2]),
                value,
            );
            RatioValue::defined(value)
        }
        Err(reason) => RatioValue::not_defined(&mut ex, "Cash conversion cycle", reason),
    };

    Ok((
        WorkingCapitalOutput {
            days_sales_outstanding: dso,
            days_inventory_outstanding: dio,
            days_payable_outstanding: dpo,
            cash_conversion_cycle: ccc,
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
        cash conversion cycle, on closing or average balances and a 360 or 365-day basis. A \
        ratio that cannot be computed is reported as not defined with the reason.";
    const VERSION: &'static str = "1";
    type Input = WorkingCapitalInput;
    type Output = WorkingCapitalOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        working_capital(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(v: i64) -> Decimal {
        Decimal::from(v)
    }

    fn input(purchases: i64) -> WorkingCapitalInput {
        WorkingCapitalInput {
            balances: Balances::Closing,
            day_basis: DayBasis::Days365,
            income_statement: WorkingCapitalIncome {
                revenue: d(5000),
                cost_of_sales: d(3000),
                purchases: d(purchases),
            },
            closing: WorkingCapitalBalances {
                trade_receivables: d(1200),
                inventories: d(600),
                trade_payables: d(700),
            },
            opening: None,
        }
    }

    #[test]
    fn cash_conversion_cycle_is_dso_plus_dio_minus_dpo() {
        let (out, _) = working_capital(&input(2800)).unwrap();
        assert_eq!(out.cash_conversion_cycle.value, Some(Decimal::new(6935, 2)));
    }

    #[test]
    fn one_undefined_part_leaves_the_cycle_not_defined() {
        let (out, _) = working_capital(&input(0)).unwrap();
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
}
