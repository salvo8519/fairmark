//! Validation before any computation (R2, R3, R5, R9, R10, R24): forms, dates, signs,
//! reclassified lines, overrides and adjustments. Every refusal is `FM-E001` naming the field
//! by its JSON pointer and stating the fix. Unknown fields, unknown destinations and an
//! override of another aggregate are refused while the input is read (`deny_unknown_fields`).

use rust_decimal::Decimal;

use super::input::{
    Adjustment, DebtLikeProvisionAdjustment, Equity, FinancialFixedAssetsOverride, LeaseAmounts,
    NetFixedAsset, Payables, ProvisionSource, Provisions, Receivables, ReclassifiedBalanceSheet,
    ReclassifiedIncomeStatement, ReorganiseInput, ShareholderLoansDestination,
    ShareholderLoansOverride, StatutoryBalanceSheet, StatutoryIncomeStatement, WithMaturity,
};
use crate::dates::{days_in_month, Date};
use crate::error::{FairmarkError, Found, Reason, Reconciliation, Result};

fn invalid(message: String) -> FairmarkError {
    FairmarkError::InvalidInput(message)
}

/// Every check of this module, in a fixed order: every refusal is collected (ADR 0014 G5), and
/// the first is the one a single refusal would be.
pub(crate) fn validate(input: &ReorganiseInput) -> Result<()> {
    let mut f = Found::new();
    if input.years.is_empty() {
        return Err(invalid(
            "`/years` is empty: give at least one financial year, oldest first".into(),
        )
        .at("/years", Reason::Length)
        .fix("give at least one financial year"));
    }
    for (i, y) in input.years.iter().enumerate() {
        if y.statutory.is_some() && y.reclassified.is_some() {
            f.push(
                invalid(format!(
                    "`/years/{i}` has both `statutory` and `reclassified`: give one form per \
                     year; to compare the two forms of the same year, make two calls"
                ))
                .at(format!("/years/{i}"), Reason::Conflict)
                .between([
                    format!("/years/{i}/statutory"),
                    format!("/years/{i}/reclassified"),
                ])
                .fix("give one form per year, and make two calls to compare the two forms"),
            );
        }
    }
    dates(input, &mut f);
    for (i, y) in input.years.iter().enumerate() {
        if let Some(st) = &y.statutory {
            if let Some(bs) = &st.balance_sheet {
                statutory_balance_sheet(bs, &format!("/years/{i}/statutory/balance_sheet"), &mut f);
            }
            if let Some(is) = &st.income_statement {
                statutory_income_statement(
                    is,
                    &format!("/years/{i}/statutory/income_statement"),
                    &mut f,
                );
            }
        }
        if let Some(rc) = &y.reclassified {
            if let Some(bs) = &rc.balance_sheet {
                reclassified_balance_sheet(
                    bs,
                    &format!("/years/{i}/reclassified/balance_sheet"),
                    &mut f,
                );
            }
            if let Some(is) = &rc.income_statement {
                reclassified_income_statement(
                    is,
                    &format!("/years/{i}/reclassified/income_statement"),
                    &mut f,
                );
            }
            if y.treatment_overrides.is_some() {
                f.push(
                    invalid(format!(
                        "`/years/{i}/treatment_overrides` is given for a reclassified entry: \
                         treatments apply to statutory entries only (R5); give the totals \
                         already reclassified, or remove the overrides"
                    ))
                    .at(format!("/years/{i}/treatment_overrides"), Reason::Requires)
                    .fix("remove the overrides, and give the totals already reclassified"),
                );
            }
        }
        if let Some(o) = &y.treatment_overrides {
            let at = format!("/years/{i}/treatment_overrides");
            if let Some(FinancialFixedAssetsOverride::Split(parts)) = &o.financial_fixed_assets {
                for (j, p) in parts.iter().enumerate() {
                    let pp = format!("{at}/financial_fixed_assets/split/{j}");
                    unsigned(&format!("{pp}/amount"), Some(p.amount), &mut f);
                    not_blank(&format!("{pp}/label"), &p.label, "name the part", &mut f);
                }
            }
            if let Some(ShareholderLoansOverride::Split(parts)) = &o.shareholder_loans {
                for (j, p) in parts.iter().enumerate() {
                    let pp = format!("{at}/shareholder_loans/split/{j}");
                    unsigned(&format!("{pp}/amount"), Some(p.amount), &mut f);
                    unsigned(&format!("{pp}/beyond_one_year"), p.beyond_one_year, &mut f);
                    if p.beyond_one_year.is_some()
                        && p.destination == ShareholderLoansDestination::QuasiEquity
                    {
                        f.push(
                            invalid(format!(
                                "`{pp}/beyond_one_year` is given for a `quasi_equity` part: a \
                                 quasi-equity part is not in the net financial position and has \
                                 no maturity there; remove the field, or give the part the \
                                 destination `financial_debt`"
                            ))
                            .at(format!("{pp}/beyond_one_year"), Reason::Requires)
                            .between([format!("{pp}/beyond_one_year"), format!("{pp}/destination")])
                            .fix(
                                "remove `beyond_one_year`, or give the part the destination \
                                 `financial_debt`",
                            ),
                        );
                    }
                    not_blank(&format!("{pp}/label"), &p.label, "name the part", &mut f);
                }
            }
        }
    }
    for (k, adj) in input.adjustments.iter().enumerate() {
        adjustment(input, k, adj, &mut f);
    }
    f.finish()
}

