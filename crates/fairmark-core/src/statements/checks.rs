//! The relations of R7 and the identity of R12: each is checked exactly, with no tolerance,
//! when both sides are given (`reconciled`), reported as `not_checked` otherwise, and refused
//! with `FM-E026` when the two sides differ (Decision 2).

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::Serialize;

use super::input::{StatutoryBalanceSheet, StatutoryIncomeStatement};
use crate::error::{FairmarkError, Reconciliation, Result};
use crate::num;

/// Outcome of one relation. A relation whose sides differ is not an outcome: it is `FM-E026`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    /// Both sides given and equal.
    Reconciled,
    /// A side is missing: the relation could not be checked.
    NotChecked,
}

/// One relation of R7 or R12 in the result.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Check {
    /// `reconciled` or `not_checked`.
    pub status: CheckStatus,
    /// JSON pointers of the inputs on both sides of the relation: for a relation checked, the
    /// items read. Empty for `funds_invested_equals_sources` of a statutory year, whose sides
    /// are computed outputs; its status counts the items taken as zero (D1, M2).
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

/// An amount read from items the user gave, with the pointers of those items. A side of a
/// relation is built only from items given: never from a default or from a line inferred from
/// another one (review of PR 1), so `FM-E026` always names amounts the user wrote.
#[derive(Clone)]
struct Read {
    value: Option<Decimal>,
    /// The pointers of the items read, or of the items needed when one is missing.
    pointers: Vec<String>,
}

impl Read {
    fn item(pointer: String, value: Option<Decimal>) -> Self {
        Self {
            value,
            pointers: vec![pointer],
        }
    }

    /// The signed sum of `terms`; missing when a term is missing.
    fn sum(terms: &[(i8, &Read)]) -> Result<Self> {
        let values: Vec<(i8, Option<Decimal>)> = terms.iter().map(|(s, r)| (*s, r.value)).collect();
        Ok(Self {
            value: signed_sum(&values)?,
            pointers: terms.iter().flat_map(|(_, r)| r.pointers.clone()).collect(),
        })
    }

    /// `self` when given, else `derived` (a total given prevails over its components).
    fn or(self, derived: Read) -> Self {
        if self.value.is_some() {
            self
        } else {
            derived
        }
    }

    fn side(&self, name: impl Into<String>) -> Side {
        Side::new(name, self.pointers.clone(), self.value)
    }
}

/// The sum of the "of which" parts given, with their pointers; `None` when none is given
/// (nothing to check).
fn given_parts(parts: &[(String, Option<Decimal>)]) -> Result<Read> {
    let mut total = None;
    let mut pointers = Vec::new();
    for (pointer, v) in parts {
        if let Some(v) = v {
            total = Some(num::add(total.unwrap_or(Decimal::ZERO), *v)?);
            pointers.push(pointer.clone());
        }
    }
    if pointers.is_empty() {
        pointers = parts.iter().map(|(p, _)| p.clone()).collect();
    }
    Ok(Read {
        value: total,
        pointers,
    })
}

const FIX_TOTAL: &str = "correct the total or the amount that differs among its components, or \
    leave out the total to have the relation reported as not checked";
const FIX_PART: &str = "a part cannot exceed its aggregate: correct the part or the aggregate";

