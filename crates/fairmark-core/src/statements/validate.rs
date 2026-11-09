//! Validation before any computation (R2, R3, R5, R9, R10, R24): forms, dates, signs,
//! reclassified lines, overrides and adjustments. Every refusal is `FM-E001` naming the field
//! by its JSON pointer and stating the fix. Unknown fields, unknown destinations and an
//! override of another aggregate are refused while the input is read (`deny_unknown_fields`).

use rust_decimal::Decimal;

use super::input::{
    Adjustment, Equity, FinancialFixedAssetsOverride, LeaseAmounts, NetFixedAsset, Payables,
    Provisions, Receivables, ReclassifiedBalanceSheet, ReclassifiedIncomeStatement,
    ReorganiseInput, ShareholderLoansDestination, ShareholderLoansOverride, StatutoryBalanceSheet,
    StatutoryIncomeStatement, WithMaturity,
};
use crate::dates::{days_in_month, Date};
use crate::error::{FairmarkError, Reconciliation, Result};

fn invalid(message: String) -> FairmarkError {
    FairmarkError::InvalidInput(message)
}

/// Every check of this module, in a fixed order, so that the first refusal is deterministic.
pub(crate) fn validate(input: &ReorganiseInput) -> Result<()> {
    if input.years.is_empty() {
        return Err(invalid(
            "`/years` is empty: give at least one financial year, oldest first".into(),
        ));
    }
    for (i, y) in input.years.iter().enumerate() {
        if y.statutory.is_some() && y.reclassified.is_some() {
            return Err(invalid(format!(
                "`/years/{i}` has both `statutory` and `reclassified`: give one form per year; \
                 to compare the two forms of the same year, make two calls"
            )));
        }
    }
    dates(input)?;
    for (i, y) in input.years.iter().enumerate() {
        if let Some(st) = &y.statutory {
            if let Some(bs) = &st.balance_sheet {
                statutory_balance_sheet(bs, &format!("/years/{i}/statutory/balance_sheet"))?;
            }
            if let Some(is) = &st.income_statement {
                statutory_income_statement(is, &format!("/years/{i}/statutory/income_statement"))?;
            }
        }
        if let Some(rc) = &y.reclassified {
            if let Some(bs) = &rc.balance_sheet {
                reclassified_balance_sheet(bs, &format!("/years/{i}/reclassified/balance_sheet"))?;
            }
            if let Some(is) = &rc.income_statement {
                reclassified_income_statement(
                    is,
                    &format!("/years/{i}/reclassified/income_statement"),
                )?;
            }
            if y.treatment_overrides.is_some() {
                return Err(invalid(format!(
                    "`/years/{i}/treatment_overrides` is given for a reclassified entry: \
                     treatments apply to statutory entries only (R5); give the totals already \
                     reclassified, or remove the overrides"
                )));
            }
        }
        if let Some(o) = &y.treatment_overrides {
            let at = format!("/years/{i}/treatment_overrides");
            if let Some(FinancialFixedAssetsOverride::Split(parts)) = &o.financial_fixed_assets {
                for (j, p) in parts.iter().enumerate() {
                    let pp = format!("{at}/financial_fixed_assets/split/{j}");
                    unsigned(&format!("{pp}/amount"), Some(p.amount))?;
                    not_blank(&format!("{pp}/label"), &p.label, "name the part")?;
                }
            }
            if let Some(ShareholderLoansOverride::Split(parts)) = &o.shareholder_loans {
                for (j, p) in parts.iter().enumerate() {
                    let pp = format!("{at}/shareholder_loans/split/{j}");
                    unsigned(&format!("{pp}/amount"), Some(p.amount))?;
                    unsigned(&format!("{pp}/beyond_one_year"), p.beyond_one_year)?;
                    if p.beyond_one_year.is_some()
                        && p.destination == ShareholderLoansDestination::QuasiEquity
                    {
                        return Err(invalid(format!(
                            "`{pp}/beyond_one_year` is given for a `quasi_equity` part: a \
                             quasi-equity part is not in the net financial position and has no \
                             maturity there; remove the field, or give the part the destination \
                             `financial_debt`"
                        )));
                    }
                    not_blank(&format!("{pp}/label"), &p.label, "name the part")?;
                }
            }
        }
    }
    for (k, adj) in input.adjustments.iter().enumerate() {
        adjustment(input, k, adj)?;
    }
    Ok(())
}