/// R2: oldest first, one financial year apart when consecutive entries both have a date.
fn dates(input: &ReorganiseInput, f: &mut Found) {
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
            f.push(
                invalid(format!(
                    "`/years/{i}/date` is {b} and `/years/{}/date` is {a}: {order}; list the \
                     years oldest first, one financial year apart (same month and day one \
                     calendar year later, or both the last day of their month); reorganise a \
                     year of a different length in a separate call",
                    i - 1
                ))
                .at(format!("/years/{i}/date"), Reason::Conflict)
                .between([format!("/years/{i}/date"), format!("/years/{}/date", i - 1)])
                .fix(
                    "list the years oldest first, one financial year apart, or reorganise a \
                     year of a different length in a separate call",
                ),
            );
        }
    }
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

fn unsigned(at: &str, value: Option<Decimal>, f: &mut Found) {
    if let Some(v) = value.filter(|v| v.is_sign_negative() && !v.is_zero()) {
        f.push(
            invalid(format!("`{at}` is {v}: {SIGN_FIX}"))
                .at(at, Reason::Sign)
                .expected(["zero or more"])
                .fix("give the amount without sign"),
        );
    }
}

fn not_blank(at: &str, text: &str, fix: &str, f: &mut Found) {
    if text.trim().is_empty() {
        f.push(
            invalid(format!("`{at}` is blank: {fix}"))
                .at(at, Reason::Blank)
                .fix(fix),
        );
    }
}

fn net(node: &Option<NetFixedAsset>, at: &str, f: &mut Found) {
    if let Some(n) = node {
        unsigned(&format!("{at}/total"), n.total, f);
        unsigned(&format!("{at}/gross"), n.gross, f);
        unsigned(&format!("{at}/accumulated"), n.accumulated, f);
    }
}

fn maturity(node: &Option<WithMaturity>, at: &str, f: &mut Found) {
    if let Some(n) = node {
        unsigned(&format!("{at}/total"), n.total, f);
        unsigned(&format!("{at}/beyond_one_year"), n.beyond_one_year, f);
    }
}

fn statutory_balance_sheet(bs: &StatutoryBalanceSheet, at: &str, f: &mut Found) {
    let p = |k: &str| format!("{at}/{k}");
    unsigned(
        &p("subscribed_capital_unpaid"),
        bs.subscribed_capital_unpaid,
        f,
    );
    net(&bs.intangible_assets, &p("intangible_assets"), f);
    net(&bs.tangible_assets, &p("tangible_assets"), f);
    unsigned(&p("financial_fixed_assets"), bs.financial_fixed_assets, f);
    unsigned(&p("fixed_assets"), bs.fixed_assets, f);
    unsigned(&p("inventories"), bs.inventories, f);
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
            unsigned(&p(&format!("receivables/{k}")), *v, f);
        }
    }
    unsigned(
        &p("current_financial_assets"),
        bs.current_financial_assets,
        f,
    );
    unsigned(&p("cash"), bs.cash, f);
    unsigned(&p("current_assets"), bs.current_assets, f);
    maturity(
        &bs.prepayments_and_accrued_income,
        &p("prepayments_and_accrued_income"),
        f,
    );
    unsigned(&p("total_assets"), bs.total_assets, f);
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
            unsigned(&p(&format!("equity/{k}")), *v, f);
        }
        if let Some(v) = negative_reserve_for_own_shares {
            if v.is_sign_positive() && !v.is_zero() {
                let at = p("equity/negative_reserve_for_own_shares");
                f.push(
                    invalid(format!(
                        "`{at}` is {v}: the negative reserve for own shares (A.X) is zero or \
                         negative, as presented in the statutory layout (R3); give it with its \
                         minus sign"
                    ))
                    .at(at, Reason::Sign)
                    .expected(["zero or less"])
                    .fix("give the amount with its minus sign"),
                );
            }
        }
    }
    if let Some(Provisions {
        total,
        beyond_one_year,
        of_which_derivative_liabilities,
    }) = &bs.provisions
    {
        unsigned(&p("provisions/total"), *total, f);
        unsigned(&p("provisions/beyond_one_year"), *beyond_one_year, f);
        unsigned(
            &p("provisions/of_which_derivative_liabilities"),
            *of_which_derivative_liabilities,
            f,
        );
    }
    maturity(
        &bs.employee_severance_indemnity,
        &p("employee_severance_indemnity"),
        f,
    );
    if let Some(Payables {
        total,
        beyond_one_year,
        financial_debt,
        shareholder_loans,
        trade,
        accruals_and_deferred_income,
    }) = &bs.payables
    {
        unsigned(&p("payables/total"), *total, f);
        unsigned(&p("payables/beyond_one_year"), *beyond_one_year, f);
        maturity(financial_debt, &p("payables/financial_debt"), f);
        maturity(shareholder_loans, &p("payables/shareholder_loans"), f);
        unsigned(&p("payables/trade"), *trade, f);
        unsigned(
            &p("payables/accruals_and_deferred_income"),
            *accruals_and_deferred_income,
            f,
        );
    }
    maturity(
        &bs.accruals_and_deferred_income,
        &p("accruals_and_deferred_income"),
        f,
    );
    unsigned(
        &p("total_liabilities_and_equity"),
        bs.total_liabilities_and_equity,
        f,
    );
}

