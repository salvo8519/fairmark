//! The relations of R7 and the identity of R12: each is checked exactly, with no tolerance,
//! when both sides are given (`reconciled`), reported as `not_checked` otherwise, and refused
//! with `FM-E026` when the two sides differ (Decision 2).

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use serde::Serialize;

use super::input::{StatutoryBalanceSheet, StatutoryIncomeStatement};
use crate::error::{FairmarkError, Reconciliation, Result};
use crate::num;

/// Outcome of one relation. A relation whose sides differ is not an outcome: it is `FM-E026`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    /// Both sides given and equal.
    Reconciled,
    /// A side is missing: the relation could not be checked.
    NotChecked,
}

/// One relation of R7 or R12 in the result.
#[derive(Debug, Clone, Serialize)]
pub struct Check {
    /// `reconciled` or `not_checked`.
    pub status: CheckStatus,
    /// JSON pointers of the inputs on both sides of the relation.
    pub pointers: Vec<String>,
}

/// One side of a relation: its words, the pointers of its inputs and its amount when known.
pub(crate) struct Side {
    pub name: String,
    pub pointers: Vec<String>,
    pub value: Option<Decimal>,
}

impl Side {
    pub(crate) fn new(
        name: impl Into<String>,
        pointers: Vec<String>,
        value: Option<Decimal>,
    ) -> Self {
        Self {
            name: name.into(),
            pointers,
            value,
        }
    }
}

/// The checks of one year, by relation id (sorted, so that a relation added later never
/// moves another in the result).
pub(crate) struct Checks {
    pub map: BTreeMap<&'static str, Check>,
    year: String,
}

impl Checks {
    pub(crate) fn new(year: &str) -> Self {
        Self {
            map: BTreeMap::new(),
            year: year.to_owned(),
        }
    }

    fn record(&mut self, relation: &'static str, status: CheckStatus, a: &Side, b: &Side) {
        let pointers = a.pointers.iter().chain(&b.pointers).cloned().collect();
        self.map.insert(relation, Check { status, pointers });
    }

    /// `left = right`, exactly.
    pub(crate) fn equal(
        &mut self,
        relation: &'static str,
        left: Side,
        right: Side,
        fix: &str,
    ) -> Result<()> {
        self.compare(relation, left, right, fix, |l, r| l == r)
    }

    /// `part ≤ whole`: an "of which" part or an amount beyond one year within its aggregate.
    pub(crate) fn at_most(
        &mut self,
        relation: &'static str,
        part: Side,
        whole: Side,
        fix: &str,
    ) -> Result<()> {
        self.compare(relation, part, whole, fix, |p, w| p <= w)
    }

    /// Records `relation` as not checked, for a relation without inputs in this year.
    pub(crate) fn not_checked(&mut self, relation: &'static str, pointers: Vec<String>) {
        self.map.insert(
            relation,
            Check {
                status: CheckStatus::NotChecked,
                pointers,
            },
        );
    }

    fn compare(
        &mut self,
        relation: &'static str,
        left: Side,
        right: Side,
        fix: &str,
        holds: impl Fn(Decimal, Decimal) -> bool,
    ) -> Result<()> {
        match (left.value, right.value) {
            (Some(l), Some(r)) => {
                if !holds(l, r) {
                    return Err(self.error(&left, l, &right, r, fix)?);
                }
                self.record(relation, CheckStatus::Reconciled, &left, &right);
            }
            _ => self.record(relation, CheckStatus::NotChecked, &left, &right),
        }
        Ok(())
    }

    /// `FM-E026` naming both sides by their pointers, with both amounts and the difference.
    pub(crate) fn error(
        &self,
        left: &Side,
        l: Decimal,
        right: &Side,
        r: Decimal,
        fix: &str,
    ) -> Result<FairmarkError> {
        Ok(FairmarkError::StatementsDoNotReconcile(Box::new(
            Reconciliation {
                left_name: format!("{} ({}, {})", left.name, quoted(&left.pointers), self.year),
                left: l,
                right_name: format!("{} ({})", right.name, quoted(&right.pointers)),
                right: r,
                difference: num::sub(l, r)?,
                fix: fix.to_owned(),
            },
        )))
    }
}