/// R2: oldest first, one financial year apart when consecutive entries both have a date.
fn dates(input: &ReorganiseInput) -> Result<()> {
    for i in 1..input.years.len() {
        let (Some(a), Some(b)) = (input.years[i - 1].date, input.years[i].date) else {
            continue;
        };
        if !consecutive(a, b) {
            let order = if b <= a {
                "the entries are not oldest first"
            } else {
                "they are not one financial year apart"
            };
            return Err(invalid(format!(
                "`/years/{i}/date` is {b} and `/years/{}/date` is {a}: {order}; list the years \
                 oldest first, one financial year apart (same month and day one calendar year \
                 later, or both the last day of their month); reorganise a year of a different \
                 length in a separate call",
                i - 1
            )));
        }
    }
    Ok(())
}

/// `b` is one financial year after `a`: the same month one calendar year later, and the same
/// day, or 28 February after 29 February, or both the last day of their month (the
/// end-of-month rule of SPEC-0010).
pub(crate) fn consecutive(a: Date, b: Date) -> bool {
    if b.year() != a.year() + 1 || b.month() != a.month() {
        return false;
    }
    if b.day() == a.day() {
        return true;
    }
    let a_end = a.day() == days_in_month(a.year(), a.month());
    let b_end = b.day() == days_in_month(b.year(), b.month());
    a_end && b_end
}

const SIGN_FIX: &str = "amounts are entered as presented in the statutory layouts, zero or more \
    for this item (R3); give the amount without sign";

fn unsigned(at: &str, value: Option<Decimal>) -> Result<()> {
    match value {
        Some(v) if v.is_sign_negative() && !v.is_zero() => {
            Err(invalid(format!("`{at}` is {v}: {SIGN_FIX}")))
        }
        _ => Ok(()),
    }
}

fn not_blank(at: &str, text: &str, fix: &str) -> Result<()> {
    if text.trim().is_empty() {
        return Err(invalid(format!("`{at}` is blank: {fix}")));
    }
    Ok(())
}

fn net(node: &Option<NetFixedAsset>, at: &str) -> Result<()> {
    if let Some(n) = node {
        unsigned(&format!("{at}/total"), n.total)?;
        unsigned(&format!("{at}/gross"), n.gross)?;
        unsigned(&format!("{at}/accumulated"), n.accumulated)?;
    }
    Ok(())
}

fn maturity(node: &Option<WithMaturity>, at: &str) -> Result<()> {
    if let Some(n) = node {
        unsigned(&format!("{at}/total"), n.total)?;
        unsigned(&format!("{at}/beyond_one_year"), n.beyond_one_year)?;
    }
    Ok(())
}