fn statutory_income_statement(is: &StatutoryIncomeStatement, at: &str, f: &mut Found) {
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
        unsigned(&p(k), v, f);
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
            unsigned(&p(&format!("depreciation_and_write_downs/{k}")), v, f);
        }
    }
}

fn reclassified_balance_sheet(bs: &ReclassifiedBalanceSheet, at: &str, f: &mut Found) {
    let p = |k: &str| format!("{at}/{k}");
    unsigned(&p("operating_fixed_assets"), bs.operating_fixed_assets, f);
    unsigned(&p("operating_provisions"), bs.operating_provisions, f);
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
            unsigned(&p(&format!("operating_working_capital/{k}")), v, f);
        }
    }
    for (key, lines) in [
        ("non_operating_assets", &bs.non_operating_assets),
        ("quasi_equity_items", &bs.quasi_equity_items),
    ] {
        for (j, line) in lines.iter().flatten().enumerate() {
            unsigned(&p(&format!("{key}/{j}/value")), Some(line.value), f);
            not_blank(
                &p(&format!("{key}/{j}/label")),
                &line.label,
                "name the item",
                f,
            );
        }
    }
    for (j, c) in bs.debt_like_items.iter().flatten().enumerate() {
        let item = p(&format!("debt_like_items/{j}"));
        unsigned(&format!("{item}/value"), Some(c.value), f);
        not_blank(&format!("{item}/label"), &c.label, "name the item", f);
        match (&c.basis_note, c.measurement_basis) {
            (Some(n), _) if n.trim().is_empty() => {
                f.push(
                    invalid(format!(
                        "`{item}/basis_note` is blank: write the note or remove the field"
                    ))
                    .at(format!("{item}/basis_note"), Reason::Blank)
                    .fix("write the note, or remove the field"),
                );
            }
            (None, crate::dcf::MeasurementBasis::Other) => {
                f.push(
                    invalid(format!(
                        "`{item}/basis_note` is missing: with `measurement_basis` `other`, \
                         describe how the amount was measured in `basis_note`"
                    ))
                    .at(format!("{item}/basis_note"), Reason::Missing)
                    .between([
                        format!("{item}/basis_note"),
                        format!("{item}/measurement_basis"),
                    ])
                    .fix("describe in `basis_note` how the amount was measured"),
                );
            }
            _ => {}
        }
    }
    if let Some(nfp) = &bs.net_financial_position {
        if let Some(pos) = &nfp.position {
            unsigned(
                &p("net_financial_position/position/gross_debt"),
                Some(pos.gross_debt),
                f,
            );
            unsigned(
                &p("net_financial_position/position/cash"),
                Some(pos.cash),
                f,
            );
        }
    }
}

fn reclassified_income_statement(is: &ReclassifiedIncomeStatement, at: &str, f: &mut Found) {
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
            "income_from_operating_investments",
            is.income_from_operating_investments,
        ),
        (
            "interest_and_other_financial_charges",
            is.interest_and_other_financial_charges,
        ),
    ] {
        unsigned(&p(k), v, f);
    }
}