fn quoted(pointers: &[String]) -> String {
    pointers
        .iter()
        .map(|p| format!("`{p}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The sum of the values given, `None` when a term is missing.
pub(crate) fn sum_all(values: &[Option<Decimal>]) -> Result<Option<Decimal>> {
    let mut total = Decimal::ZERO;
    for v in values {
        match v {
            Some(v) => total = num::add(total, *v)?,
            None => return Ok(None),
        }
    }
    Ok(Some(total))
}

/// `Σ sign × value`, `None` when a term is missing.
pub(crate) fn signed_sum(terms: &[(i8, Option<Decimal>)]) -> Result<Option<Decimal>> {
    let mut total = Decimal::ZERO;
    for (sign, v) in terms {
        let Some(v) = v else { return Ok(None) };
        total = if *sign < 0 {
            num::sub(total, *v)?
        } else {
            num::add(total, *v)?
        };
    }
    Ok(Some(total))
}

/// The sum of the "of which" parts given, `None` when none is given (nothing to check).
fn given_parts(values: &[Option<Decimal>]) -> Result<Option<Decimal>> {
    let mut total = None;
    for v in values.iter().flatten() {
        total = Some(num::add(total.unwrap_or(Decimal::ZERO), *v)?);
    }
    Ok(total)
}

const FIX_TOTAL: &str = "correct the total or the amount that differs among its components, or \
    leave out the total to have the relation reported as not checked";
const FIX_PART: &str = "a part cannot exceed its aggregate: correct the part or the aggregate";

/// The relations of R7 on the statutory balance sheet (Appendix A.1), and the profit of A.IX
/// against item 21. `base` is the pointer of the balance sheet; `is_base` of the income
/// statement.
pub(crate) fn balance_sheet(
    c: &mut Checks,
    bs: &StatutoryBalanceSheet,
    is: &StatutoryIncomeStatement,
    base: &str,
    is_base: &str,
) -> Result<()> {
    let p = |k: &str| format!("{base}/{k}");
    for (key, node, name) in [
        ("intangible_assets", &bs.intangible_assets, "B.I"),
        ("tangible_assets", &bs.tangible_assets, "B.II"),
    ] {
        let n = node.clone().unwrap_or_default();
        let relation = if key == "intangible_assets" {
            "intangible_assets_net"
        } else {
            "tangible_assets_net"
        };
        c.equal(
            relation,
            Side::new(
                format!("{name} net"),
                vec![p(&format!("{key}/total"))],
                n.total,
            ),
            Side::new(
                format!("{name} gross − accumulated"),
                vec![p(&format!("{key}/gross")), p(&format!("{key}/accumulated"))],
                signed_sum(&[(1, n.gross), (-1, n.accumulated)])?,
            ),
            FIX_TOTAL,
        )?;
    }
    let b1 = net(&bs.intangible_assets)?;
    let b2 = net(&bs.tangible_assets)?;
    let b = sum_all(&[b1, b2, bs.financial_fixed_assets])?;
    c.equal(
        "fixed_assets_total",
        Side::new(
            "total B, fixed assets",
            vec![p("fixed_assets")],
            bs.fixed_assets,
        ),
        Side::new(
            "B.I + B.II + B.III",
            vec![
                p("intangible_assets"),
                p("tangible_assets"),
                p("financial_fixed_assets"),
            ],
            b,
        ),
        FIX_TOTAL,
    )?;
    let receivables = bs.receivables.clone().unwrap_or_default();
    let cc = sum_all(&[
        bs.inventories,
        receivables.total,
        bs.current_financial_assets,
        bs.cash,
    ])?;
    c.equal(
        "current_assets_total",
        Side::new(
            "total C, current assets",
            vec![p("current_assets")],
            bs.current_assets,
        ),
        Side::new(
            "C.I + C.II + C.III + C.IV",
            vec![
                p("inventories"),
                p("receivables/total"),
                p("current_financial_assets"),
                p("cash"),
            ],
            cc,
        ),
        FIX_TOTAL,
    )?;
    let eq = bs.equity.clone().unwrap_or_default();
    let eq_parts = [
        eq.share_capital,
        eq.share_premium,
        eq.revaluation_reserves,
        eq.legal_reserve,
        eq.statutory_reserves,
        eq.other_reserves,
        eq.cash_flow_hedge_reserve,
        eq.retained_earnings,
        eq.profit_or_loss_for_the_year,
        eq.negative_reserve_for_own_shares,
    ];
    let eq_sum = sum_all(&eq_parts)?;
    c.equal(
        "equity_total",
        Side::new("total equity A", vec![p("equity/total")], eq.total),
        Side::new("A.I + … + A.X", vec![p("equity")], eq_sum),
        FIX_TOTAL,
    )?;
    let payables = bs.payables.clone().unwrap_or_default();
    let a_sep = separate(
        bs.subscribed_capital_unpaid,
        receivables.subscribed_capital_unpaid,
    );
    let d_sep = separate(
        bs.prepayments_and_accrued_income
            .as_ref()
            .and_then(|n| n.total),
        receivables.prepayments_and_accrued_income,
    );
    let d_sep = given_node_or(&bs.prepayments_and_accrued_income, d_sep);
    let e_sep = given_node_or(
        &bs.accruals_and_deferred_income,
        separate(
            bs.accruals_and_deferred_income
                .as_ref()
                .and_then(|n| n.total),
            payables.accruals_and_deferred_income,
        ),
    );
    let b_tot = if bs.fixed_assets.is_some() {
        bs.fixed_assets
    } else {
        b
    };
    let c_tot = if bs.current_assets.is_some() {
        bs.current_assets
    } else {
        cc
    };
    let assets = sum_all(&[a_sep, b_tot, c_tot, d_sep])?;
    c.equal(
        "total_assets_total",
        Side::new("total assets", vec![p("total_assets")], bs.total_assets),
        Side::new(
            "A + B + C + D",
            vec![
                p("subscribed_capital_unpaid"),
                p("fixed_assets"),
                p("current_assets"),
                p("prepayments_and_accrued_income"),
            ],
            assets,
        ),
        FIX_TOTAL,
    )?;
    let equity = equity_amount(&bs.equity)?;
    let liabilities = sum_all(&[
        equity,
        bs.provisions.as_ref().and_then(|n| n.total),
        bs.employee_severance_indemnity
            .as_ref()
            .and_then(|n| n.total),
        payables.total,
        e_sep,
    ])?;
    c.equal(
        "total_liabilities_and_equity_total",
        Side::new(
            "total liabilities and equity",
            vec![p("total_liabilities_and_equity")],
            bs.total_liabilities_and_equity,
        ),
        Side::new(
            "liabilities A + B + C + D + E",
            vec![
                p("equity"),
                p("provisions/total"),
                p("employee_severance_indemnity/total"),
                p("payables/total"),
                p("accruals_and_deferred_income"),
            ],
            liabilities,
        ),
        FIX_TOTAL,
    )?;
    let ta = if bs.total_assets.is_some() {
        bs.total_assets
    } else {
        assets
    };
    let tl = if bs.total_liabilities_and_equity.is_some() {
        bs.total_liabilities_and_equity
    } else {
        liabilities
    };
    c.equal(
        "total_assets_equals_total_liabilities_and_equity",
        Side::new("total assets", vec![p("total_assets")], ta),
        Side::new(
            "total liabilities and equity",
            vec![p("total_liabilities_and_equity")],
            tl,
        ),
        "the balance sheet does not balance: correct the item that differs from the filing; \
         for statements in euro units, give the rounding reserve within other reserves (A.VI)",
    )?;
    c.at_most(
        "receivables_beyond_one_year",
        Side::new(
            "receivables beyond one year",
            vec![p("receivables/beyond_one_year")],
            receivables.beyond_one_year,
        ),
        Side::new(
            "receivables C.II",
            vec![p("receivables/total")],
            receivables.total,
        ),
        FIX_PART,
    )?;
    c.at_most(
        "receivables_of_which",
        Side::new(
            "the \"of which\" parts of receivables",
            vec![
                p("receivables/trade"),
                p("receivables/subscribed_capital_unpaid"),
                p("receivables/prepayments_and_accrued_income"),
            ],
            given_parts(&[
                receivables.trade,
                receivables.subscribed_capital_unpaid,
                receivables.prepayments_and_accrued_income,
            ])?,
        ),
        Side::new(
            "receivables C.II",
            vec![p("receivables/total")],
            receivables.total,
        ),
        FIX_PART,
    )?;
    c.at_most(
        "payables_beyond_one_year",
        Side::new(
            "payables beyond one year",
            vec![p("payables/beyond_one_year")],
            payables.beyond_one_year,
        ),
        Side::new("payables D", vec![p("payables/total")], payables.total),
        FIX_PART,
    )?;
    let fd = payables.financial_debt.clone().unwrap_or_default();
    let sl = payables.shareholder_loans.clone().unwrap_or_default();
    c.at_most(
        "payables_of_which",
        Side::new(
            "the \"of which\" parts of payables",
            vec![
                p("payables/financial_debt/total"),
                p("payables/shareholder_loans/total"),
                p("payables/trade"),
                p("payables/accruals_and_deferred_income"),
            ],
            given_parts(&[
                fd.total,
                sl.total,
                payables.trade,
                payables.accruals_and_deferred_income,
            ])?,
        ),
        Side::new("payables D", vec![p("payables/total")], payables.total),
        FIX_PART,
    )?;
    for (relation, key, node) in [
        (
            "financial_debt_beyond_one_year",
            "payables/financial_debt",
            &fd,
        ),
        (
            "shareholder_loans_beyond_one_year",
            "payables/shareholder_loans",
            &sl,
        ),
    ] {
        c.at_most(
            relation,
            Side::new(
                format!("{key} beyond one year"),
                vec![p(&format!("{key}/beyond_one_year"))],
                node.beyond_one_year,
            ),
            Side::new(key.to_owned(), vec![p(&format!("{key}/total"))], node.total),
            FIX_PART,
        )?;
    }
    let prov = bs.provisions.clone().unwrap_or_default();
    c.at_most(
        "provisions_beyond_one_year",
        Side::new(
            "provisions beyond one year",
            vec![p("provisions/beyond_one_year")],
            prov.beyond_one_year,
        ),
        Side::new("provisions B", vec![p("provisions/total")], prov.total),
        FIX_PART,
    )?;
    c.at_most(
        "provisions_of_which",
        Side::new(
            "derivative liabilities within provisions",
            vec![p("provisions/of_which_derivative_liabilities")],
            prov.of_which_derivative_liabilities,
        ),
        Side::new("provisions B", vec![p("provisions/total")], prov.total),
        FIX_PART,
    )?;
    for (relation, key, node) in [
        (
            "employee_severance_indemnity_beyond_one_year",
            "employee_severance_indemnity",
            &bs.employee_severance_indemnity,
        ),
        (
            "prepayments_and_accrued_income_beyond_one_year",
            "prepayments_and_accrued_income",
            &bs.prepayments_and_accrued_income,
        ),
        (
            "accruals_and_deferred_income_beyond_one_year",
            "accruals_and_deferred_income",
            &bs.accruals_and_deferred_income,
        ),
    ] {
        let n = node.clone().unwrap_or_default();
        c.at_most(
            relation,
            Side::new(
                format!("{key} beyond one year"),
                vec![p(&format!("{key}/beyond_one_year"))],
                n.beyond_one_year,
            ),
            Side::new(key.to_owned(), vec![p(&format!("{key}/total"))], n.total),
            FIX_PART,
        )?;
    }
    c.equal(
        "profit_for_the_year",
        Side::new(
            "profit (loss) for the year in equity, A.IX",
            vec![p("equity/profit_or_loss_for_the_year")],
            eq.profit_or_loss_for_the_year,
        ),
        Side::new(
            "item 21 of the income statement",
            vec![format!("{is_base}/profit_or_loss_for_the_year")],
            is.profit_or_loss_for_the_year,
        ),
        "the profit of equity A.IX and item 21 of the income statement are the same amount: \
         correct the one that differs from the filing",
    )
}

/// The relations of R7 on the statutory income statement (Appendix A.2).
pub(crate) fn income_statement(
    c: &mut Checks,
    is: &StatutoryIncomeStatement,
    base: &str,
) -> Result<()> {
    let p = |k: &str| format!("{base}/{k}");
    let a_parts = [
        is.revenue,
        is.change_in_product_inventories,
        is.own_work_capitalised,
        is.other_revenue_and_income,
    ];
    let a_der = sum_all(&a_parts)?;
    c.equal(
        "value_of_production_total",
        Side::new(
            "total A",
            vec![p("value_of_production")],
            is.value_of_production,
        ),
        Side::new(
            "A1 + A2 and A3 + A4 + A5",
            vec![
                p("revenue"),
                p("change_in_product_inventories"),
                p("own_work_capitalised"),
                p("other_revenue_and_income"),
            ],
            a_der,
        ),
        FIX_TOTAL,
    )?;
    let b10 = is.depreciation_and_write_downs.clone().unwrap_or_default();
    let b10p = |k: &str| p(&format!("depreciation_and_write_downs/{k}"));
    let ab_c = sum_all(&[
        b10.depreciation_and_amortisation,
        b10.write_downs_of_fixed_assets,
    ])?;
    c.equal(
        "value_adjustments_of_fixed_assets_total",
        Side::new(
            "B10 a to c",
            vec![b10p("value_adjustments_of_fixed_assets")],
            b10.value_adjustments_of_fixed_assets,
        ),
        Side::new(
            "B10 a and b + B10 c",
            vec![
                b10p("depreciation_and_amortisation"),
                b10p("write_downs_of_fixed_assets"),
            ],
            ab_c,
        ),
        FIX_TOTAL,
    )?;
    let ac = b10.value_adjustments_of_fixed_assets.or(ab_c);
    let b10_der = sum_all(&[ac, b10.write_downs_of_receivables])?;
    c.equal(
        "depreciation_and_write_downs_total",
        Side::new("total B10", vec![b10p("total")], b10.total),
        Side::new(
            "B10 a to c + B10 d",
            vec![
                b10p("value_adjustments_of_fixed_assets"),
                b10p("write_downs_of_receivables"),
            ],
            b10_der,
        ),
        FIX_TOTAL,
    )?;
    let b10_amt = if b10.total.is_some() {
        b10.total
    } else {
        b10_der
    };
    let b_der = sum_all(&[
        is.purchases,
        is.services,
        is.use_of_third_party_assets,
        is.personnel_costs,
        b10_amt,
        is.change_in_raw_material_inventories,
        is.provisions_for_risks,
        is.other_provisions,
        is.other_operating_expenses,
    ])?;
    c.equal(
        "costs_of_production_total",
        Side::new(
            "total B",
            vec![p("costs_of_production")],
            is.costs_of_production,
        ),
        Side::new("B6 + … + B14", vec![base.to_owned()], b_der),
        FIX_TOTAL,
    )?;
    let a_amt = is.value_of_production.or(a_der);
    let b_amt = is.costs_of_production.or(b_der);
    let diff = signed_sum(&[(1, a_amt), (-1, b_amt)])?;
    c.equal(
        "difference_between_value_and_costs_of_production_total",
        Side::new(
            "A − B",
            vec![p("difference_between_value_and_costs_of_production")],
            is.difference_between_value_and_costs_of_production,
        ),
        Side::new(
            "total A − total B",
            vec![p("value_of_production"), p("costs_of_production")],
            diff,
        ),
        FIX_TOTAL,
    )?;
    let c_der = signed_sum(&[
        (1, is.income_from_equity_investments),
        (1, is.other_financial_income),
        (-1, is.interest_and_other_financial_charges),
        (1, is.exchange_gains_and_losses),
    ])?;
    c.equal(
        "financial_income_and_charges_total",
        Side::new(
            "total C",
            vec![p("financial_income_and_charges")],
            is.financial_income_and_charges,
        ),
        Side::new(
            "C15 + C16 − C17 ± C17-bis",
            vec![
                p("income_from_equity_investments"),
                p("other_financial_income"),
                p("interest_and_other_financial_charges"),
                p("exchange_gains_and_losses"),
            ],
            c_der,
        ),
        FIX_TOTAL,
    )?;
    let d_der = signed_sum(&[
        (1, is.revaluations_of_financial_assets),
        (-1, is.write_downs_of_financial_assets),
    ])?;
    c.equal(
        "value_adjustments_of_financial_assets_total",
        Side::new(
            "total D",
            vec![p("value_adjustments_of_financial_assets")],
            is.value_adjustments_of_financial_assets,
        ),
        Side::new(
            "D18 − D19",
            vec![
                p("revaluations_of_financial_assets"),
                p("write_downs_of_financial_assets"),
            ],
            d_der,
        ),
        FIX_TOTAL,
    )?;
    let diff_amt = is.difference_between_value_and_costs_of_production.or(diff);
    let c_amt = is.financial_income_and_charges.or(c_der);
    let d_amt = is.value_adjustments_of_financial_assets.or(d_der);
    let pbt_der = sum_all(&[diff_amt, c_amt, d_amt])?;
    c.equal(
        "profit_before_tax_total",
        Side::new(
            "profit before tax",
            vec![p("profit_before_tax")],
            is.profit_before_tax,
        ),
        Side::new(
            "A − B ± C ± D",
            vec![
                p("difference_between_value_and_costs_of_production"),
                p("financial_income_and_charges"),
                p("value_adjustments_of_financial_assets"),
            ],
            pbt_der,
        ),
        FIX_TOTAL,
    )?;
    let pbt = is.profit_before_tax.or(pbt_der);
    c.equal(
        "profit_or_loss_for_the_year_total",
        Side::new(
            "item 21",
            vec![p("profit_or_loss_for_the_year")],
            is.profit_or_loss_for_the_year,
        ),
        Side::new(
            "profit before tax − item 20",
            vec![p("profit_before_tax"), p("income_taxes")],
            signed_sum(&[(1, pbt), (-1, is.income_taxes)])?,
        ),
        FIX_TOTAL,
    )
}

/// B.I or B.II net: the total, or gross − accumulated.
fn net(node: &Option<super::input::NetFixedAsset>) -> Result<Option<Decimal>> {
    let Some(n) = node else { return Ok(None) };
    if n.total.is_some() {
        return Ok(n.total);
    }
    signed_sum(&[(1, n.gross), (-1, n.accumulated)])
}

/// Equity: the total, or the sum of the ten Roman numerals.
fn equity_amount(node: &Option<super::input::Equity>) -> Result<Option<Decimal>> {
    let Some(e) = node else { return Ok(None) };
    if e.total.is_some() {
        return Ok(e.total);
    }
    sum_all(&[
        e.share_capital,
        e.share_premium,
        e.revaluation_reserves,
        e.legal_reserve,
        e.statutory_reserves,
        e.other_reserves,
        e.cash_flow_hedge_reserve,
        e.retained_earnings,
        e.profit_or_loss_for_the_year,
        e.negative_reserve_for_own_shares,
    ])
}

/// Item A, assets D or liabilities E as a separate item: zero when presented within C.II or D
/// (its "of which" part given there) and not given separately.
fn separate(given: Option<Decimal>, within: Option<Decimal>) -> Option<Decimal> {
    match (given, within) {
        (Some(v), _) => Some(v),
        (None, Some(_)) => Some(Decimal::ZERO),
        (None, None) => None,
    }
}

/// A node given without its total stays missing, even when an "of which" part is given within
/// another aggregate.
fn given_node_or(
    node: &Option<super::input::WithMaturity>,
    fallback: Option<Decimal>,
) -> Option<Decimal> {
    match node {
        Some(n) => n.total,
        None => fallback,
    }
}
