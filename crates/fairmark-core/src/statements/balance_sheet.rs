//! The statutory balance sheet reorganised (Appendix A.1 and B.1; R8 to R15, R22): each
//! aggregate assigned to its destination (fixed, default, or by the analyst's treatment or
//! override), the balance sheet adjustments of Appendix C applied in the order given, then the
//! operating block, the non-operating assets, the sources with the net financial position in
//! the SPEC-0007 type, the bridge items in the `dcf.fcff` types and the current and non-current
//! block for SPEC-0007.

use std::collections::BTreeMap;

use rust_decimal::Decimal;

use super::checks::{self, Checks, Side};
use super::input::{
    Adjustment, DerivativeLiabilitiesDestination as Deriv, Equity,
    FinancialFixedAssetsDestination as Ffa, FinancialFixedAssetsOverride, Maturity, NetFixedAsset,
    NonOperatingSource, ProvisionSource, ReorganiseInput, ShareholderLoansDestination as Sl,
    ShareholderLoansOverride, StatutoryBalanceSheet, StatutoryIncomeStatement, WithMaturity,
};
use super::partial::{total, Partial, Sign};
use super::{
    AppliedAdjustment, Assignment, BalanceSheet, CurrentNonCurrent, Destination,
    NetFinancialPositionBlock, NfpComponent, Notes, OperatingBlock, Origin, Sources, YearBody,
};
use crate::dcf::{BridgeClaim, MeasurementBasis};
use crate::error::{FairmarkError, Result};
use crate::explain::Explain;
use crate::ratios::{NetFinancialPosition, NfpItem};
use crate::tv::Line;

use Sign::{Minus, Plus};

/// The amount of an aggregate that a treatment sends to one destination.
struct Share {
    destination: Destination,
    label: String,
    amount: Partial,
    /// For a part of a split of shareholder loans: its part beyond one year.
    beyond: Option<Partial>,
}

/// A treatment as read for one year: its pointer, and one destination or a split.
enum Choice {
    None,
    Whole(Destination),
    Split(Vec<(Destination, Decimal, String, Option<Decimal>)>),
}

/// The destination shares of an aggregate with a required treatment (R8, R9), and the
/// permitted destinations of a treatment that is needed and missing.
#[allow(clippy::too_many_arguments)]
fn shares_of(
    checks: &mut Checks,
    value: &Partial,
    aggregate: (&str, String),
    choice: Choice,
    pointer: &str,
    permitted: &[(Destination, &'static str)],
    label: &str,
    relation: &'static str,
    permits: &mut BTreeMap<String, Vec<&'static str>>,
) -> Result<Vec<Share>> {
    let share = |destination, amount: &Partial| Share {
        destination,
        label: label.to_owned(),
        amount: amount.clone(),
        beyond: None,
    };
    Ok(match choice {
        Choice::None => {
            if value.value().is_some_and(|v| v.is_zero()) {
                return Ok(Vec::new());
            }
            let amount = if value.is_known() {
                permits.insert(
                    pointer.to_owned(),
                    permitted.iter().map(|(_, name)| *name).collect(),
                );
                Partial::lacking(pointer)
            } else {
                value.clone()
            };
            permitted.iter().map(|(d, _)| share(*d, &amount)).collect()
        }
        Choice::Whole(d) => vec![share(d, value)],
        Choice::Split(parts) => {
            let split_pointer = format!("{pointer}/split");
            let mut sum = Decimal::ZERO;
            for (_, amount, _, _) in &parts {
                sum = crate::num::add(sum, *amount)?;
            }
            match value.value() {
                Some(v) => checks.equal(
                    relation,
                    Side::new(aggregate.0, vec![aggregate.1], Some(v)),
                    Side::new(
                        "the parts of the split",
                        vec![split_pointer.clone()],
                        Some(sum),
                    ),
                    "the parts of a split sum to the aggregate of the year (R9): correct an \
                     amount of the split",
                )?,
                None => checks.not_checked(relation, vec![split_pointer.clone()]),
            }
            parts
                .into_iter()
                .enumerate()
                .map(|(j, (destination, amount, label, beyond))| Share {
                    destination,
                    label,
                    amount: if value.is_known() {
                        Partial::Known(amount)
                    } else {
                        value.clone()
                    },
                    beyond: Some(Partial::of(beyond, || {
                        format!("{split_pointer}/{j}/beyond_one_year")
                    })),
                })
                .collect()
        }
    })
}

fn dest_sum(shares: &[Share], destination: Destination) -> Result<Partial> {
    Partial::plus_all(
        shares
            .iter()
            .filter(|s| s.destination == destination)
            .map(|s| &s.amount),
    )
}

/// Where an adjustment takes its amount from (Appendix C).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Source {
    IntangibleAssets,
    TangibleAssets,
    Receivables,
    CurrentFinancialAssets,
    Cash,
    Accruals,
    PayablesRemainder,
    Provisions,
    EmployeeSeveranceIndemnity,
    Equity,
}

impl Source {
    fn key(self) -> &'static str {
        match self {
            Self::IntangibleAssets => "intangible_assets",
            Self::TangibleAssets => "tangible_assets",
            Self::Receivables => "receivables",
            Self::CurrentFinancialAssets => "current_financial_assets",
            Self::Cash => "cash",
            Self::Accruals => "accruals_and_deferred_income",
            Self::PayablesRemainder => "payables",
            Self::Provisions => "provisions",
            Self::EmployeeSeveranceIndemnity => "employee_severance_indemnity",
            Self::Equity => "equity",
        }
    }
}

/// An adjustment applied to one year.
struct Moved {
    index: usize,
    kind: &'static str,
    label: String,
    amount: Partial,
    maturity: Option<Maturity>,
}

/// Reads the statutory amounts with the pointers that name them when they are missing (R6).
struct Reader {
    base: String,
}

impl Reader {
    fn p(&self, key: &str) -> String {
        format!("{}/{key}", self.base)
    }

    fn leaf(&self, value: Option<Decimal>, key: &str) -> Partial {
        Partial::of(value, || self.p(key))
    }

    /// A node with a `total`: the node's pointer when it is absent, its total's when that is.
    fn node(&self, node: Option<&WithMaturity>, key: &str) -> Partial {
        match node {
            None => Partial::lacking(self.p(key)),
            Some(n) => Partial::of(n.total, || self.p(&format!("{key}/total"))),
        }
    }