/// R5, R10, R21: the list of amounts, the label and reason, the signs, the current part of a
/// lease liability within the liability, and no amount for a reclassified entry.
fn adjustment(input: &ReorganiseInput, k: usize, adj: &Adjustment, f: &mut Found) {
    let at = format!("/adjustments/{k}");
    not_blank(
        &format!("{at}/label"),
        adj.label(),
        "name the adjustment",
        f,
    );
    not_blank(
        &format!("{at}/reason"),
        adj.reason(),
        "state why the adjustment is made (R10)",
        f,
    );
    if let Adjustment::Investing(a) = adj {
        if let Some(related) = &a.related_non_operating_asset {
            not_blank(
                &format!("{at}/related_non_operating_asset"),
                related,
                "name the non-operating asset, or remove the field",
                f,
            );
        }
    }
    let n = input.years.len();
    if adj.len() != n {
        // The amounts cannot be read year by year: the rest of this adjustment is not checked.
        f.push(
            invalid(format!(
                "`{at}/amounts` has {} entries for {n} years: give one entry per year, in the \
                 order of `years`, with null where the adjustment does not apply",
                adj.len()
            ))
            .at(format!("{at}/amounts"), Reason::Length)
            .expected([format!("{n} entries, one per year")])
            .fix("give one entry per year, with null where the adjustment does not apply"),
        );
        return;
    }
    for (i, year) in input.years.iter().enumerate() {
        let amount = adj.amounts().and_then(|a| a[i]);
        unsigned(&format!("{at}/amounts/{i}"), amount, f);
        let lease = match adj {
            Adjustment::Lease(l) => l.amounts[i].as_ref(),
            _ => None,
        };
        if let Some(la) = lease {
            lease_signs(la, &format!("{at}/amounts/{i}"), f);
        }
        if adj.applies_to(i) && year.reclassified.is_some() {
            f.push(
                invalid(format!(
                    "`{at}/amounts/{i}` is given for a reclassified entry: adjustments apply to \
                     statutory entries only (R5); give the reclassified totals already \
                     adjusted, and null here"
                ))
                .at(format!("{at}/amounts/{i}"), Reason::Requires)
                .between([
                    format!("{at}/amounts/{i}"),
                    format!("/years/{i}/reclassified"),
                ])
                .fix("give null here, and the reclassified totals already adjusted"),
            );
        }
        if let Some(la) = lease {
            f.check(lease_current_part(la, &format!("{at}/amounts/{i}")));
        }
    }
    if let Adjustment::DebtLikeProvision(a) = adj {
        if let Some(costs) = &a.interest_cost {
            interest_cost(a, costs, n, &at, f);
        }
    }
}

/// P2: the interest cost of the employee severance indemnity, one entry per year, zero or
/// more, only with `aggregate` `employee_severance_indemnity` and only where the adjustment
/// applies; that it fits in B9 is checked when it is moved (R10).
fn interest_cost(
    a: &DebtLikeProvisionAdjustment,
    costs: &[Option<Decimal>],
    n: usize,
    at: &str,
    f: &mut Found,
) {
    if a.aggregate != ProvisionSource::EmployeeSeveranceIndemnity {
        f.push(
            invalid(format!(
                "`{at}/interest_cost` is given for a debt-like provision of `provisions`: the \
                 interest cost applies only to the employee severance indemnity (P2); remove it"
            ))
            .at(format!("{at}/interest_cost"), Reason::Requires)
            .between([format!("{at}/interest_cost"), format!("{at}/aggregate")])
            .fix("remove `interest_cost`"),
        );
        return;
    }
    if costs.len() != n {
        f.push(
            invalid(format!(
                "`{at}/interest_cost` has {} entries for {n} years: give one entry per year, in \
                 the order of `years`, with null where it is not given",
                costs.len()
            ))
            .at(format!("{at}/interest_cost"), Reason::Length)
            .expected([format!("{n} entries, one per year")])
            .fix("give one entry per year, with null where it is not given"),
        );
        return;
    }
    for (i, c) in costs.iter().enumerate() {
        unsigned(&format!("{at}/interest_cost/{i}"), *c, f);
        if c.is_some() && a.amounts[i].is_none() {
            f.push(
                invalid(format!(
                    "`{at}/interest_cost/{i}` is given for a year where the adjustment does \
                     not apply (`{at}/amounts/{i}` is null): the interest cost moves only with \
                     a debt-like severance indemnity; give the amount, or null here"
                ))
                .at(format!("{at}/interest_cost/{i}"), Reason::Requires)
                .between([
                    format!("{at}/interest_cost/{i}"),
                    format!("{at}/amounts/{i}"),
                ])
                .fix("give the amount of the adjustment for that year, or null here"),
            );
        }
    }
}

/// R21: every amount of a lease restatement is zero or more.
fn lease_signs(la: &LeaseAmounts, at: &str, f: &mut Found) {
    for (k, v) in [
        ("asset", la.asset),
        ("liability", la.liability),
        ("liability_current", la.liability_current),
        ("prepayment", la.prepayment),
        ("payments", la.payments),
        ("depreciation", la.depreciation),
        ("interest", la.interest),
    ] {
        unsigned(&format!("{at}/{k}"), Some(v), f);
    }
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
                pointers: vec![format!("{at}/liability_current"), format!("{at}/liability")],
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