/// The relations of R7 on the statutory balance sheet (Appendix A.1), and the profit of A.IX
/// against item 21. `base` is the pointer of the balance sheet; `is_base` of the income
/// statement. Every side is read from items given only; a relation with a side that is not
/// fully given is `not_checked`.
pub(crate) fn balance_sheet(
    c: &mut Checks,
    bs: &StatutoryBalanceSheet,
    is: &StatutoryIncomeStatement,
    base: &str,
    is_base: &str,
) -> Result<()> {
    let p = |k: &str| format!("{base}/{k}");
    let item = |k: &str, v: Option<Decimal>| Read::item(p(k), v);
    let mut net_of = |key: &str, node: &Option<super::input::NetFixedAsset>, name: &str| {
        let n = node.clone().unwrap_or_default();
        let total = item(&format!("{key}/total"), n.total);
        let derived = Read::sum(&[
            (1, &item(&format!("{key}/gross"), n.gross)),
            (-1, &item(&format!("{key}/accumulated"), n.accumulated)),
        ])?;
        let relation = if key == "intangible_assets" {
            "intangible_assets_net"
        } else {
            "tangible_assets_net"
        };
        c.equal(
            relation,
            total.side(format!("{name} net")),
            derived.side(format!("{name} gross − accumulated")),
            FIX_TOTAL,
        )?;
        Ok::<Read, FairmarkError>(total.or(derived))
    };
    let b1 = net_of("intangible_assets", &bs.intangible_assets, "B.I")?;
    let b2 = net_of("tangible_assets", &bs.tangible_assets, "B.II")?;
    let b3 = item("financial_fixed_assets", bs.financial_fixed_assets);
    let b = Read::sum(&[(1, &b1), (1, &b2), (1, &b3)])?;
    let b_given = item("fixed_assets", bs.fixed_assets);
    c.equal(
        "fixed_assets_total",
        b_given.side("total B, fixed assets"),
        b.side("B.I + B.II + B.III"),
        FIX_TOTAL,
    )?;
    let receivables = bs.receivables.clone().unwrap_or_default();
    let cc = Read::sum(&[
        (1, &item("inventories", bs.inventories)),
        (1, &item("receivables/total", receivables.total)),
        (
            1,
            &item("current_financial_assets", bs.current_financial_assets),
        ),
        (1, &item("cash", bs.cash)),
    ])?;
    let c_given = item("current_assets", bs.current_assets);
    c.equal(
        "current_assets_total",
        c_given.side("total C, current assets"),
        cc.side("C.I + C.II + C.III + C.IV"),
        FIX_TOTAL,
    )?;
    let eq = bs.equity.clone().unwrap_or_default();
    let eq_total = item("equity/total", eq.total);
    let eq_parts = Read {
        value: sum_all(&equity_parts(&eq))?,
        pointers: EQUITY_PARTS
            .iter()
            .map(|k| p(&format!("equity/{k}")))
            .collect(),
    };
    c.equal(
        "equity_total",
        eq_total.side("total equity A"),
        eq_parts.side("A.I + … + A.X"),
        FIX_TOTAL,
    )?;
    // Item A as given only: taken as zero by default (D1) it never enters a check. Assets D
    // and liabilities E as separate lines: as given, or zero when their part is presented
    // within C.II or D, which the user gave (M2): that part is inside C or D already.
    let a = item("subscribed_capital_unpaid", bs.subscribed_capital_unpaid);
    let within = |separate: Option<Decimal>, key: &str, within: Option<Decimal>, at: &str| match (
        separate, within,
    ) {
        (None, Some(_)) => Read::item(p(at), Some(Decimal::ZERO)),
        _ => item(key, separate),
    };
    let d = within(
        bs.prepayments_and_accrued_income
            .as_ref()
            .and_then(|n| n.total),
        "prepayments_and_accrued_income/total",
        bs.receivables
            .as_ref()
            .and_then(|n| n.prepayments_and_accrued_income),
        "receivables/prepayments_and_accrued_income",
    );
    let e = within(
        bs.accruals_and_deferred_income
            .as_ref()
            .and_then(|n| n.total),
        "accruals_and_deferred_income/total",
        bs.payables
            .as_ref()
            .and_then(|n| n.accruals_and_deferred_income),
        "payables/accruals_and_deferred_income",
    );
    let assets = Read::sum(&[(1, &a), (1, &b_given.or(b)), (1, &c_given.or(cc)), (1, &d)])?;
    let ta_given = item("total_assets", bs.total_assets);
    c.equal(
        "total_assets_total",
        ta_given.side("total assets"),
        assets.side("A + B + C + D"),
        FIX_TOTAL,
    )?;
    let payables = bs.payables.clone().unwrap_or_default();
    let liabilities = Read::sum(&[
        (1, &eq_total.or(eq_parts)),
        (
            1,
            &item(
                "provisions/total",
                bs.provisions.as_ref().and_then(|n| n.total),
            ),
        ),
        (
            1,
            &item(
                "employee_severance_indemnity/total",
                bs.employee_severance_indemnity
                    .as_ref()
                    .and_then(|n| n.total),
            ),
        ),
        (1, &item("payables/total", payables.total)),
        (1, &e),
    ])?;
    let tl_given = item(
        "total_liabilities_and_equity",
        bs.total_liabilities_and_equity,
    );
    c.equal(
        "total_liabilities_and_equity_total",
        tl_given.side("total liabilities and equity"),
        liabilities.side("liabilities A + B + C + D + E"),
        FIX_TOTAL,
    )?;
    c.equal(
        "total_assets_equals_total_liabilities_and_equity",
        ta_given.or(assets).side("total assets"),
        tl_given
            .or(liabilities)
            .side("total liabilities and equity"),
        "the balance sheet does not balance: correct the item that differs from the filing; \
         for statements in euro units, give the rounding reserve within other reserves (A.VI)",
    )?;
    let rec_total = item("receivables/total", receivables.total);
    c.at_most(
        "receivables_beyond_one_year",
        item("receivables/beyond_one_year", receivables.beyond_one_year)
            .side("receivables beyond one year"),
        rec_total.side("receivables C.II"),
        FIX_PART,
    )?;
    c.at_most(
        "receivables_of_which",
        given_parts(&[
            (p("receivables/trade"), receivables.trade),
            (
                p("receivables/subscribed_capital_unpaid"),
                receivables.subscribed_capital_unpaid,
            ),
            (
                p("receivables/prepayments_and_accrued_income"),
                receivables.prepayments_and_accrued_income,
            ),
        ])?
        .side("the \"of which\" parts of receivables"),
        rec_total.side("receivables C.II"),
        FIX_PART,
    )?;
    let pay_total = item("payables/total", payables.total);
    c.at_most(
        "payables_beyond_one_year",
        item("payables/beyond_one_year", payables.beyond_one_year).side("payables beyond one year"),
        pay_total.side("payables D"),
        FIX_PART,
    )?;
    let fd = payables.financial_debt.clone().unwrap_or_default();
    let sl = payables.shareholder_loans.clone().unwrap_or_default();
    c.at_most(
        "payables_of_which",
        given_parts(&[
            (p("payables/financial_debt/total"), fd.total),
            (p("payables/shareholder_loans/total"), sl.total),
            (p("payables/trade"), payables.trade),
            (
                p("payables/accruals_and_deferred_income"),
                payables.accruals_and_deferred_income,
            ),
        ])?
        .side("the \"of which\" parts of payables"),
        pay_total.side("payables D"),
        FIX_PART,
    )?;
    let prov = bs.provisions.clone().unwrap_or_default();
    let maturities = [
        (
            "financial_debt_beyond_one_year",
            "payables/financial_debt",
            Some(fd.clone()),
        ),
        (
            "shareholder_loans_beyond_one_year",
            "payables/shareholder_loans",
            Some(sl.clone()),
        ),
        (
            "provisions_beyond_one_year",
            "provisions",
            Some(super::input::WithMaturity {
                total: prov.total,
                beyond_one_year: prov.beyond_one_year,
            }),
        ),
        (
            "employee_severance_indemnity_beyond_one_year",
            "employee_severance_indemnity",
            bs.employee_severance_indemnity.clone(),
        ),
        (
            "prepayments_and_accrued_income_beyond_one_year",
            "prepayments_and_accrued_income",
            bs.prepayments_and_accrued_income.clone(),
        ),
        (
            "accruals_and_deferred_income_beyond_one_year",
            "accruals_and_deferred_income",
            bs.accruals_and_deferred_income.clone(),
        ),
    ];
    for (relation, key, node) in maturities {
        let n = node.unwrap_or_default();
        c.at_most(
            relation,
            item(&format!("{key}/beyond_one_year"), n.beyond_one_year)
                .side(format!("{key} beyond one year")),
            item(&format!("{key}/total"), n.total).side(key.to_owned()),
            FIX_PART,
        )?;
    }
    c.at_most(
        "provisions_of_which",
        item(
            "provisions/of_which_derivative_liabilities",
            prov.of_which_derivative_liabilities,
        )
        .side("derivative liabilities within provisions"),
        item("provisions/total", prov.total).side("provisions B"),
        FIX_PART,
    )?;
    c.equal(
        "profit_for_the_year",
        item(
            "equity/profit_or_loss_for_the_year",
            eq.profit_or_loss_for_the_year,
        )
        .side("profit (loss) for the year in equity, A.IX"),
        Read::item(
            format!("{is_base}/profit_or_loss_for_the_year"),
            is.profit_or_loss_for_the_year,
        )
        .side("item 21 of the income statement"),
        "the profit of equity A.IX and item 21 of the income statement are the same amount: \
         correct the one that differs from the filing",
    )
}