    fn part(&self, given: bool, value: Option<Decimal>, key: &str, sub: &str) -> Partial {
        if !given {
            return Partial::lacking(self.p(key));
        }
        Partial::of(value, || self.p(&format!("{key}/{sub}")))
    }

    fn net(&self, node: Option<&NetFixedAsset>, key: &str) -> Result<Partial> {
        let Some(n) = node else {
            return Ok(Partial::lacking(self.p(key)));
        };
        if let Some(t) = n.total {
            return Ok(Partial::Known(t));
        }
        if let (Some(g), Some(a)) = (n.gross, n.accumulated) {
            return Ok(Partial::Known(crate::num::sub(g, a)?));
        }
        Ok(Partial::lacking(self.p(&format!("{key}/total"))))
    }

    fn equity(&self, node: Option<&Equity>) -> Result<Partial> {
        let Some(e) = node else {
            return Ok(Partial::lacking(self.p("equity")));
        };
        if let Some(t) = e.total {
            return Ok(Partial::Known(t));
        }
        let parts = [
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
        ];
        Ok(match checks::sum_all(&parts)? {
            Some(v) => Partial::Known(v),
            None => Partial::lacking(self.p("equity/total")),
        })
    }

    /// Item A, assets D or liabilities E as a separate item: zero when presented within C.II
    /// or D and not given separately (art. 2435-bis).
    fn separate(&self, given: Partial, is_given: bool, within_given: bool) -> Partial {
        if !is_given && within_given {
            Partial::ZERO
        } else {
            given
        }
    }

    /// The non-current part of a node with a default maturity (R15): given, or by the rule.
    fn beyond_or(
        &self,
        node: Option<&WithMaturity>,
        whole: &Partial,
        whole_if_absent: bool,
    ) -> Partial {
        if !whole.is_known() {
            return whole.clone();
        }
        match node.and_then(|n| n.beyond_one_year) {
            Some(v) => Partial::Known(v),
            None if whole_if_absent => whole.clone(),
            None => Partial::ZERO,
        }
    }
}