fn statutory_balance_sheet(bs: &StatutoryBalanceSheet, at: &str) -> Result<()> {
    let p = |k: &str| format!("{at}/{k}");
    unsigned(
        &p("subscribed_capital_unpaid"),
        bs.subscribed_capital_unpaid,
    )?;
    net(&bs.intangible_assets, &p("intangible_assets"))?;
    net(&bs.tangible_assets, &p("tangible_assets"))?;
    unsigned(&p("financial_fixed_assets"), bs.financial_fixed_assets)?;
    unsigned(&p("fixed_assets"), bs.fixed_assets)?;
    unsigned(&p("inventories"), bs.inventories)?;
    if let Some(Receivables {
        total,
        beyond_one_year,
        trade,
        subscribed_capital_unpaid,
        prepayments_and_accrued_income,
    }) = &bs.receivables
    {
        for (k, v) in [
            ("total", total),
            ("beyond_one_year", beyond_one_year),
            ("trade", trade),
            ("subscribed_capital_unpaid", subscribed_capital_unpaid),
            (
                "prepayments_and_accrued_income",
                prepayments_and_accrued_income,
            ),
        ] {
            unsigned(&p(&format!("receivables/{k}")), *v)?;
        }
    }
    unsigned(&p("current_financial_assets"), bs.current_financial_assets)?;
    unsigned(&p("cash"), bs.cash)?;
    unsigned(&p("current_assets"), bs.current_assets)?;
    maturity(
        &bs.prepayments_and_accrued_income,
        &p("prepayments_and_accrued_income"),
    )?;
    unsigned(&p("total_assets"), bs.total_assets)?;
    if let Some(Equity {
        share_capital,
        share_premium,
        revaluation_reserves,
        legal_reserve,
        statutory_reserves,
        negative_reserve_for_own_shares,
        ..
    }) = &bs.equity
    {
        for (k, v) in [
            ("share_capital", share_capital),
            ("share_premium", share_premium),
            ("revaluation_reserves", revaluation_reserves),
            ("legal_reserve", legal_reserve),
            ("statutory_reserves", statutory_reserves),
        ] {
            unsigned(&p(&format!("equity/{k}")), *v)?;
        }
        if let Some(v) = negative_reserve_for_own_shares {
            if v.is_sign_positive() && !v.is_zero() {
                return Err(invalid(format!(
                    "`{}` is {v}: the negative reserve for own shares (A.X) is zero or \
                     negative, as presented in the statutory layout (R3); give it with its \
                     minus sign",
                    p("equity/negative_reserve_for_own_shares")
                )));
            }
        }
    }
    if let Some(Provisions {
        total,
        beyond_one_year,
        of_which_derivative_liabilities,
    }) = &bs.provisions
    {
        unsigned(&p("provisions/total"), *total)?;
        unsigned(&p("provisions/beyond_one_year"), *beyond_one_year)?;
        unsigned(
            &p("provisions/of_which_derivative_liabilities"),
            *of_which_derivative_liabilities,
        )?;
    }
    maturity(
        &bs.employee_severance_indemnity,
        &p("employee_severance_indemnity"),
    )?;
    if let Some(Payables {
        total,
        beyond_one_year,
        financial_debt,
        shareholder_loans,
        trade,
        accruals_and_deferred_income,
    }) = &bs.payables
    {
        unsigned(&p("payables/total"), *total)?;
        unsigned(&p("payables/beyond_one_year"), *beyond_one_year)?;
        maturity(financial_debt, &p("payables/financial_debt"))?;
        maturity(shareholder_loans, &p("payables/shareholder_loans"))?;
        unsigned(&p("payables/trade"), *trade)?;
        unsigned(
            &p("payables/accruals_and_deferred_income"),
            *accruals_and_deferred_income,
        )?;
    }
    maturity(
        &bs.accruals_and_deferred_income,
        &p("accruals_and_deferred_income"),
    )?;
    unsigned(
        &p("total_liabilities_and_equity"),
        bs.total_liabilities_and_equity,
    )
}

fn statutory_income_statement(is: &StatutoryIncomeStatement, at: &str) -> Result<()> {
    let p = |k: &str| format!("{at}/{k}");
    for (k, v) in [
        ("revenue", is.revenue),
        ("own_work_capitalised", is.own_work_capitalised),
        ("other_revenue_and_income", is.other_revenue_and_income),
        ("purchases", is.purchases),
        ("services", is.services),
        ("use_of_third_party_assets", is.use_of_third_party_assets),
        ("personnel_costs", is.personnel_costs),
        ("provisions_for_risks", is.provisions_for_risks),
        ("other_provisions", is.other_provisions),
        ("other_operating_expenses", is.other_operating_expenses),
        (
            "income_from_equity_investments",
            is.income_from_equity_investments,
        ),
        ("other_financial_income", is.other_financial_income),
        (
            "interest_and_other_financial_charges",
            is.interest_and_other_financial_charges,
        ),
        (
            "revaluations_of_financial_assets",
            is.revaluations_of_financial_assets,
        ),
        (
            "write_downs_of_financial_assets",
            is.write_downs_of_financial_assets,
        ),
    ] {
        unsigned(&p(k), v)?;
    }
    if let Some(b10) = &is.depreciation_and_write_downs {
        for (k, v) in [
            ("total", b10.total),
            (
                "value_adjustments_of_fixed_assets",
                b10.value_adjustments_of_fixed_assets,
            ),
            (
                "depreciation_and_amortisation",
                b10.depreciation_and_amortisation,
            ),
            (
                "write_downs_of_fixed_assets",
                b10.write_downs_of_fixed_assets,
            ),
            ("write_downs_of_receivables", b10.write_downs_of_receivables),
        ] {
            unsigned(&p(&format!("depreciation_and_write_downs/{k}")), v)?;
        }
    }
    Ok(())
}