/// The keys of the Roman numerals of equity, in the order of art. 2424.
const EQUITY_PARTS: [&str; 10] = [
    "share_capital",
    "share_premium",
    "revaluation_reserves",
    "legal_reserve",
    "statutory_reserves",
    "other_reserves",
    "cash_flow_hedge_reserve",
    "retained_earnings",
    "profit_or_loss_for_the_year",
    "negative_reserve_for_own_shares",
];

fn equity_parts(e: &super::input::Equity) -> [Option<Decimal>; 10] {
    [
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
    ]
}

/// The relations of R7 on the statutory income statement (Appendix A.2), each side read from
/// items given only.
pub(crate) fn income_statement(
    c: &mut Checks,
    is: &StatutoryIncomeStatement,
    base: &str,
) -> Result<()> {
    let p = |k: &str| format!("{base}/{k}");
    let item = |k: &str, v: Option<Decimal>| Read::item(p(k), v);
    let a_der = Read::sum(&[
        (1, &item("revenue", is.revenue)),
        (
            1,
            &item(
                "change_in_product_inventories",
                is.change_in_product_inventories,
            ),
        ),
        (1, &item("own_work_capitalised", is.own_work_capitalised)),
        (
            1,
            &item("other_revenue_and_income", is.other_revenue_and_income),
        ),
    ])?;
    let a_given = item("value_of_production", is.value_of_production);
    c.equal(
        "value_of_production_total",
        a_given.side("total A"),
        a_der.side("A1 + A2 and A3 + A4 + A5"),
        FIX_TOTAL,
    )?;
    let b10 = is.depreciation_and_write_downs.clone().unwrap_or_default();
    let b10i = |k: &str, v: Option<Decimal>| item(&format!("depreciation_and_write_downs/{k}"), v);
    let ab_c = Read::sum(&[
        (
            1,
            &b10i(
                "depreciation_and_amortisation",
                b10.depreciation_and_amortisation,
            ),
        ),
        (
            1,
            &b10i(
                "write_downs_of_fixed_assets",
                b10.write_downs_of_fixed_assets,
            ),
        ),
    ])?;
    let ac_given = b10i(
        "value_adjustments_of_fixed_assets",
        b10.value_adjustments_of_fixed_assets,
    );
    c.equal(
        "value_adjustments_of_fixed_assets_total",
        ac_given.side("B10 a to c"),
        ab_c.side("B10 a and b + B10 c"),
        FIX_TOTAL,
    )?;
    // B10 c is a part of the group B10 a to c (review of PR 2, blocking 1): the group less
    // B10 c is depreciation and amortisation, never negative.
    c.at_most(
        "value_adjustments_of_fixed_assets_of_which",
        b10i(
            "write_downs_of_fixed_assets",
            b10.write_downs_of_fixed_assets,
        )
        .side("B10 c"),
        ac_given.side("B10 a to c"),
        "B10 c is a part of B10 a to c: correct B10 c or the group",
    )?;
    let b10_der = Read::sum(&[
        (1, &ac_given.or(ab_c)),
        (
            1,
            &b10i("write_downs_of_receivables", b10.write_downs_of_receivables),
        ),
    ])?;
    let b10_given = b10i("total", b10.total);
    c.equal(
        "depreciation_and_write_downs_total",
        b10_given.side("total B10"),
        b10_der.side("B10 a to c + B10 d"),
        FIX_TOTAL,
    )?;
    let b_der = Read::sum(&[
        (1, &item("purchases", is.purchases)),
        (1, &item("services", is.services)),
        (
            1,
            &item("use_of_third_party_assets", is.use_of_third_party_assets),
        ),
        (1, &item("personnel_costs", is.personnel_costs)),
        (1, &b10_given.or(b10_der)),
        (
            1,
            &item(
                "change_in_raw_material_inventories",
                is.change_in_raw_material_inventories,
            ),
        ),
        (1, &item("provisions_for_risks", is.provisions_for_risks)),
        (1, &item("other_provisions", is.other_provisions)),
        (
            1,
            &item("other_operating_expenses", is.other_operating_expenses),
        ),
    ])?;
    let b_given = item("costs_of_production", is.costs_of_production);
    c.equal(
        "costs_of_production_total",
        b_given.side("total B"),
        b_der.side("B6 + … + B14"),
        FIX_TOTAL,
    )?;
    let diff = Read::sum(&[(1, &a_given.or(a_der)), (-1, &b_given.or(b_der))])?;
    let diff_given = item(
        "difference_between_value_and_costs_of_production",
        is.difference_between_value_and_costs_of_production,
    );
    c.equal(
        "difference_between_value_and_costs_of_production_total",
        diff_given.side("A − B"),
        diff.side("total A − total B"),
        FIX_TOTAL,
    )?;
    let c_der = Read::sum(&[
        (
            1,
            &item(
                "income_from_equity_investments",
                is.income_from_equity_investments,
            ),
        ),
        (
            1,
            &item("other_financial_income", is.other_financial_income),
        ),
        (
            -1,
            &item(
                "interest_and_other_financial_charges",
                is.interest_and_other_financial_charges,
            ),
        ),
        (
            1,
            &item("exchange_gains_and_losses", is.exchange_gains_and_losses),
        ),
    ])?;
    let c_given = item(
        "financial_income_and_charges",
        is.financial_income_and_charges,
    );
    c.equal(
        "financial_income_and_charges_total",
        c_given.side("total C"),
        c_der.side("C15 + C16 − C17 ± C17-bis"),
        FIX_TOTAL,
    )?;
    let d_der = Read::sum(&[
        (
            1,
            &item(
                "revaluations_of_financial_assets",
                is.revaluations_of_financial_assets,
            ),
        ),
        (
            -1,
            &item(
                "write_downs_of_financial_assets",
                is.write_downs_of_financial_assets,
            ),
        ),
    ])?;
    let d_given = item(
        "value_adjustments_of_financial_assets",
        is.value_adjustments_of_financial_assets,
    );
    c.equal(
        "value_adjustments_of_financial_assets_total",
        d_given.side("total D"),
        d_der.side("D18 − D19"),
        FIX_TOTAL,
    )?;
    let pbt_der = Read::sum(&[
        (1, &diff_given.or(diff)),
        (1, &c_given.or(c_der)),
        (1, &d_given.or(d_der)),
    ])?;
    let pbt_given = item("profit_before_tax", is.profit_before_tax);
    c.equal(
        "profit_before_tax_total",
        pbt_given.side("profit before tax"),
        pbt_der.side("A − B ± C ± D"),
        FIX_TOTAL,
    )?;
    let net = Read::sum(&[
        (1, &pbt_given.or(pbt_der)),
        (-1, &item("income_taxes", is.income_taxes)),
    ])?;
    c.equal(
        "profit_or_loss_for_the_year_total",
        item(
            "profit_or_loss_for_the_year",
            is.profit_or_loss_for_the_year,
        )
        .side("item 21"),
        net.side("profit before tax − item 20"),
        FIX_TOTAL,
    )
}