/// The reorganised statutory balance sheet of year `i` (`year` names it in messages).
pub(crate) fn statutory(
    input: &ReorganiseInput,
    i: usize,
    year: &str,
    ex: &mut Explain,
    notes: &mut Notes,
) -> Result<YearBody> {
    let y = &input.years[i];
    let st = y.statutory.clone().unwrap_or_default();
    let bs = st.balance_sheet.clone().unwrap_or_default();
    let is = st.income_statement.clone().unwrap_or_default();
    let base = format!("/years/{i}/statutory/balance_sheet");
    let is_base = format!("/years/{i}/statutory/income_statement");
    let r = Reader { base: base.clone() };
    let mut checks = Checks::new(year);
    checks::balance_sheet(&mut checks, &bs, &is, &base, &is_base)?;
    checks::income_statement(&mut checks, &is, &is_base)?;
    statutory_conventions(&bs, &is, &r, year, ex);

    // Items of Appendix A.1.
    let rec = bs.receivables.clone();
    let pay = bs.payables.clone();
    let prov_node = bs.provisions.clone();
    let a_within = rec.as_ref().and_then(|n| n.subscribed_capital_unpaid);
    let d_within = rec.as_ref().and_then(|n| n.prepayments_and_accrued_income);
    let e_within = pay.as_ref().and_then(|n| n.accruals_and_deferred_income);
    let a_within_v = Partial::Known(a_within.unwrap_or_default());
    let a_sep = r.separate(
        r.leaf(bs.subscribed_capital_unpaid, "subscribed_capital_unpaid"),
        bs.subscribed_capital_unpaid.is_some(),
        a_within.is_some(),
    );
    let a_total = total(
        ex,
        &format!("Subscribed capital unpaid, deducted from equity ({year})"),
        &[
            (Plus, "assets A", &a_sep),
            (Plus, "within C.II", &a_within_v),
        ],
    )?;
    let b1 = r.net(bs.intangible_assets.as_ref(), "intangible_assets")?;
    let b2 = r.net(bs.tangible_assets.as_ref(), "tangible_assets")?;
    let b3 = r.leaf(bs.financial_fixed_assets, "financial_fixed_assets");
    let c1 = r.leaf(bs.inventories, "inventories");
    let c2 = match &rec {
        None => Partial::lacking(r.p("receivables")),
        Some(n) => Partial::of(n.total, || r.p("receivables/total")),
    };
    let c3 = r.leaf(bs.current_financial_assets, "current_financial_assets");
    let c4 = r.leaf(bs.cash, "cash");
    let d_sep = r.separate(
        r.node(
            bs.prepayments_and_accrued_income.as_ref(),
            "prepayments_and_accrued_income",
        ),
        bs.prepayments_and_accrued_income.is_some(),
        d_within.is_some(),
    );
    let eq = r.equity(bs.equity.as_ref())?;
    let prov = match &prov_node {
        None => Partial::lacking(r.p("provisions")),
        Some(n) => Partial::of(n.total, || r.p("provisions/total")),
    };
    let deriv = prov_node
        .as_ref()
        .and_then(|n| n.of_which_derivative_liabilities)
        .unwrap_or_default();
    let tfr = r.node(
        bs.employee_severance_indemnity.as_ref(),
        "employee_severance_indemnity",
    );
    let payables = match &pay {
        None => Partial::lacking(r.p("payables")),
        Some(n) => Partial::of(n.total, || r.p("payables/total")),
    };
    let fd_node = pay.as_ref().and_then(|n| n.financial_debt.as_ref());
    let sl_node = pay.as_ref().and_then(|n| n.shareholder_loans.as_ref());
    let fd = r.node(fd_node, "payables/financial_debt");
    let sl = r.node(sl_node, "payables/shareholder_loans");
    let e_sep = r.separate(
        r.node(
            bs.accruals_and_deferred_income.as_ref(),
            "accruals_and_deferred_income",
        ),
        bs.accruals_and_deferred_income.is_some(),
        e_within.is_some(),
    );
    let e_within_v = Partial::Known(e_within.unwrap_or_default());
    let deriv_v = Partial::Known(deriv);

    // Treatments and overrides (R8, R9).
    let mut permits = BTreeMap::new();
    let ffa_choice = match y
        .treatment_overrides
        .as_ref()
        .and_then(|o| o.financial_fixed_assets.as_ref())
    {
        Some(FinancialFixedAssetsOverride::Destination(d)) => (
            Choice::Whole(ffa_destination(*d)),
            format!("/years/{i}/treatment_overrides/financial_fixed_assets"),
        ),
        Some(FinancialFixedAssetsOverride::Split(parts)) => (
            Choice::Split(
                parts
                    .iter()
                    .map(|p| {
                        (
                            ffa_destination(p.destination),
                            p.amount,
                            p.label.clone(),
                            None,
                        )
                    })
                    .collect(),
            ),
            format!("/years/{i}/treatment_overrides/financial_fixed_assets"),
        ),
        None => (
            input
                .treatments
                .financial_fixed_assets
                .map_or(Choice::None, |d| Choice::Whole(ffa_destination(d))),
            "/treatments/financial_fixed_assets".to_owned(),
        ),
    };
    let sl_choice = match y
        .treatment_overrides
        .as_ref()
        .and_then(|o| o.shareholder_loans.as_ref())
    {
        Some(ShareholderLoansOverride::Destination(d)) => (
            Choice::Whole(sl_destination(*d)),
            format!("/years/{i}/treatment_overrides/shareholder_loans"),
        ),
        Some(ShareholderLoansOverride::Split(parts)) => (
            Choice::Split(
                parts
                    .iter()
                    .map(|p| {
                        (
                            sl_destination(p.destination),
                            p.amount,
                            p.label.clone(),
                            p.beyond_one_year,
                        )
                    })
                    .collect(),
            ),
            format!("/years/{i}/treatment_overrides/shareholder_loans"),
        ),
        None => (
            input
                .treatments
                .shareholder_loans
                .map_or(Choice::None, |d| Choice::Whole(sl_destination(d))),
            "/treatments/shareholder_loans".to_owned(),
        ),
    };
    let ffa_analyst = !matches!(ffa_choice.0, Choice::None);
    let sl_analyst = !matches!(sl_choice.0, Choice::None);
    checks.not_checked("financial_fixed_assets_split", Vec::new());
    checks.not_checked("shareholder_loans_split", Vec::new());
    let ffa = shares_of(
        &mut checks,
        &b3,
        (
            "financial fixed assets B.III",
            r.p("financial_fixed_assets"),
        ),
        ffa_choice.0,
        &ffa_choice.1,
        &[
            (Destination::OperatingFixedAssets, "operating"),
            (Destination::NonOperatingAssets, "non_operating"),
            (
                Destination::NetFinancialPositionOtherFinancialAsset,
                "financial_asset",
            ),
        ],
        "Financial fixed assets (B.III)",
        "financial_fixed_assets_split",
        &mut permits,
    )?;
    let sls = shares_of(
        &mut checks,
        &sl,
        ("shareholder loans", r.p("payables/shareholder_loans/total")),
        sl_choice.0,
        &sl_choice.1,
        &[
            (Destination::NetFinancialPositionGrossDebt, "financial_debt"),
            (Destination::QuasiEquityItems, "quasi_equity"),
        ],
        "Shareholder loans (D)",
        "shareholder_loans_split",
        &mut permits,
    )?;
    let deriv_choice = input
        .treatments
        .derivative_liabilities
        .map_or(Choice::None, |d| Choice::Whole(deriv_destination(d)));
    let deriv_analyst = !matches!(deriv_choice, Choice::None);
    let dshares = shares_of(
        &mut checks,
        &deriv_v,
        (
            "derivative liabilities",
            r.p("provisions/of_which_derivative_liabilities"),
        ),
        deriv_choice,
        "/treatments/derivative_liabilities",
        &[
            (
                Destination::NetFinancialPositionOtherFinancialLiability,
                "financial_liability",
            ),
            (Destination::OperatingProvisions, "operating_provision"),
        ],
        "Derivative liabilities (B)",
        "derivative_liabilities_split",
        &mut permits,
    )?;

    // Assignments (R11): every aggregate given, with its destination and origin.
    let mut assignments = Vec::new();
    let mut assign = |ex: &mut Explain,
                      reference: &'static str,
                      pointer: String,
                      amount: &Partial,
                      destination: Destination,
                      origin: Origin| {
        if let Some(v) = amount.value() {
            ex.step(
                format!(
                    "Assignment ({year}): {reference} to {} ({})",
                    destination.id(),
                    origin.id()
                ),
                format!("`{pointer}` = {v}"),
                v,
            );
            assignments.push(Assignment {
                reference,
                aggregate: pointer,
                amount: v,
                destination,
                origin,
            });
        }
    };
    let mapping = Origin::Mapping;
    let origin_of = |analyst: bool| {
        if analyst {
            Origin::Analyst
        } else {
            Origin::Mapping
        }
    };
    if bs.subscribed_capital_unpaid.is_some() {
        assign(
            ex,
            "assets A",
            r.p("subscribed_capital_unpaid"),
            &a_sep,
            Destination::DeductedFromEquity,
            mapping,
        );
    }
    if a_within.is_some() {
        assign(
            ex,
            "assets A within C.II",
            r.p("receivables/subscribed_capital_unpaid"),
            &a_within_v,
            Destination::DeductedFromEquity,
            mapping,
        );
    }
    assign(
        ex,
        "B.I",
        r.p("intangible_assets"),
        &b1,
        Destination::OperatingFixedAssets,
        mapping,
    );
    assign(
        ex,
        "B.II",
        r.p("tangible_assets"),
        &b2,
        Destination::OperatingFixedAssets,
        mapping,
    );
    for s in &ffa {
        assign(
            ex,
            "B.III",
            r.p("financial_fixed_assets"),
            &s.amount,
            s.destination,
            origin_of(ffa_analyst),
        );
    }
    assign(
        ex,
        "C.I",
        r.p("inventories"),
        &c1,
        Destination::OperatingWorkingCapital,
        mapping,
    );
    let c2_op = Partial::sum(&[(Plus, &c2), (Minus, &a_within_v)])?;
    assign(
        ex,
        "C.II",
        r.p("receivables"),
        &c2_op,
        Destination::OperatingWorkingCapital,
        mapping,
    );
    assign(
        ex,
        "C.III",
        r.p("current_financial_assets"),
        &c3,
        Destination::NetFinancialPositionOtherFinancialAsset,
        mapping,
    );
    assign(
        ex,
        "C.IV",
        r.p("cash"),
        &c4,
        Destination::NetFinancialPositionCash,
        mapping,
    );
    if bs.prepayments_and_accrued_income.is_some() {
        assign(
            ex,
            "assets D",
            r.p("prepayments_and_accrued_income"),
            &d_sep,
            Destination::OperatingWorkingCapital,
            mapping,
        );
    }
    assign(
        ex,
        "liabilities A",
        r.p("equity"),
        &eq,
        Destination::Equity,
        mapping,
    );
    let prov_rest = Partial::sum(&[(Plus, &prov), (Minus, &deriv_v)])?;
    assign(
        ex,
        "liabilities B",
        r.p("provisions"),
        &prov_rest,
        Destination::OperatingProvisions,
        mapping,
    );
    for s in &dshares {
        assign(
            ex,
            "liabilities B, derivatives",
            r.p("provisions/of_which_derivative_liabilities"),
            &s.amount,
            s.destination,
            origin_of(deriv_analyst),
        );
    }
    assign(
        ex,
        "liabilities C",
        r.p("employee_severance_indemnity"),
        &tfr,
        Destination::OperatingProvisions,
        mapping,
    );
    assign(
        ex,
        "liabilities D, financial debt",
        r.p("payables/financial_debt"),
        &fd,
        Destination::NetFinancialPositionGrossDebt,
        mapping,
    );
    for s in &sls {
        assign(
            ex,
            "liabilities D, shareholder loans",
            r.p("payables/shareholder_loans"),
            &s.amount,
            s.destination,
            origin_of(sl_analyst),
        );
    }
    let pay_rest = Partial::sum(&[
        (Plus, &payables),
        (Minus, &fd),
        (Minus, &sl),
        (Minus, &e_within_v),
    ])?;
    assign(
        ex,
        "liabilities D, remainder",
        r.p("payables"),
        &pay_rest,
        Destination::OperatingWorkingCapital,
        mapping,
    );
    if bs.accruals_and_deferred_income.is_some() {
        assign(
            ex,
            "liabilities E",
            r.p("accruals_and_deferred_income"),
            &e_sep,
            Destination::OperatingWorkingCapital,
            mapping,
        );
    }
    if e_within.is_some() {
        assign(
            ex,
            "liabilities E within D",
            r.p("payables/accruals_and_deferred_income"),
            &e_within_v,
            Destination::OperatingWorkingCapital,
            mapping,
        );
    }

    // Adjustments of Appendix C, in the order given (R10).
    let deriv_fin = dest_sum(
        &dshares,
        Destination::NetFinancialPositionOtherFinancialLiability,
    )?;
    let mut remaining: BTreeMap<Source, Partial> = BTreeMap::from([
        (Source::IntangibleAssets, b1.clone()),
        (Source::TangibleAssets, b2.clone()),
        (Source::Receivables, c2_op.clone()),
        (Source::CurrentFinancialAssets, c3.clone()),
        (Source::Cash, c4.clone()),
        (
            Source::Accruals,
            Partial::sum(&[(Plus, &e_sep), (Plus, &e_within_v)])?,
        ),
        (Source::PayablesRemainder, pay_rest.clone()),
        (
            Source::Provisions,
            Partial::sum(&[(Plus, &prov), (Minus, &deriv_fin)])?,
        ),
        (Source::EmployeeSeveranceIndemnity, tfr.clone()),
        (
            Source::Equity,
            Partial::sum(&[(Plus, &eq), (Minus, &a_total)])?,
        ),
    ]);
    let mut moved = Vec::new();
    let mut applied = Vec::new();
    for (k, adj) in input.adjustments.iter().enumerate() {
        let Some(amount) = adj.amounts().and_then(|a| a.get(i)).copied().flatten() else {
            if !adj.applies_to(i) {
                notes.convention(format!(
                    "adjustment {k} '{}' does not apply to {year} (amount null)",
                    adj.label()
                ));
            }
            continue;
        };
        let (source, target, maturity) = route(adj);
        let rem = remaining
            .get(&source)
            .cloned()
            .ok_or(FairmarkError::Internal(
                "statements: unknown adjustment source",
            ))?;
        let took = match rem.value() {
            Some(left) => {
                if amount > left && !amount.is_zero() {
                    return Err(checks.error(
                        &Side::new(
                            format!("adjustment {k} '{}'", adj.label()),
                            vec![format!("/adjustments/{k}/amounts/{i}")],
                            Some(amount),
                        ),
                        amount,
                        &Side::new(
                            "what remains in its source".to_owned(),
                            vec![r.p(source.key())],
                            Some(left),
                        ),
                        left,
                        "an adjustment moves at most what remains in its source after the \
                         adjustments listed before it (R10): lower the amount or check the \
                         source",
                    )?);
                }
                remaining.insert(source, Partial::Known(crate::num::sub(left, amount)?));
                ex.step(
                    format!(
                        "Adjustment {k} {} '{}' ({year}): {} to {}",
                        adj.kind(),
                        adj.label(),
                        source.key(),
                        target.id()
                    ),
                    format!("as given; reason: {}", adj.reason()),
                    amount,
                );
                applied.push(AppliedAdjustment {
                    index: k,
                    kind: adj.kind(),
                    label: adj.label().to_owned(),
                    amount,
                    reason: adj.reason().to_owned(),
                    source: r.p(source.key()),
                    target,
                });
                Partial::Known(amount)
            }
            None => {
                notes.convention(format!(
                    "adjustment {k} '{}' not applied to {year}: its source `{}` is missing; the \
                     outputs it reaches are not computed",
                    adj.label(),
                    r.p(source.key())
                ));
                rem
            }
        };
        moved.push(Moved {
            index: k,
            kind: adj.kind(),
            label: adj.label().to_owned(),
            amount: took,
            maturity,
        });
    }
    let rem = |s: Source| remaining[&s].clone();
    let of_kind = |kind: &str| -> Vec<&Moved> { moved.iter().filter(|m| m.kind == kind).collect() };
    let op_cash = Partial::plus_all(of_kind("operating_cash").iter().map(|m| &m.amount))?;

    // Operating block (B.1).
    let ffa_op = dest_sum(&ffa, Destination::OperatingFixedAssets)?;
    let ofa = total(
        ex,
        &format!("Operating fixed assets ({year})"),
        &[
            (Plus, "B.I", &rem(Source::IntangibleAssets)),
            (Plus, "B.II", &rem(Source::TangibleAssets)),
            (Plus, "B.III operating", &ffa_op),
        ],
    )?;
    let owc = total(
        ex,
        &format!("Operating working capital ({year})"),
        &[
            (Plus, "C.I", &c1),
            (Plus, "C.II operating", &rem(Source::Receivables)),
            (Plus, "assets D", &d_sep),
            (Plus, "operating cash", &op_cash),
            (
                Minus,
                "payables D operating",
                &rem(Source::PayablesRemainder),
            ),
            (Minus, "liabilities E operating", &rem(Source::Accruals)),
        ],
    )?;
    let trade_rec = r.part(
        rec.is_some(),
        rec.as_ref().and_then(|n| n.trade),
        "receivables",
        "trade",
    );
    let trade_pay = r.part(
        pay.is_some(),
        pay.as_ref().and_then(|n| n.trade),
        "payables",
        "trade",
    );
    let twc = total(
        ex,
        &format!("Trade working capital ({year})"),
        &[
            (Plus, "trade receivables", &trade_rec),
            (Plus, "C.I", &c1),
            (Minus, "trade payables", &trade_pay),
        ],
    )?;
    let op_prov = total(
        ex,
        &format!("Operating provisions ({year})"),
        &[
            (Plus, "provisions B operating", &rem(Source::Provisions)),
            (
                Plus,
                "employee severance indemnity C operating",
                &rem(Source::EmployeeSeveranceIndemnity),
            ),
        ],
    )?;
    let ic = total(
        ex,
        &format!("Invested capital ({year})"),
        &[
            (Plus, "operating fixed assets", &ofa),
            (Plus, "operating working capital", &owc),
            (Minus, "operating provisions", &op_prov),
        ],
    )?;

    // Non-operating assets, outside invested capital (R12, R14).
    let mut noa_lines: Vec<(String, Partial)> = ffa
        .iter()
        .filter(|s| s.destination == Destination::NonOperatingAssets)
        .map(|s| (s.label.clone(), s.amount.clone()))
        .collect();
    noa_lines.extend(
        of_kind("non_operating_asset")
            .iter()
            .map(|m| (m.label.clone(), m.amount.clone())),
    );
    let noa_terms: Vec<(Sign, &str, &Partial)> = noa_lines
        .iter()
        .map(|(l, a)| (Plus, l.as_str(), a))
        .collect();
    let noa_total = total(ex, &format!("Non-operating assets ({year})"), &noa_terms)?;
    let noa = noa_total.value().map(|_| {
        noa_lines
            .iter()
            .map(|(label, a)| Line {
                label: label.clone(),
                value: a.value().unwrap_or_default(),
            })
            .collect::<Vec<_>>()
    });
    let tfi = total(
        ex,
        &format!("Total funds invested ({year})"),
        &[
            (Plus, "invested capital", &ic),
            (Plus, "non-operating assets", &noa_total),
        ],
    )?;

    // Sources.
    let equity = rem(Source::Equity);
    if let Some(v) = equity.value() {
        ex.step(
            format!("Equity ({year})"),
            format!(
                "liabilities A {} − subscribed capital unpaid {} − dividends declared {}",
                eq.value().unwrap_or_default(),
                a_total.value().unwrap_or_default(),
                Partial::plus_all(of_kind("dividends_declared").iter().map(|m| &m.amount))?
                    .value()
                    .unwrap_or_default()
            ),
            v,
        );
    }
    let q_lines: Vec<(String, Partial)> = sls
        .iter()
        .filter(|s| s.destination == Destination::QuasiEquityItems)
        .map(|s| (s.label.clone(), s.amount.clone()))
        .collect();
    let q_terms: Vec<(Sign, &str, &Partial)> =
        q_lines.iter().map(|(l, a)| (Plus, l.as_str(), a)).collect();
    let q_total = total(ex, &format!("Quasi-equity items ({year})"), &q_terms)?;
    let qe = q_total.value().map(|_| {
        q_lines
            .iter()
            .map(|(label, a)| Line {
                label: label.clone(),
                value: a.value().unwrap_or_default(),
            })
            .collect::<Vec<_>>()
    });
    let dl_moves: Vec<&Moved> = moved
        .iter()
        .filter(|m| {
            matches!(
                m.kind,
                "debt_like_provision" | "tax_instalments" | "dividends_declared"
            )
        })
        .collect();
    let dl_terms: Vec<(Sign, &str, &Partial)> = dl_moves
        .iter()
        .map(|m| (Plus, m.label.as_str(), &m.amount))
        .collect();
    let dl_total = total(ex, &format!("Debt-like items ({year})"), &dl_terms)?;
    let dl = dl_total.value().map(|_| {
        dl_moves
            .iter()
            .map(|m| BridgeClaim {
                label: m.label.clone(),
                value: m.amount.value().unwrap_or_default(),
                measurement_basis: MeasurementBasis::BookValue,
                basis_note: None,
            })
            .collect::<Vec<_>>()
    });

    // Net financial position (R13): signed components with their maturities.
    let mut comps: Vec<Component> = Vec::new();
    let fd_beyond = if fd.is_known() {
        Partial::of(fd_node.and_then(|n| n.beyond_one_year), || {
            r.p("payables/financial_debt/beyond_one_year")
        })
    } else {
        fd.clone()
    };
    comps.push(Component::split(
        "Financial debt (D)",
        r.p("payables/financial_debt"),
        &fd,
        &fd_beyond,
    )?);
    let sl_beyond = if sl.is_known() {
        Partial::of(sl_node.and_then(|n| n.beyond_one_year), || {
            r.p("payables/shareholder_loans/beyond_one_year")
        })
    } else {
        sl.clone()
    };
    for s in sls
        .iter()
        .filter(|s| s.destination == Destination::NetFinancialPositionGrossDebt)
    {
        let beyond = s.beyond.clone().unwrap_or_else(|| sl_beyond.clone());
        comps.push(Component::split(
            &s.label,
            r.p("payables/shareholder_loans"),
            &s.amount,
            &beyond,
        )?);
    }
    let cash_nfp = Partial::sum(&[(Plus, &c4), (Minus, &op_cash)])?;
    comps.push(Component::current(
        "Cash (C.IV)",
        r.p("cash"),
        &cash_nfp.negated()?,
    ));
    let mut other: Vec<(String, Partial)> = Vec::new();
    let neg_c3 = rem(Source::CurrentFinancialAssets).negated()?;
    other.push(("Current financial assets (C.III)".into(), neg_c3.clone()));
    comps.push(Component::current(
        "Current financial assets (C.III)",
        r.p("current_financial_assets"),
        &neg_c3,
    ));
    for m in of_kind("financial_receivable") {
        let neg = m.amount.negated()?;
        other.push((m.label.clone(), neg.clone()));
        let at = format!("/adjustments/{}", m.index);
        comps.push(match m.maturity {
            Some(Maturity::WithinOneYear) => Component::current(&m.label, at, &neg),
            _ => Component::non_current(&m.label, at, &neg),
        });
    }
    for s in ffa
        .iter()
        .filter(|s| s.destination == Destination::NetFinancialPositionOtherFinancialAsset)
    {
        let neg = s.amount.negated()?;
        other.push((s.label.clone(), neg.clone()));
        comps.push(Component::non_current(
            &s.label,
            r.p("financial_fixed_assets"),
            &neg,
        ));
    }
    for s in dshares
        .iter()
        .filter(|s| s.destination == Destination::NetFinancialPositionOtherFinancialLiability)
    {
        other.push((s.label.clone(), s.amount.clone()));
        comps.push(Component::non_current(
            &s.label,
            r.p("provisions/of_which_derivative_liabilities"),
            &s.amount,
        ));
    }
    for m in &moved {
        let at = format!("/adjustments/{}", m.index);
        match m.kind {
            "accrued_interest" => {
                other.push((m.label.clone(), m.amount.clone()));
                comps.push(Component::current(&m.label, at, &m.amount));
            }
            "non_current_trade_payables" => {
                other.push((m.label.clone(), m.amount.clone()));
                comps.push(Component::non_current(&m.label, at, &m.amount));
            }
            _ => {}
        }
    }
    let nfp_total = Partial::plus_all(comps.iter().map(|c| &c.amount))?;
    let nfp_current = Partial::plus_all(comps.iter().map(|c| &c.current))?;
    let nfp_non_current = Partial::plus_all(comps.iter().map(|c| &c.non_current))?;
    let gross = Partial::sum(&[
        (Plus, &fd),
        (
            Plus,
            &dest_sum(&sls, Destination::NetFinancialPositionGrossDebt)?,
        ),
    ])?;
    let (position, composition) = match nfp_total.value() {
        Some(expected) => {
            let position = NetFinancialPosition {
                gross_debt: gross.value().unwrap_or_default(),
                cash: cash_nfp.value().unwrap_or_default(),
                other_items: other
                    .iter()
                    .map(|(name, a)| NfpItem {
                        name: name.clone(),
                        amount: a.value().unwrap_or_default(),
                    })
                    .collect(),
            };
            // R13: the total is the SPEC-0007 function's, so AC8 holds by construction.
            if position.total(ex, year)? != expected {
                return Err(FairmarkError::Internal(
                    "statements: net financial position differs from its composition",
                ));
            }
            let composition = comps
                .iter()
                .map(|c| NfpComponent {
                    label: c.label.clone(),
                    source: c.source.clone(),
                    amount: c.amount.value().unwrap_or_default(),
                    current: c.current.value(),
                    non_current: c.non_current.value(),
                })
                .collect();
            (Some(position), Some(composition))
        }
        None => (None, None),
    };
    if let (Some(c), Some(n)) = (nfp_current.value(), nfp_non_current.value()) {
        ex.step(
            format!("Net financial position, current ({year})"),
            "Σ parts due within one year",
            c,
        );
        ex.step(
            format!("Net financial position, non-current ({year})"),
            "Σ parts due beyond one year",
            n,
        );
    }
    let total_sources = total(
        ex,
        &format!("Total sources ({year})"),
        &[
            (Plus, "equity", &equity),
            (Plus, "quasi-equity items", &q_total),
            (Plus, "net financial position", &nfp_total),
            (Plus, "debt-like items", &dl_total),
        ],
    )?;
    match (tfi.value(), total_sources.value()) {
        (Some(uses), Some(sources)) => {
            // R12: in the statutory form the identity follows from R7; a difference is a defect.
            if uses != sources {
                return Err(FairmarkError::Internal(
                    "statements: total funds invested differ from the sources",
                ));
            }
            checks.map.insert(
                "funds_invested_equals_sources",
                checks::Check {
                    status: checks::CheckStatus::Reconciled,
                    pointers: Vec::new(),
                },
            );
        }
        _ => checks.not_checked("funds_invested_equals_sources", Vec::new()),
    }
    let excess_cash = cash_nfp.clone();

    // Current and non-current (R15).
    let rec_b = r.part(
        rec.is_some(),
        rec.as_ref().and_then(|n| n.beyond_one_year),
        "receivables",
        "beyond_one_year",
    );
    let pay_b = r.part(
        pay.is_some(),
        pay.as_ref().and_then(|n| n.beyond_one_year),
        "payables",
        "beyond_one_year",
    );
    let d_b = r.beyond_or(bs.prepayments_and_accrued_income.as_ref(), &d_sep, false);
    let e_b = r.beyond_or(bs.accruals_and_deferred_income.as_ref(), &e_sep, false);
    let prov_nc = r.beyond_or(
        prov_node
            .as_ref()
            .map(|n| WithMaturity {
                total: n.total,
                beyond_one_year: n.beyond_one_year,
            })
            .as_ref(),
        &prov,
        true,
    );
    let tfr_nc = r.beyond_or(bs.employee_severance_indemnity.as_ref(), &tfr, true);
    let fixed = total(
        ex,
        &format!("Non-current assets ({year})"),
        &[
            (Plus, "B.I", &b1),
            (Plus, "B.II", &b2),
            (Plus, "B.III", &b3),
            (Plus, "C.II beyond one year", &rec_b),
            (Plus, "assets D beyond one year", &d_b),
        ],
    )?;
    let current = total(
        ex,
        &format!("Current assets ({year})"),
        &[
            (Plus, "C.I", &c1),
            (Plus, "C.II", &c2),
            (Minus, "C.II beyond one year", &rec_b),
            (Minus, "assets A within C.II", &a_within_v),
            (Plus, "C.III", &c3),
            (Plus, "C.IV", &c4),
            (Plus, "assets D", &d_sep),
            (Minus, "assets D beyond one year", &d_b),
        ],
    )?;
    let cur_liab = total(
        ex,
        &format!("Current liabilities ({year})"),
        &[
            (Plus, "provisions B", &prov),
            (Minus, "provisions B non-current", &prov_nc),
            (Plus, "employee severance indemnity C", &tfr),
            (Minus, "employee severance indemnity C non-current", &tfr_nc),
            (Plus, "payables D", &payables),
            (Minus, "payables D beyond one year", &pay_b),
            (Plus, "liabilities E", &e_sep),
            (Minus, "liabilities E beyond one year", &e_b),
        ],
    )?;
    let nc_liab = total(
        ex,
        &format!("Non-current liabilities ({year})"),
        &[
            (Plus, "provisions B non-current", &prov_nc),
            (Plus, "employee severance indemnity C non-current", &tfr_nc),
            (Plus, "payables D beyond one year", &pay_b),
            (Plus, "liabilities E beyond one year", &e_b),
        ],
    )?;
    let total_assets = total(
        ex,
        &format!("Total assets less subscribed capital unpaid ({year})"),
        &[
            (Plus, "non-current assets", &fixed),
            (Plus, "current assets", &current),
        ],
    )?;

    let mut out = Outputs::new(i, permits);
    let bsb = "balance_sheet";
    let cncb = "current_non_current";
    let balance_sheet = BalanceSheet {
        operating: OperatingBlock {
            operating_fixed_assets: out.amount(bsb, "operating/operating_fixed_assets", &ofa),
            operating_working_capital: out.amount(bsb, "operating/operating_working_capital", &owc),
            trade_working_capital: out.amount(bsb, "operating/trade_working_capital", &twc),
            operating_provisions: out.amount(bsb, "operating/operating_provisions", &op_prov),
            invested_capital: out.amount(bsb, "operating/invested_capital", &ic),
        },
        non_operating_assets: out.list(bsb, "non_operating_assets", noa, &noa_total),
        non_operating_assets_total: out.amount(bsb, "non_operating_assets_total", &noa_total),
        total_funds_invested: out.amount(bsb, "total_funds_invested", &tfi),
        sources: Sources {
            equity: out.amount(bsb, "sources/equity", &equity),
            quasi_equity_items: out.list(bsb, "sources/quasi_equity_items", qe, &q_total),
            quasi_equity_items_total: out.amount(bsb, "sources/quasi_equity_items_total", &q_total),
            net_financial_position: NetFinancialPositionBlock {
                position: out.list(
                    bsb,
                    "sources/net_financial_position/position",
                    position,
                    &nfp_total,
                ),
                total: out.amount(bsb, "sources/net_financial_position/total", &nfp_total),
                current: out.amount(bsb, "sources/net_financial_position/current", &nfp_current),
                non_current: out.amount(
                    bsb,
                    "sources/net_financial_position/non_current",
                    &nfp_non_current,
                ),
                composition: out.list(
                    bsb,
                    "sources/net_financial_position/composition",
                    composition,
                    &nfp_total,
                ),
            },
            debt_like_items: out.list(bsb, "sources/debt_like_items", dl, &dl_total),
            debt_like_items_total: out.amount(bsb, "sources/debt_like_items_total", &dl_total),
            total_sources: out.amount(bsb, "sources/total_sources", &total_sources),
        },
        excess_cash: out.amount(bsb, "excess_cash", &excess_cash),
    };
    let current_non_current = CurrentNonCurrent {
        fixed_assets: out.amount(cncb, "fixed_assets", &fixed),
        current_assets: out.amount(cncb, "current_assets", &current),
        inventories: out.amount(cncb, "inventories", &c1),
        cash: out.amount(cncb, "cash", &c4),
        total_assets: out.amount(cncb, "total_assets", &total_assets),
        current_liabilities: out.amount(cncb, "current_liabilities", &cur_liab),
        non_current_liabilities: out.amount(cncb, "non_current_liabilities", &nc_liab),
    };
    Ok(YearBody {
        balance_sheet,
        current_non_current,
        assignments,
        adjustments_applied: applied,
        checks: checks.map,
        not_computed: out.finish(),
    })
}