fn reclassified_balance_sheet(bs: &ReclassifiedBalanceSheet, at: &str) -> Result<()> {
    let p = |k: &str| format!("{at}/{k}");
    unsigned(&p("operating_fixed_assets"), bs.operating_fixed_assets)?;
    unsigned(&p("operating_provisions"), bs.operating_provisions)?;
    if let Some(owc) = &bs.operating_working_capital {
        for (k, v) in [
            ("trade_receivables", owc.trade_receivables),
            ("inventories", owc.inventories),
            ("other_operating_assets", owc.other_operating_assets),
            ("trade_payables", owc.trade_payables),
            (
                "other_operating_liabilities",
                owc.other_operating_liabilities,
            ),
        ] {
            unsigned(&p(&format!("operating_working_capital/{k}")), v)?;
        }
    }
    for (key, lines) in [
        ("non_operating_assets", &bs.non_operating_assets),
        ("quasi_equity_items", &bs.quasi_equity_items),
    ] {
        for (j, line) in lines.iter().flatten().enumerate() {
            unsigned(&p(&format!("{key}/{j}/value")), Some(line.value))?;
            not_blank(
                &p(&format!("{key}/{j}/label")),
                &line.label,
                "name the item",
            )?;
        }
    }
    for (j, c) in bs.debt_like_items.iter().flatten().enumerate() {
        let item = p(&format!("debt_like_items/{j}"));
        unsigned(&format!("{item}/value"), Some(c.value))?;
        not_blank(&format!("{item}/label"), &c.label, "name the item")?;
        match (&c.basis_note, c.measurement_basis) {
            (Some(n), _) if n.trim().is_empty() => {
                return Err(invalid(format!(
                    "`{item}/basis_note` is blank: write the note or remove the field"
                )));
            }
            (None, crate::dcf::MeasurementBasis::Other) => {
                return Err(invalid(format!(
                    "`{item}/basis_note` is missing: with `measurement_basis` `other`, describe \
                     how the amount was measured in `basis_note`"
                )));
            }
            _ => {}
        }
    }
    if let Some(nfp) = &bs.net_financial_position {
        if let Some(pos) = &nfp.position {
            unsigned(
                &p("net_financial_position/position/gross_debt"),
                Some(pos.gross_debt),
            )?;
            unsigned(&p("net_financial_position/position/cash"), Some(pos.cash))?;
        }
    }
    Ok(())
}

fn reclassified_income_statement(is: &ReclassifiedIncomeStatement, at: &str) -> Result<()> {
    let p = |k: &str| format!("{at}/{k}");
    for (k, v) in [
        ("revenue", is.revenue),
        ("purchases", is.purchases),
        ("personnel_costs", is.personnel_costs),
        (
            "depreciation_and_amortisation",
            is.depreciation_and_amortisation,
        ),
        ("other_charges_below_ebitda", is.other_charges_below_ebitda),
        (
            "interest_and_other_financial_charges",
            is.interest_and_other_financial_charges,
        ),
    ] {
        unsigned(&p(k), v)?;
    }
    Ok(())
}