/// A component of the net financial position: its signed amount and maturities.
struct Component {
    label: String,
    source: String,
    amount: Partial,
    current: Partial,
    non_current: Partial,
}

impl Component {
    fn current(label: &str, source: String, amount: &Partial) -> Self {
        Self {
            label: label.to_owned(),
            source,
            amount: amount.clone(),
            current: amount.clone(),
            non_current: Partial::ZERO,
        }
    }

    fn non_current(label: &str, source: String, amount: &Partial) -> Self {
        Self {
            label: label.to_owned(),
            source,
            amount: amount.clone(),
            current: Partial::ZERO,
            non_current: amount.clone(),
        }
    }

    /// A loan split by its part beyond one year.
    fn split(label: &str, source: String, amount: &Partial, beyond: &Partial) -> Result<Self> {
        Ok(Self {
            label: label.to_owned(),
            source,
            amount: amount.clone(),
            current: Partial::sum(&[(Plus, amount), (Minus, beyond)])?,
            non_current: beyond.clone(),
        })
    }
}

/// Which source an adjustment takes from, where it goes and its maturity.
fn route(adj: &Adjustment) -> (Source, Destination, Option<Maturity>) {
    match adj {
        Adjustment::NonOperatingAsset(a) => (
            match a.aggregate {
                NonOperatingSource::IntangibleAssets => Source::IntangibleAssets,
                NonOperatingSource::TangibleAssets => Source::TangibleAssets,
                NonOperatingSource::Receivables => Source::Receivables,
                NonOperatingSource::CurrentFinancialAssets => Source::CurrentFinancialAssets,
            },
            Destination::NonOperatingAssets,
            None,
        ),
        Adjustment::FinancialReceivable(a) => (
            Source::Receivables,
            Destination::NetFinancialPositionOtherFinancialAsset,
            Some(a.maturity),
        ),
        Adjustment::OperatingCash(_) => (Source::Cash, Destination::OperatingWorkingCapital, None),
        Adjustment::AccruedInterest(_) => (
            Source::Accruals,
            Destination::NetFinancialPositionOtherFinancialLiability,
            Some(Maturity::WithinOneYear),
        ),
        Adjustment::TaxInstalments(a) => (
            Source::PayablesRemainder,
            Destination::DebtLikeItems,
            Some(a.maturity),
        ),
        Adjustment::NonCurrentTradePayables(_) => (
            Source::PayablesRemainder,
            Destination::NetFinancialPositionOtherFinancialLiability,
            Some(Maturity::BeyondOneYear),
        ),
        Adjustment::DebtLikeProvision(a) => (
            match a.aggregate {
                ProvisionSource::Provisions => Source::Provisions,
                ProvisionSource::EmployeeSeveranceIndemnity => Source::EmployeeSeveranceIndemnity,
            },
            Destination::DebtLikeItems,
            None,
        ),
        // Income statement kinds are refused before the computation in this build; dividends
        // declared, and only they, remain.
        _ => (Source::Equity, Destination::DebtLikeItems, None),
    }
}

fn ffa_destination(d: Ffa) -> Destination {
    match d {
        Ffa::Operating => Destination::OperatingFixedAssets,
        Ffa::NonOperating => Destination::NonOperatingAssets,
        Ffa::FinancialAsset => Destination::NetFinancialPositionOtherFinancialAsset,
    }
}

fn sl_destination(d: Sl) -> Destination {
    match d {
        Sl::FinancialDebt => Destination::NetFinancialPositionGrossDebt,
        Sl::QuasiEquity => Destination::QuasiEquityItems,
    }
}

fn deriv_destination(d: Deriv) -> Destination {
    match d {
        Deriv::FinancialLiability => Destination::NetFinancialPositionOtherFinancialLiability,
        Deriv::OperatingProvision => Destination::OperatingProvisions,
    }
}

/// The conventions of a statutory year that depend on what was given (R2, R8, R15).
fn statutory_conventions(
    bs: &StatutoryBalanceSheet,
    _is: &StatutoryIncomeStatement,
    r: &Reader,
    year: &str,
    ex: &mut Explain,
) {
    let mut absent = Vec::new();
    if let Some(n) = &bs.receivables {
        if n.subscribed_capital_unpaid.is_none() {
            absent.push(r.p("receivables/subscribed_capital_unpaid"));
        }
        if n.prepayments_and_accrued_income.is_none() {
            absent.push(r.p("receivables/prepayments_and_accrued_income"));
        }
    }
    if let Some(n) = &bs.provisions {
        if n.of_which_derivative_liabilities.is_none() {
            absent.push(r.p("provisions/of_which_derivative_liabilities"));
        }
    }
    if let Some(n) = &bs.payables {
        if n.accruals_and_deferred_income.is_none() {
            absent.push(r.p("payables/accruals_and_deferred_income"));
        }
    }
    if !absent.is_empty() {
        let list: Vec<String> = absent.iter().map(|p| format!("`{p}`")).collect();
        ex.convention(format!(
            "year {year}: \"of which\" parts not given, taken as none within their aggregate \
             (each is a separate item of the ordinary layout): {}",
            list.join(", ")
        ));
    }
    for (node, what, current) in [
        (
            bs.provisions.as_ref().map(|n| n.beyond_one_year),
            "provisions (B)",
            false,
        ),
        (
            bs.employee_severance_indemnity
                .as_ref()
                .map(|n| n.beyond_one_year),
            "employee severance indemnity (C)",
            false,
        ),
        (
            bs.prepayments_and_accrued_income
                .as_ref()
                .map(|n| n.beyond_one_year),
            "prepayments and accrued income (assets D)",
            true,
        ),
        (
            bs.accruals_and_deferred_income
                .as_ref()
                .map(|n| n.beyond_one_year),
            "accruals and deferred income (liabilities E)",
            true,
        ),
    ] {
        if let Some(None) = node {
            let reading = if current { "current" } else { "non-current" };
            ex.convention(format!(
                "year {year}: {what} read as {reading}: the part beyond one year is not given \
                 (R15)"
            ));
        }
    }
}