/// R5, R10, R21: the list of amounts, the label and reason, the signs, the current part of a
/// lease liability within the liability, and no amount for a reclassified entry.
fn adjustment(input: &ReorganiseInput, k: usize, adj: &Adjustment) -> Result<()> {
    let at = format!("/adjustments/{k}");
    not_blank(&format!("{at}/label"), adj.label(), "name the adjustment")?;
    not_blank(
        &format!("{at}/reason"),
        adj.reason(),
        "state why the adjustment is made (R10)",
    )?;
    if let Adjustment::Investing(a) = adj {
        if let Some(related) = &a.related_non_operating_asset {
            not_blank(
                &format!("{at}/related_non_operating_asset"),
                related,
                "name the non-operating asset, or remove the field",
            )?;
        }
    }
    let n = input.years.len();
    if adj.len() != n {
        return Err(invalid(format!(
            "`{at}/amounts` has {} entries for {n} years: give one entry per year, in the order \
             of `years`, with null where the adjustment does not apply",
            adj.len()
        )));
    }
    for (i, year) in input.years.iter().enumerate() {
        let amount = adj.amounts().and_then(|a| a[i]);
        unsigned(&format!("{at}/amounts/{i}"), amount)?;
        let lease = match adj {
            Adjustment::Lease(l) => l.amounts[i].as_ref(),
            _ => None,
        };
        if let Some(la) = lease {
            lease_signs(la, &format!("{at}/amounts/{i}"))?;
        }
        if adj.applies_to(i) && year.reclassified.is_some() {
            return Err(invalid(format!(
                "`{at}/amounts/{i}` is given for a reclassified entry: adjustments apply to \
                 statutory entries only (R5); give the reclassified totals already adjusted, \
                 and null here"
            )));
        }
        if let Some(la) = lease {
            lease_current_part(la, &format!("{at}/amounts/{i}"))?;
        }
    }
    Ok(())
}

/// R21: every amount of a lease restatement is zero or more.
fn lease_signs(la: &LeaseAmounts, at: &str) -> Result<()> {
    for (k, v) in [
        ("asset", la.asset),
        ("liability", la.liability),
        ("liability_current", la.liability_current),
        ("prepayment", la.prepayment),
        ("payments", la.payments),
        ("depreciation", la.depreciation),
        ("interest", la.interest),
    ] {
        unsigned(&format!("{at}/{k}"), Some(v))?;
    }
    Ok(())
}

/// R21: the current part of the lease liability is a part of it (`FM-E026` otherwise, as for
/// any part within its whole).
fn lease_current_part(la: &LeaseAmounts, at: &str) -> Result<()> {
    if la.liability_current > la.liability {
        return Err(FairmarkError::StatementsDoNotReconcile(Box::new(
            Reconciliation {
                left_name: format!(
                    "the current part of the lease liability (`{at}/liability_current`)"
                ),
                left: la.liability_current,
                right_name: format!("the lease liability (`{at}/liability`)"),
                right: la.liability,
                difference: crate::num::sub(la.liability_current, la.liability)?,
                fix: "the part due within the next financial year cannot exceed the liability: \
                      correct one of them (R21)"
                    .to_owned(),
            },
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Date {
        s.parse().unwrap()
    }

    #[test]
    fn consecutive_years_follow_the_end_of_month_rule() {
        assert!(consecutive(d("2023-12-31"), d("2024-12-31")));
        assert!(consecutive(d("2023-02-28"), d("2024-02-29")));
        assert!(consecutive(d("2024-02-29"), d("2025-02-28")));
        assert!(consecutive(d("2023-04-30"), d("2024-04-30")));
        assert!(consecutive(d("2023-06-15"), d("2024-06-15")));
        assert!(!consecutive(d("2023-02-27"), d("2024-02-29")));
        assert!(!consecutive(d("2024-12-31"), d("2023-12-31")));
        assert!(!consecutive(d("2023-12-31"), d("2025-12-31")));
        assert!(!consecutive(d("2023-06-30"), d("2024-07-31")));
        assert!(!consecutive(d("2023-04-29"), d("2024-04-30")));
    }
}