/// The outputs of one year as values, and every missing one listed in `not_computed` with the
/// pointers of the missing inputs and, for a missing treatment, its permitted destinations
/// (R6, R8).
pub(crate) struct Outputs {
    i: usize,
    permits: BTreeMap<String, Vec<&'static str>>,
    balance_sheet: Vec<super::NotComputedEntry>,
    current_non_current: Vec<super::NotComputedEntry>,
}

impl Outputs {
    pub(crate) fn new(i: usize, permits: BTreeMap<String, Vec<&'static str>>) -> Self {
        Self {
            i,
            permits,
            balance_sheet: Vec::new(),
            current_non_current: Vec::new(),
        }
    }

    fn note(&mut self, block: &str, at: &str, missing: &Partial) {
        let output = format!("/years/{}/{block}/{at}", self.i);
        let missing = missing.missing();
        let permitted = missing
            .iter()
            .filter_map(|p| {
                self.permits.get(p).map(|d| super::Permitted {
                    treatment: p.clone(),
                    destinations: d.clone(),
                })
            })
            .collect();
        let entry = super::NotComputedEntry {
            output,
            missing: missing.into_iter().collect(),
            permitted,
        };
        match block {
            "current_non_current" => self.current_non_current.push(entry),
            _ => self.balance_sheet.push(entry),
        }
    }

    pub(crate) fn amount(&mut self, block: &str, at: &str, p: &Partial) -> Option<Decimal> {
        if !p.is_known() {
            self.note(block, at, p);
        }
        p.value()
    }

    pub(crate) fn list<T>(
        &mut self,
        block: &str,
        at: &str,
        value: Option<T>,
        why: &Partial,
    ) -> Option<T> {
        if value.is_none() {
            self.note(block, at, why);
        }
        value
    }

    pub(crate) fn finish(mut self) -> super::NotComputed {
        self.balance_sheet.sort_by(|a, b| a.output.cmp(&b.output));
        self.current_non_current
            .sort_by(|a, b| a.output.cmp(&b.output));
        super::NotComputed {
            balance_sheet: self.balance_sheet,
            current_non_current: self.current_non_current,
        }
    }
}
