//! The income statement by nature reorganised (Appendix A.2 and B.2; R16 to R21): each aggregate
//! assigned to its layer, the `investing`, `non_recurring` and `lease` adjustments applied in the
//! order given, then the layers of Appendix B.2 with the subtotals aligned with IFRS 18
//! (`operating_profit`, `profit_before_financing_and_taxes`), the write-down rule of Decision 17,
//! and the reconciliations of both subtotals with A − B (R19). The reclassified form takes the
//! totals as given and derives what Appendix B.2 allows (R5).

use std::collections::BTreeMap;

use rust_decimal::Decimal;

use super::balance_sheet::Outputs;
use super::checks::{Check, Checks, Side};
use super::input::{
    Adjustment, InvestingSource, NonRecurringSource, ReorganiseInput, StatutoryIncomeStatement,
    WriteDownsOfReceivables,
};
use super::partial::{total, Partial, Sign};
use super::{
    AppliedAdjustment, Assignment, Destination, IncomeStatement, NotComputedEntry, Notes, Origin,
    Reconciliation, ReconciliationLine, Reconciliations,
};
use crate::error::{FairmarkError, Result};
use crate::explain::Explain;

use Sign::{Minus, Plus};

/// EBITDA in words (R18), usable as `ebitda_definition` of SPEC-0007.
pub(crate) const EBITDA_DEFINITION: &str = "EBITDA (margine operativo lordo, MOL) = value added \
    − personnel costs (B9): value of production (A1 to A5, core) less raw materials, services, \
    use of third-party assets, change in raw material inventories and other operating expenses \
    (B6, B7, B8, B11, B14, core) and personnel costs (B9); before depreciation, amortisation and \
    write-downs (B10), provisions (B12, B13), the investing and financing results and the \
    non-recurring items (SPEC-0005 Appendix B.2)";

/// An aggregate of Appendix A.2 that an adjustment may move (Appendix C).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Agg {
    Revenue,
    OtherRevenueAndIncome,
    Purchases,
    Services,
    UseOfThirdPartyAssets,
    PersonnelCosts,
    DepreciationAndAmortisation,
    WriteDownsOfFixedAssets,
    WriteDownsOfReceivables,
    ProvisionsForRisks,
    OtherProvisions,
    OtherOperatingExpenses,
    IncomeFromEquityInvestments,
    OtherFinancialIncome,
    InterestAndOtherFinancialCharges,
    ExchangeGainsAndLosses,
}

impl Agg {
    const ALL: [Agg; 16] = [
        Agg::Revenue,
        Agg::OtherRevenueAndIncome,
        Agg::Purchases,
        Agg::Services,
        Agg::UseOfThirdPartyAssets,
        Agg::PersonnelCosts,
        Agg::DepreciationAndAmortisation,
        Agg::WriteDownsOfFixedAssets,
        Agg::WriteDownsOfReceivables,
        Agg::ProvisionsForRisks,
        Agg::OtherProvisions,
        Agg::OtherOperatingExpenses,
        Agg::IncomeFromEquityInvestments,
        Agg::OtherFinancialIncome,
        Agg::InterestAndOtherFinancialCharges,
        Agg::ExchangeGainsAndLosses,
    ];

    fn key(self) -> &'static str {
        match self {
            Self::Revenue => "revenue",
            Self::OtherRevenueAndIncome => "other_revenue_and_income",
            Self::Purchases => "purchases",
            Self::Services => "services",
            Self::UseOfThirdPartyAssets => "use_of_third_party_assets",
            Self::PersonnelCosts => "personnel_costs",
            Self::DepreciationAndAmortisation => {
                "depreciation_and_write_downs/\
                                                  depreciation_and_amortisation"
            }
            Self::WriteDownsOfFixedAssets => {
                "depreciation_and_write_downs/\
                                              write_downs_of_fixed_assets"
            }
            Self::WriteDownsOfReceivables => {
                "depreciation_and_write_downs/\
                                              write_downs_of_receivables"
            }
            Self::ProvisionsForRisks => "provisions_for_risks",
            Self::OtherProvisions => "other_provisions",
            Self::OtherOperatingExpenses => "other_operating_expenses",
            Self::IncomeFromEquityInvestments => "income_from_equity_investments",
            Self::OtherFinancialIncome => "other_financial_income",
            Self::InterestAndOtherFinancialCharges => "interest_and_other_financial_charges",
            Self::ExchangeGainsAndLosses => "exchange_gains_and_losses",
        }
    }

    fn reference(self) -> &'static str {
        match self {
            Self::Revenue => "A1",
            Self::OtherRevenueAndIncome => "A5",
            Self::Purchases => "B6",
            Self::Services => "B7",
            Self::UseOfThirdPartyAssets => "B8",
            Self::PersonnelCosts => "B9",
            Self::DepreciationAndAmortisation => "B10 a, b",
            Self::WriteDownsOfFixedAssets => "B10 c",
            Self::WriteDownsOfReceivables => "B10 d",
            Self::ProvisionsForRisks => "B12",
            Self::OtherProvisions => "B13",
            Self::OtherOperatingExpenses => "B14",
            Self::IncomeFromEquityInvestments => "C15",
            Self::OtherFinancialIncome => "C16",
            Self::InterestAndOtherFinancialCharges => "C17",
            Self::ExchangeGainsAndLosses => "C17-bis",
        }
    }

    /// Income adds to profit; the other aggregates are charges.
    fn is_income(self) -> bool {
        matches!(
            self,
            Self::Revenue
                | Self::OtherRevenueAndIncome
                | Self::IncomeFromEquityInvestments
                | Self::OtherFinancialIncome
                | Self::ExchangeGainsAndLosses
        )
    }

    /// An aggregate of A or B, which enters A − B (R19).
    fn in_a_or_b(self) -> bool {
        !matches!(
            self,
            Self::IncomeFromEquityInvestments
                | Self::OtherFinancialIncome
                | Self::InterestAndOtherFinancialCharges
                | Self::ExchangeGainsAndLosses
        )
    }
}

impl From<NonRecurringSource> for Agg {
    fn from(s: NonRecurringSource) -> Self {
        use NonRecurringSource as N;
        match s {
            N::Revenue => Self::Revenue,
            N::OtherRevenueAndIncome => Self::OtherRevenueAndIncome,
            N::Purchases => Self::Purchases,
            N::Services => Self::Services,
            N::UseOfThirdPartyAssets => Self::UseOfThirdPartyAssets,
            N::PersonnelCosts => Self::PersonnelCosts,
            N::DepreciationAndAmortisation => Self::DepreciationAndAmortisation,
            N::WriteDownsOfFixedAssets => Self::WriteDownsOfFixedAssets,
            N::WriteDownsOfReceivables => Self::WriteDownsOfReceivables,
            N::ProvisionsForRisks => Self::ProvisionsForRisks,
            N::OtherProvisions => Self::OtherProvisions,
            N::OtherOperatingExpenses => Self::OtherOperatingExpenses,
            N::IncomeFromEquityInvestments => Self::IncomeFromEquityInvestments,
            N::OtherFinancialIncome => Self::OtherFinancialIncome,
            N::InterestAndOtherFinancialCharges => Self::InterestAndOtherFinancialCharges,
            N::ExchangeGainsAndLosses => Self::ExchangeGainsAndLosses,
        }
    }
}

impl From<InvestingSource> for Agg {
    fn from(s: InvestingSource) -> Self {
        match s {
            InvestingSource::OtherRevenueAndIncome => Self::OtherRevenueAndIncome,
            InvestingSource::OtherOperatingExpenses => Self::OtherOperatingExpenses,
        }
    }
}

/// B10 as read (Decision 8): B10 a and b, B10 c and B10 d, and whether B10 a to c was given as
/// one amount without its B10 c part (then depreciation and amortisation, B10 c none).
struct B10 {
    ab: Partial,
    c: Partial,
    d: Partial,
    grouped: bool,
}

fn b10(is: &StatutoryIncomeStatement, base: &str) -> Result<B10> {
    let at = format!("{base}/depreciation_and_write_downs");
    let Some(n) = &is.depreciation_and_write_downs else {
        let m = Partial::lacking(at);
        return Ok(B10 {
            ab: m.clone(),
            c: m.clone(),
            d: m,
            grouped: false,
        });
    };
    let grouped = n.depreciation_and_amortisation.is_none()
        && n.write_downs_of_fixed_assets.is_none()
        && n.value_adjustments_of_fixed_assets.is_some();
    let ab = match (
        n.depreciation_and_amortisation,
        n.value_adjustments_of_fixed_assets,
    ) {
        (Some(ab), _) => Partial::Known(ab),
        (None, Some(va)) => Partial::Known(crate::num::sub(
            va,
            n.write_downs_of_fixed_assets.unwrap_or_default(),
        )?),
        (None, None) => Partial::lacking(format!("{at}/depreciation_and_amortisation")),
    };
    let c = match n.write_downs_of_fixed_assets {
        Some(c) => Partial::Known(c),
        None if grouped => Partial::ZERO,
        None => Partial::lacking(format!("{at}/write_downs_of_fixed_assets")),
    };
    let d = Partial::of(n.write_downs_of_receivables, || {
        format!("{at}/write_downs_of_receivables")
    });
    Ok(B10 { ab, c, d, grouped })
}

/// The amount of an aggregate as given, or as read for B10 (missing with its pointer).
fn given(is: &StatutoryIncomeStatement, b10: &B10, agg: Agg, base: &str) -> Partial {
    let value = match agg {
        Agg::DepreciationAndAmortisation => return b10.ab.clone(),
        Agg::WriteDownsOfFixedAssets => return b10.c.clone(),
        Agg::WriteDownsOfReceivables => return b10.d.clone(),
        Agg::Revenue => is.revenue,
        Agg::OtherRevenueAndIncome => is.other_revenue_and_income,
        Agg::Purchases => is.purchases,
        Agg::Services => is.services,
        Agg::UseOfThirdPartyAssets => is.use_of_third_party_assets,
        Agg::PersonnelCosts => is.personnel_costs,
        Agg::ProvisionsForRisks => is.provisions_for_risks,
        Agg::OtherProvisions => is.other_provisions,
        Agg::OtherOperatingExpenses => is.other_operating_expenses,
        Agg::IncomeFromEquityInvestments => is.income_from_equity_investments,
        Agg::OtherFinancialIncome => is.other_financial_income,
        Agg::InterestAndOtherFinancialCharges => is.interest_and_other_financial_charges,
        Agg::ExchangeGainsAndLosses => is.exchange_gains_and_losses,
    };
    Partial::of(value, || format!("{base}/{}", agg.key()))
}

/// A − B of the statutory statement (R19): given, or from the totals A and B given, or from
/// their items given (the reading of the checks of R7).
fn a_minus_b(is: &StatutoryIncomeStatement, base: &str) -> Result<Partial> {
    if let Some(v) = is.difference_between_value_and_costs_of_production {
        return Ok(Partial::Known(v));
    }
    let leaf = |v: Option<Decimal>, k: &str| Partial::of(v, || format!("{base}/{k}"));
    let a = match is.value_of_production {
        Some(v) => Partial::Known(v),
        None => Partial::plus_all(&[
            leaf(is.revenue, "revenue"),
            leaf(
                is.change_in_product_inventories,
                "change_in_product_inventories",
            ),
            leaf(is.own_work_capitalised, "own_work_capitalised"),
            leaf(is.other_revenue_and_income, "other_revenue_and_income"),
        ])?,
    };
    let b = match is.costs_of_production {
        Some(v) => Partial::Known(v),
        None => {
            let nb = format!("{base}/depreciation_and_write_downs");
            let b10 = match &is.depreciation_and_write_downs {
                None => Partial::lacking(nb),
                Some(n) => match n.total {
                    Some(t) => Partial::Known(t),
                    None => {
                        let part =
                            |v: Option<Decimal>, k: &str| Partial::of(v, || format!("{nb}/{k}"));
                        let ac = match n.value_adjustments_of_fixed_assets {
                            Some(v) => Partial::Known(v),
                            None => Partial::plus_all(&[
                                part(
                                    n.depreciation_and_amortisation,
                                    "depreciation_and_amortisation",
                                ),
                                part(n.write_downs_of_fixed_assets, "write_downs_of_fixed_assets"),
                            ])?,
                        };
                        Partial::plus_all(&[
                            ac,
                            part(n.write_downs_of_receivables, "write_downs_of_receivables"),
                        ])?
                    }
                },
            };
            Partial::plus_all(&[
                leaf(is.purchases, "purchases"),
                leaf(is.services, "services"),
                leaf(is.use_of_third_party_assets, "use_of_third_party_assets"),
                leaf(is.personnel_costs, "personnel_costs"),
                b10,
                leaf(
                    is.change_in_raw_material_inventories,
                    "change_in_raw_material_inventories",
                ),
                leaf(is.provisions_for_risks, "provisions_for_risks"),
                leaf(is.other_provisions, "other_provisions"),
                leaf(is.other_operating_expenses, "other_operating_expenses"),
            ])?
        }
    };
    Partial::sum(&[(Plus, &a), (Minus, &b)])
}

/// One part moved out of an aggregate by an adjustment of the year.
struct Move {
    index: usize,
    label: String,
    agg: Agg,
    amount: Partial,
}

/// The income statement side of a `lease` adjustment for one year (R21).
struct LeaseLines {
    index: usize,
    label: String,
    payments: Partial,
    depreciation: Partial,
    interest: Partial,
}

/// What the income statement of one year adds to the year's output.
pub(crate) struct IncomeStatementBody {
    pub income_statement: IncomeStatement,
    pub assignments: Vec<Assignment>,
    pub adjustments_applied: Vec<AppliedAdjustment>,
    pub checks: BTreeMap<&'static str, Check>,
    pub not_computed: Vec<NotComputedEntry>,
}

/// The conventions of the income statement, stated once when a statutory year gives it.
pub(crate) fn conventions(input: &ReorganiseInput, ex: &mut Explain) {
    ex.convention(
        "income statement by nature reorganised into the layers of Appendix B.2 (R16): value of \
         production, value added, EBITDA, operating profit, investing result, profit before \
         financing and taxes, non-recurring result, financing result, profit before tax, income \
         taxes and net income; operating profit and profit before financing and taxes have the \
         content of the IFRS 18 subtotals \"operating profit or loss\" and \"profit or loss \
         before financing and income taxes\" (IFRS 18 for IFRS preparers, art. 2425 for OIC \
         preparers, both mapped to these subtotals)",
    );
    ex.convention(format!(
        "ebitda_definition: {EBITDA_DEFINITION} (Decision 8, R18)"
    ));
    ex.convention(
        "A5 and B14 are core by default (Decision 7, R18); investing and non-recurring parts \
         move only by labelled `investing` and `non_recurring` adjustments",
    );
    ex.convention(match input.treatments.write_downs_of_receivables {
        Some(WriteDownsOfReceivables::Operating) => {
            "write-downs and revaluations are extra-characteristic (Decision 17): D18, D19 and \
             B10 c in the investing result; B10 d in operating profit, below EBITDA, by the \
             treatment `write_downs_of_receivables: operating` (R17); EBITDA is the same either \
             way"
        }
        _ => {
            "write-downs and revaluations are extra-characteristic (Decision 17): D18, D19, B10 \
             c and B10 d in the investing result, below operating profit and above profit \
             before financing and taxes (R17); EBITDA is the same with the treatment \
             `write_downs_of_receivables: operating`"
        }
    });
    ex.convention(
        "C15 and C16 in the investing result; C17 and C17-bis in the financing result (IFRS \
         18); no adjustment moves D18 or D19 (R17)",
    );
    ex.convention(
        "income taxes: item 20 as given; no IRES, IRAP, deferred taxes or NOPAT are computed \
         (R19; SPEC-0003, SPEC-0001)",
    );
    ex.convention(
        "outputs named after a statutory item (revenue, purchases, materials_consumed, \
         personnel_costs, depreciation_and_amortisation, interest_and_other_financial_charges) \
         are the core amounts that enter the layers: less the parts moved by adjustments, with \
         lease depreciation and interest added (R21); cost_of_sales is not returned (Decision \
         13)",
    );
    ex.convention(
        "operating_profit feeds `ebit` of dcf.free_cash_flows and ROI on invested capital; \
         profit_before_financing_and_taxes feeds `ebit` of the interest covers of \
         ratios.debt_service and ROA on total assets (R20, Decision 18)",
    );
    if input
        .adjustments
        .iter()
        .any(|a| matches!(a, Adjustment::Lease(_)))
    {
        ex.convention(
            "leases restated by the financial method (R21, Decision 9; art. 2427 n. 22): asset \
             to operating fixed assets, liability to the net financial position with its \
             current part, prepayment out of assets D, asset − liability − prepayment to equity; \
             payments out of B8, depreciation and interest added; no deferred taxes; profit \
             before tax and net income therefore differ from the statutory amounts by payments \
             − depreciation − interest: payments and depreciation are lines of both \
             reconciliations with A − B, interest is in the financing result",
        );
    }
}

/// The reorganised income statement of a statutory year `i` (or of an entry with no form).
pub(crate) fn statutory(
    input: &ReorganiseInput,
    i: usize,
    year: &str,
    ex: &mut Explain,
    notes: &mut Notes,
) -> Result<IncomeStatementBody> {
    let is = input.years[i]
        .statutory
        .as_ref()
        .and_then(|s| s.income_statement.clone())
        .unwrap_or_default();
    let base = format!("/years/{i}/statutory/income_statement");
    let p = |k: &str| format!("{base}/{k}");
    let operating_wdr =
        input.treatments.write_downs_of_receivables == Some(WriteDownsOfReceivables::Operating);
    let b10 = b10(&is, &base)?;
    if b10.grouped {
        ex.convention(format!(
            "year {year}: B10 a to c given as one amount without its B10 c part: read as \
             depreciation and amortisation, B10 c none (Decision 8, R17)"
        ));
    }
    let checks = Checks::new(year);

    // Assignments (R11): every aggregate given, with its layer and origin.
    let mut assignments = Vec::new();
    {
        let mut assign = |reference: &'static str, key: &str, amount: &Partial, d: Destination| {
            let origin = if d == Destination::OperatingProfit && reference == "B10 d" {
                Origin::Analyst
            } else {
                Origin::Mapping
            };
            if let Some(v) = amount.value() {
                ex.step(
                    format!(
                        "Assignment ({year}): {reference} to {} ({})",
                        d.id(),
                        origin.id()
                    ),
                    format!("`{}` = {v}", p(key)),
                    v,
                );
                assignments.push(Assignment {
                    reference,
                    aggregate: p(key),
                    amount: v,
                    destination: d,
                    origin,
                });
            }
        };
        let k = |v: Option<Decimal>| v.map_or(Partial::unavailable(), Partial::Known);
        use Destination as Dn;
        assign("A1", "revenue", &k(is.revenue), Dn::ValueOfProduction);
        assign(
            "A2, A3",
            "change_in_product_inventories",
            &k(is.change_in_product_inventories),
            Dn::ValueOfProduction,
        );
        assign(
            "A4",
            "own_work_capitalised",
            &k(is.own_work_capitalised),
            Dn::ValueOfProduction,
        );
        assign(
            "A5",
            "other_revenue_and_income",
            &k(is.other_revenue_and_income),
            Dn::ValueOfProduction,
        );
        assign("B6", "purchases", &k(is.purchases), Dn::ValueAdded);
        assign("B7", "services", &k(is.services), Dn::ValueAdded);
        assign(
            "B8",
            "use_of_third_party_assets",
            &k(is.use_of_third_party_assets),
            Dn::ValueAdded,
        );
        assign("B9", "personnel_costs", &k(is.personnel_costs), Dn::Ebitda);
        let n = is.depreciation_and_write_downs.clone().unwrap_or_default();
        match (
            n.depreciation_and_amortisation,
            n.value_adjustments_of_fixed_assets,
        ) {
            (Some(ab), _) => assign(
                "B10 a, b",
                "depreciation_and_write_downs/depreciation_and_amortisation",
                &Partial::Known(ab),
                Dn::OperatingProfit,
            ),
            (None, Some(_)) => assign(
                if b10.grouped {
                    "B10 a to c, grouped"
                } else {
                    "B10 a, b (B10 a to c less B10 c)"
                },
                "depreciation_and_write_downs/value_adjustments_of_fixed_assets",
                &b10.ab,
                Dn::OperatingProfit,
            ),
            (None, None) => {}
        }
        assign(
            "B10 c",
            "depreciation_and_write_downs/write_downs_of_fixed_assets",
            &k(n.write_downs_of_fixed_assets),
            Dn::InvestingResult,
        );
        assign(
            "B10 d",
            "depreciation_and_write_downs/write_downs_of_receivables",
            &k(n.write_downs_of_receivables),
            if operating_wdr {
                Dn::OperatingProfit
            } else {
                Dn::InvestingResult
            },
        );
        assign(
            "B11",
            "change_in_raw_material_inventories",
            &k(is.change_in_raw_material_inventories),
            Dn::ValueAdded,
        );
        assign(
            "B12",
            "provisions_for_risks",
            &k(is.provisions_for_risks),
            Dn::OperatingProfit,
        );
        assign(
            "B13",
            "other_provisions",
            &k(is.other_provisions),
            Dn::OperatingProfit,
        );
        assign(
            "B14",
            "other_operating_expenses",
            &k(is.other_operating_expenses),
            Dn::ValueAdded,
        );
        assign(
            "C15",
            "income_from_equity_investments",
            &k(is.income_from_equity_investments),
            Dn::InvestingResult,
        );
        assign(
            "C16",
            "other_financial_income",
            &k(is.other_financial_income),
            Dn::InvestingResult,
        );
        assign(
            "C17",
            "interest_and_other_financial_charges",
            &k(is.interest_and_other_financial_charges),
            Dn::FinancingResult,
        );
        assign(
            "C17-bis",
            "exchange_gains_and_losses",
            &k(is.exchange_gains_and_losses),
            Dn::FinancingResult,
        );
        if is.revaluations_of_financial_assets.is_none()
            && is.write_downs_of_financial_assets.is_none()
        {
            assign(
                "total D",
                "value_adjustments_of_financial_assets",
                &k(is.value_adjustments_of_financial_assets),
                Dn::InvestingResult,
            );
        }
        assign(
            "D18",
            "revaluations_of_financial_assets",
            &k(is.revaluations_of_financial_assets),
            Dn::InvestingResult,
        );
        assign(
            "D19",
            "write_downs_of_financial_assets",
            &k(is.write_downs_of_financial_assets),
            Dn::InvestingResult,
        );
        assign("20", "income_taxes", &k(is.income_taxes), Dn::IncomeTaxes);
    }

    // Adjustments of Appendix C, in the order given (R10): what remains of each aggregate.
    let mut remaining: BTreeMap<Agg, Partial> = Agg::ALL
        .iter()
        .map(|a| (*a, given(&is, &b10, *a, &base)))
        .collect();
    let mut investing: Vec<Move> = Vec::new();
    let mut non_recurring: Vec<Move> = Vec::new();
    let mut leases: Vec<LeaseLines> = Vec::new();
    let mut applied = Vec::new();
    for (k, adj) in input.adjustments.iter().enumerate() {
        let (agg, amount, at, target) = match adj {
            Adjustment::Investing(a) => match a.amounts.get(i).copied().flatten() {
                Some(v) => (
                    Agg::from(a.aggregate),
                    v,
                    format!("/adjustments/{k}/amounts/{i}"),
                    Destination::InvestingResult,
                ),
                None => continue,
            },
            Adjustment::NonRecurring(a) => match a.amounts.get(i).copied().flatten() {
                Some(v) => (
                    Agg::from(a.aggregate),
                    v,
                    format!("/adjustments/{k}/amounts/{i}"),
                    Destination::NonRecurringResult,
                ),
                None => continue,
            },
            Adjustment::Lease(a) => match a.amounts.get(i).cloned().flatten() {
                Some(la) => (
                    Agg::UseOfThirdPartyAssets,
                    la.payments,
                    format!("/adjustments/{k}/amounts/{i}/payments"),
                    Destination::RemovedByRestatement,
                ),
                None => continue,
            },
            _ => continue,
        };
        let source = p(agg.key());
        let rem = remaining[&agg].clone();
        let took = match rem.value() {
            Some(left) => {
                if amount > left && !amount.is_zero() {
                    return Err(checks.error(
                        &Side::new(
                            format!("adjustment {k} '{}'", adj.label()),
                            vec![at],
                            Some(amount),
                        ),
                        amount,
                        &Side::new("what remains in its source", vec![source], Some(left)),
                        left,
                        "an adjustment moves at most what remains in its source after the \
                         adjustments listed before it (R10): lower the amount or check the \
                         source",
                    )?);
                }
                remaining.insert(agg, Partial::Known(crate::num::sub(left, amount)?));
                let line = matches!(adj, Adjustment::Lease(_)).then_some("payments");
                ex.step(
                    format!(
                        "Adjustment {k} {} '{}' ({year}): {} {} to {}",
                        adj.kind(),
                        adj.label(),
                        agg.reference(),
                        line.unwrap_or("part"),
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
                    source: source.clone(),
                    target,
                    line,
                });
                Partial::Known(amount)
            }
            None => {
                notes.convention(format!(
                    "adjustment {k} '{}' not applied to {year}: its source `{source}` is \
                     missing; the outputs it reaches are not computed",
                    adj.label()
                ));
                rem
            }
        };
        let label = adj.label().to_owned();
        match adj {
            Adjustment::Investing(_) => investing.push(Move {
                index: k,
                label,
                agg,
                amount: took,
            }),
            Adjustment::NonRecurring(_) => non_recurring.push(Move {
                index: k,
                label,
                agg,
                amount: took,
            }),
            Adjustment::Lease(a) => {
                let la = a.amounts[i].clone().ok_or(FairmarkError::Internal(
                    "statements: lease amounts of the year",
                ))?;
                let at = format!("/adjustments/{k}/amounts/{i}");
                for (line, v, target) in [
                    (
                        "depreciation",
                        la.depreciation,
                        Destination::OperatingProfit,
                    ),
                    ("interest", la.interest, Destination::FinancingResult),
                ] {
                    ex.step(
                        format!(
                            "Adjustment {k} lease '{}' ({year}): {line} to {}",
                            adj.label(),
                            target.id()
                        ),
                        format!("as given; reason: {}", adj.reason()),
                        v,
                    );
                    applied.push(AppliedAdjustment {
                        index: k,
                        kind: adj.kind(),
                        label: adj.label().to_owned(),
                        amount: v,
                        reason: adj.reason().to_owned(),
                        source: format!("{at}/{line}"),
                        target,
                        line: Some(line),
                    });
                }
                leases.push(LeaseLines {
                    index: k,
                    label,
                    payments: took,
                    depreciation: Partial::Known(la.depreciation),
                    interest: Partial::Known(la.interest),
                });
            }
            _ => {}
        }
    }
    let core = |a: Agg| remaining[&a].clone();
    let moved_from = |moves: &[Move], aggs: &[Agg]| -> Vec<(String, Partial)> {
        moves
            .iter()
            .filter(|m| aggs.contains(&m.agg))
            .map(|m| (m.label.clone(), m.amount.clone()))
            .collect()
    };
    let labelled = |sign: Sign, items: &[(String, Partial)]| -> Vec<(Sign, String, Partial)> {
        items
            .iter()
            .map(|(l, a)| (sign, l.clone(), a.clone()))
            .collect()
    };
    // A step with owned terms: `partial::total` with every term substituted.
    let step = |ex: &mut Explain, label: String, terms: &[(Sign, String, Partial)]| {
        let borrowed: Vec<(Sign, &str, &Partial)> =
            terms.iter().map(|(s, l, a)| (*s, l.as_str(), a)).collect();
        total(ex, &label, &borrowed)
    };
    let t = |sign: Sign, label: &str, a: &Partial| (sign, label.to_owned(), a.clone());
    let core_of = |ex: &mut Explain, agg: Agg, name: &str| -> Result<Partial> {
        // The core amount of an aggregate that adjustments moved parts out of, as a step.
        let moves: Vec<(String, Partial)> = investing
            .iter()
            .chain(&non_recurring)
            .filter(|m| m.agg == agg)
            .map(|m| (m.label.clone(), m.amount.clone()))
            .chain(
                leases
                    .iter()
                    .filter(|_| agg == Agg::UseOfThirdPartyAssets)
                    .map(|l| (format!("{} (lease payments)", l.label), l.payments.clone())),
            )
            .collect();
        if moves.is_empty() {
            return Ok(core(agg));
        }
        let mut terms = vec![t(Plus, agg.reference(), &given(&is, &b10, agg, &base))];
        terms.extend(labelled(Minus, &moves));
        step(ex, format!("{name}, core ({year})"), &terms)
    };

    // Layers of Appendix B.2.
    let revenue = core_of(ex, Agg::Revenue, "Revenue")?;
    let a5_inv = moved_from(&investing, &[Agg::OtherRevenueAndIncome]);
    let a_nr = moved_from(&non_recurring, &[Agg::Revenue, Agg::OtherRevenueAndIncome]);
    let mut vop_terms = match is.value_of_production {
        Some(v) => vec![t(Plus, "total A", &Partial::Known(v))],
        None => vec![
            t(Plus, "A1", &given(&is, &b10, Agg::Revenue, &base)),
            t(
                Plus,
                "A2 and A3",
                &Partial::of(is.change_in_product_inventories, || {
                    p("change_in_product_inventories")
                }),
            ),
            t(
                Plus,
                "A4",
                &Partial::of(is.own_work_capitalised, || p("own_work_capitalised")),
            ),
            t(
                Plus,
                "A5",
                &given(&is, &b10, Agg::OtherRevenueAndIncome, &base),
            ),
        ],
    };
    vop_terms.extend(labelled(Minus, &a5_inv));
    vop_terms.extend(labelled(Minus, &a_nr));
    let vop = step(ex, format!("Value of production ({year})"), &vop_terms)?;
    let purchases = core_of(ex, Agg::Purchases, "Purchases")?;
    let b11 = Partial::of(is.change_in_raw_material_inventories, || {
        p("change_in_raw_material_inventories")
    });
    let materials = step(
        ex,
        format!("Materials consumed ({year})"),
        &[t(Plus, "B6", &purchases), t(Plus, "B11", &b11)],
    )?;
    let services = core_of(ex, Agg::Services, "Services")?;
    let third = core_of(ex, Agg::UseOfThirdPartyAssets, "Use of third-party assets")?;
    let b14 = core_of(ex, Agg::OtherOperatingExpenses, "Other operating expenses")?;
    let value_added = step(
        ex,
        format!("Value added ({year})"),
        &[
            t(Plus, "value of production", &vop),
            t(Minus, "materials consumed (B6 + B11)", &materials),
            t(Minus, "B7", &services),
            t(Minus, "B8", &third),
            t(Minus, "B14", &b14),
        ],
    )?;
    let personnel = core_of(ex, Agg::PersonnelCosts, "Personnel costs")?;
    let ebitda = step(
        ex,
        format!("EBITDA ({year})"),
        &[
            t(Plus, "value added", &value_added),
            t(Minus, "B9", &personnel),
        ],
    )?;
    let ab_core = core_of(
        ex,
        Agg::DepreciationAndAmortisation,
        "Depreciation and amortisation (B10 a, b)",
    )?;
    let dna = if leases.is_empty() {
        ab_core
    } else {
        let mut terms = vec![t(Plus, "B10 a, b", &ab_core)];
        terms.extend(leases.iter().map(|l| {
            t(
                Plus,
                &format!("{} (lease depreciation)", l.label),
                &l.depreciation,
            )
        }));
        step(
            ex,
            format!("Depreciation and amortisation ({year})"),
            &terms,
        )?
    };
    let b12 = core_of(ex, Agg::ProvisionsForRisks, "Provisions for risks")?;
    let b13 = core_of(ex, Agg::OtherProvisions, "Other provisions")?;
    let c10 = core_of(
        ex,
        Agg::WriteDownsOfFixedAssets,
        "Write-downs of fixed assets (B10 c)",
    )?;
    let d10 = core_of(
        ex,
        Agg::WriteDownsOfReceivables,
        "Write-downs of receivables (B10 d)",
    )?;
    let mut op_terms = vec![
        t(Plus, "EBITDA", &ebitda),
        t(Minus, "depreciation and amortisation", &dna),
        t(Minus, "B12", &b12),
        t(Minus, "B13", &b13),
    ];
    if operating_wdr {
        op_terms.push(t(Minus, "B10 d (treatment operating)", &d10));
    }
    let operating_profit = step(ex, format!("Operating profit ({year})"), &op_terms)?;
    let d_total = match is.value_adjustments_of_financial_assets {
        Some(v) => Partial::Known(v),
        None => Partial::sum(&[
            (
                Plus,
                &Partial::of(is.revaluations_of_financial_assets, || {
                    p("revaluations_of_financial_assets")
                }),
            ),
            (
                Minus,
                &Partial::of(is.write_downs_of_financial_assets, || {
                    p("write_downs_of_financial_assets")
                }),
            ),
        ])?,
    };
    let c15 = core_of(
        ex,
        Agg::IncomeFromEquityInvestments,
        "Income from equity investments",
    )?;
    let c16 = core_of(ex, Agg::OtherFinancialIncome, "Other financial income")?;
    let b14_inv = moved_from(&investing, &[Agg::OtherOperatingExpenses]);
    let mut inv_terms = labelled(Plus, &a5_inv);
    inv_terms.extend(labelled(Minus, &b14_inv));
    inv_terms.extend([
        t(Plus, "C15", &c15),
        t(Plus, "C16", &c16),
        t(Plus, "D18 − D19", &d_total),
        t(Minus, "B10 c", &c10),
    ]);
    if !operating_wdr {
        inv_terms.push(t(Minus, "B10 d", &d10));
    }
    let investing_result = step(ex, format!("Investing result ({year})"), &inv_terms)?;
    let pbft = step(
        ex,
        format!("Profit before financing and taxes ({year})"),
        &[
            t(Plus, "operating profit", &operating_profit),
            t(Plus, "investing result", &investing_result),
        ],
    )?;
    let nr_terms: Vec<(Sign, String, Partial)> = non_recurring
        .iter()
        .map(|m| {
            (
                if m.agg.is_income() { Plus } else { Minus },
                format!("{} ({})", m.label, m.agg.reference()),
                m.amount.clone(),
            )
        })
        .collect();
    let nr_result = step(ex, format!("Non-recurring result ({year})"), &nr_terms)?;
    let c17_core = core_of(
        ex,
        Agg::InterestAndOtherFinancialCharges,
        "Interest and other financial charges (C17)",
    )?;
    let interest = if leases.is_empty() {
        c17_core
    } else {
        let mut terms = vec![t(Plus, "C17", &c17_core)];
        terms.extend(
            leases
                .iter()
                .map(|l| t(Plus, &format!("{} (lease interest)", l.label), &l.interest)),
        );
        step(
            ex,
            format!("Interest and other financial charges ({year})"),
            &terms,
        )?
    };
    let c17_bis = core_of(ex, Agg::ExchangeGainsAndLosses, "Exchange gains and losses")?;
    let financing = step(
        ex,
        format!("Financing result ({year})"),
        &[
            t(Plus, "C17-bis", &c17_bis),
            t(Minus, "interest and other financial charges", &interest),
        ],
    )?;
    let pbt = step(
        ex,
        format!("Profit before tax ({year})"),
        &[
            t(Plus, "profit before financing and taxes", &pbft),
            t(Plus, "non-recurring result", &nr_result),
            t(Plus, "financing result", &financing),
        ],
    )?;
    let taxes = Partial::of(is.income_taxes, || p("income_taxes"));
    let net_income = step(
        ex,
        format!("Net income ({year})"),
        &[
            t(Plus, "profit before tax", &pbt),
            t(Minus, "income taxes (item 20)", &taxes),
        ],
    )?;
    statutory_identity(&is, &base, &pbt, &net_income, &leases)?;

    // Reconciliations with A − B (R19).
    let a_b = a_minus_b(&is, &base)?;
    let mut nr_lines = Vec::new();
    for m in non_recurring.iter().filter(|m| m.agg.in_a_or_b()) {
        let sign = if m.agg.is_income() { Minus } else { Plus };
        nr_lines.push((
            m.index,
            format!("{} (non-recurring part of {})", m.label, m.agg.reference()),
            format!("/adjustments/{}", m.index),
            Partial::sum(&[(sign, &m.amount)])?,
        ));
    }
    let mut lease_lines = Vec::new();
    for l in &leases {
        lease_lines.push((
            l.index,
            format!("{} (lease payments removed from B8)", l.label),
            format!("/adjustments/{}", l.index),
            l.payments.clone(),
        ));
        lease_lines.push((
            l.index,
            format!("{} (lease depreciation)", l.label),
            format!("/adjustments/{}", l.index),
            l.depreciation.negated()?,
        ));
    }
    let mut op_lines = vec![(
        0,
        "B10 c, write-downs of fixed assets (investing result)".to_owned(),
        p(Agg::WriteDownsOfFixedAssets.key()),
        c10.clone(),
    )];
    if !operating_wdr {
        op_lines.push((
            0,
            "B10 d, write-downs of receivables (investing result)".to_owned(),
            p(Agg::WriteDownsOfReceivables.key()),
            d10.clone(),
        ));
    }
    let mut moves = Vec::new();
    for m in &investing {
        let sign = if m.agg.is_income() { Minus } else { Plus };
        moves.push((
            m.index,
            format!("{} (investing part of {})", m.label, m.agg.reference()),
            format!("/adjustments/{}", m.index),
            Partial::sum(&[(sign, &m.amount)])?,
        ));
    }
    moves.extend(nr_lines.iter().cloned());
    moves.extend(lease_lines.iter().cloned());
    moves.sort_by_key(|m| m.0);
    op_lines.extend(moves);
    let mut pbft_lines: Vec<(usize, String, String, Partial)> =
        nr_lines.into_iter().chain(lease_lines).collect();
    pbft_lines.sort_by_key(|m| m.0);
    pbft_lines.extend([
        (
            0,
            "C15, income from equity investments (investing result)".to_owned(),
            p("income_from_equity_investments"),
            c15.clone(),
        ),
        (
            0,
            "C16, other financial income (investing result)".to_owned(),
            p("other_financial_income"),
            c16.clone(),
        ),
        (
            0,
            "D18 − D19, value adjustments of financial assets (investing result)".to_owned(),
            if is.value_adjustments_of_financial_assets.is_some() {
                p("value_adjustments_of_financial_assets")
            } else {
                p("revaluations_of_financial_assets")
            },
            d_total.clone(),
        ),
    ]);
    let op_reconciliation = reconciliation(
        ex,
        &format!("Reconciliation of operating profit with A − B ({year})"),
        &a_b,
        &op_lines,
        &operating_profit,
    )?;
    let pbft_reconciliation = reconciliation(
        ex,
        &format!("Reconciliation of profit before financing and taxes with A − B ({year})"),
        &a_b,
        &pbft_lines,
        &pbft,
    )?;

    let mut out = Outputs::new(i, BTreeMap::new());
    let b = "income_statement";
    let income_statement = IncomeStatement {
        revenue: out.amount(b, "revenue", &revenue),
        value_of_production: out.amount(b, "value_of_production", &vop),
        purchases: out.amount(b, "purchases", &purchases),
        materials_consumed: out.amount(b, "materials_consumed", &materials),
        value_added: out.amount(b, "value_added", &value_added),
        personnel_costs: out.amount(b, "personnel_costs", &personnel),
        ebitda: out.amount(b, "ebitda", &ebitda),
        ebitda_definition: EBITDA_DEFINITION,
        depreciation_and_amortisation: out.amount(b, "depreciation_and_amortisation", &dna),
        operating_profit: out.amount(b, "operating_profit", &operating_profit),
        investing_result: out.amount(b, "investing_result", &investing_result),
        profit_before_financing_and_taxes: out.amount(
            b,
            "profit_before_financing_and_taxes",
            &pbft,
        ),
        non_recurring_result: out.amount(b, "non_recurring_result", &nr_result),
        financing_result: out.amount(b, "financing_result", &financing),
        interest_and_other_financial_charges: out.amount(
            b,
            "interest_and_other_financial_charges",
            &interest,
        ),
        profit_before_tax: out.amount(b, "profit_before_tax", &pbt),
        income_taxes: out.amount(b, "income_taxes", &taxes),
        net_income: out.amount(b, "net_income", &net_income),
        reconciliations: Reconciliations {
            operating_profit: out.list(
                b,
                "reconciliations/operating_profit",
                op_reconciliation.0,
                &op_reconciliation.1,
            ),
            profit_before_financing_and_taxes: out.list(
                b,
                "reconciliations/profit_before_financing_and_taxes",
                pbft_reconciliation.0,
                &pbft_reconciliation.1,
            ),
        },
    };
    Ok(IncomeStatementBody {
        income_statement,
        assignments,
        adjustments_applied: applied,
        checks: BTreeMap::new(),
        not_computed: out.finish().income_statement,
    })
}

/// R19: A − B + Σ `lines` = `end`, exactly, as a step; `None` with the missing pointers of
/// every side when a side is missing. A difference is a defect (`FM-E012`): the layers and the
/// lines are built from the same items.
fn reconciliation(
    ex: &mut Explain,
    label: &str,
    a_b: &Partial,
    lines: &[(usize, String, String, Partial)],
    end: &Partial,
) -> Result<(Option<Reconciliation>, Partial)> {
    let mut terms: Vec<(Sign, &str, &Partial)> = vec![(Plus, "A − B", a_b)];
    terms.extend(lines.iter().map(|(_, l, _, a)| (Plus, l.as_str(), a)));
    let signed: Vec<(Sign, &Partial)> = terms.iter().map(|(s, _, a)| (*s, *a)).collect();
    let summed = Partial::sum(&signed)?;
    let (Some(sum), Some(e)) = (summed.value(), end.value()) else {
        let mut all = signed.clone();
        all.push((Plus, end));
        return Ok((None, Partial::sum(&all)?));
    };
    if sum != e {
        return Err(FairmarkError::Internal(
            "statements: a reconciliation with A − B does not add up",
        ));
    }
    total(ex, label, &terms)?;
    Ok((
        Some(Reconciliation {
            start: a_b.value().unwrap_or_default(),
            lines: lines
                .iter()
                .map(|(_, label, source, a)| ReconciliationLine {
                    label: label.clone(),
                    source: source.clone(),
                    amount: a.value().unwrap_or_default(),
                })
                .collect(),
            end: e,
        }),
        end.clone(),
    ))
}

/// R19: the reorganised profit before tax equals the statutory A − B ± C ± D, and net income
/// item 21, when given, except for the lease restatements (payments − depreciation − interest).
/// The layers are built from the same items as the statutory totals, which R7 has checked, so a
/// difference is a defect (`FM-E012`).
fn statutory_identity(
    is: &StatutoryIncomeStatement,
    base: &str,
    pbt: &Partial,
    net_income: &Partial,
    leases: &[LeaseLines],
) -> Result<()> {
    let mut effect = Partial::ZERO;
    for l in leases {
        effect = Partial::sum(&[
            (Plus, &effect),
            (Plus, &l.payments),
            (Minus, &l.depreciation),
            (Minus, &l.interest),
        ])?;
    }
    let leaf = |v: Option<Decimal>, k: &str| Partial::of(v, || format!("{base}/{k}"));
    let c = match is.financial_income_and_charges {
        Some(v) => Partial::Known(v),
        None => Partial::sum(&[
            (
                Plus,
                &leaf(
                    is.income_from_equity_investments,
                    "income_from_equity_investments",
                ),
            ),
            (
                Plus,
                &leaf(is.other_financial_income, "other_financial_income"),
            ),
            (
                Minus,
                &leaf(
                    is.interest_and_other_financial_charges,
                    "interest_and_other_financial_charges",
                ),
            ),
            (
                Plus,
                &leaf(is.exchange_gains_and_losses, "exchange_gains_and_losses"),
            ),
        ])?,
    };
    let d = match is.value_adjustments_of_financial_assets {
        Some(v) => Partial::Known(v),
        None => Partial::sum(&[
            (
                Plus,
                &leaf(
                    is.revaluations_of_financial_assets,
                    "revaluations_of_financial_assets",
                ),
            ),
            (
                Minus,
                &leaf(
                    is.write_downs_of_financial_assets,
                    "write_downs_of_financial_assets",
                ),
            ),
        ])?,
    };
    let statutory_pbt = match is.profit_before_tax {
        Some(v) => Partial::Known(v),
        None => Partial::sum(&[(Plus, &a_minus_b(is, base)?), (Plus, &c), (Plus, &d)])?,
    };
    let restated = |p: &Partial| Partial::sum(&[(Plus, p), (Minus, &effect)]);
    let pairs = [
        (restated(pbt)?, statutory_pbt),
        (
            restated(net_income)?,
            is.profit_or_loss_for_the_year
                .map_or(Partial::unavailable(), Partial::Known),
        ),
    ];
    for (reorganised, statutory) in pairs {
        if let (Some(a), Some(b)) = (reorganised.value(), statutory.value()) {
            if a != b {
                return Err(FairmarkError::Internal(
                    "statements: the reorganised profit differs from the statutory one",
                ));
            }
        }
    }
    Ok(())
}

/// The reorganised income statement of a reclassified year `i` (R5): the totals as given;
/// EBITDA, profit before financing and taxes, profit before tax and net income derived by
/// Appendix B.2 when not given and checked exactly when given (`FM-E026`).
pub(crate) fn reclassified(
    input: &ReorganiseInput,
    i: usize,
    year: &str,
    ex: &mut Explain,
) -> Result<IncomeStatementBody> {
    let is = input.years[i]
        .reclassified
        .as_ref()
        .and_then(|r| r.income_statement.clone())
        .unwrap_or_default();
    let base = format!("/years/{i}/reclassified/income_statement");
    let p = |k: &str| format!("{base}/{k}");
    let f = |v: Option<Decimal>, k: &str| Partial::of(v, || p(k));
    let mut checks = Checks::new(year);
    ex.convention(format!(
        "year {year}: the reconciliations of the income statement with A − B need the statutory \
         form (R19): they are not computed for a reclassified entry"
    ));
    const FIX: &str = "correct the total or its components so that the reclassified income \
        statement adds up (Appendix B.2), or leave out the total to have it derived";
    let mut given_or_derived = |ex: &mut Explain,
                                relation: &'static str,
                                key: &str,
                                given: Option<Decimal>,
                                label: &str,
                                terms: &[(Sign, &str, &Partial)],
                                components: Vec<String>|
     -> Result<(Partial, Vec<String>)> {
        let derived = total(ex, &format!("{label} from its components ({year})"), terms)?;
        checks.equal(
            relation,
            Side::new(label.to_owned(), vec![p(key)], given),
            Side::new(
                format!("{label} derived by Appendix B.2"),
                components.clone(),
                derived.value(),
            ),
            FIX,
        )?;
        Ok(match given {
            Some(v) => (Partial::Known(v), vec![p(key)]),
            None if derived.is_known() => (derived, components),
            None => (Partial::lacking(p(key)), vec![p(key)]),
        })
    };
    let value_added = f(is.value_added, "value_added");
    let personnel = f(is.personnel_costs, "personnel_costs");
    let (ebitda, _) = given_or_derived(
        ex,
        "ebitda_total",
        "ebitda",
        is.ebitda,
        "EBITDA",
        &[
            (Plus, "value added", &value_added),
            (Minus, "personnel costs", &personnel),
        ],
        vec![p("value_added"), p("personnel_costs")],
    )?;
    let operating_profit = f(is.operating_profit, "operating_profit");
    let investing_result = f(is.investing_result, "investing_result");
    let (pbft, pbft_used) = given_or_derived(
        ex,
        "profit_before_financing_and_taxes_total",
        "profit_before_financing_and_taxes",
        is.profit_before_financing_and_taxes,
        "Profit before financing and taxes",
        &[
            (Plus, "operating profit", &operating_profit),
            (Plus, "investing result", &investing_result),
        ],
        vec![p("operating_profit"), p("investing_result")],
    )?;
    let nr = f(is.non_recurring_result, "non_recurring_result");
    let financing = f(is.financing_result, "financing_result");
    let mut pbt_components = pbft_used;
    pbt_components.extend([p("non_recurring_result"), p("financing_result")]);
    let (pbt, pbt_used) = given_or_derived(
        ex,
        "profit_before_tax_total",
        "profit_before_tax",
        is.profit_before_tax,
        "Profit before tax",
        &[
            (Plus, "profit before financing and taxes", &pbft),
            (Plus, "non-recurring result", &nr),
            (Plus, "financing result", &financing),
        ],
        pbt_components,
    )?;
    let taxes = f(is.income_taxes, "income_taxes");
    let mut net_components = pbt_used;
    net_components.push(p("income_taxes"));
    let (net_income, _) = given_or_derived(
        ex,
        "net_income_total",
        "net_income",
        is.net_income,
        "Net income",
        &[
            (Plus, "profit before tax", &pbt),
            (Minus, "income taxes", &taxes),
        ],
        net_components,
    )?;

    let statutory = Partial::unavailable();
    let mut out = Outputs::new(i, BTreeMap::new());
    let b = "income_statement";
    let income_statement = IncomeStatement {
        revenue: out.amount(b, "revenue", &f(is.revenue, "revenue")),
        value_of_production: out.amount(
            b,
            "value_of_production",
            &f(is.value_of_production, "value_of_production"),
        ),
        purchases: out.amount(b, "purchases", &f(is.purchases, "purchases")),
        materials_consumed: out.amount(
            b,
            "materials_consumed",
            &f(is.materials_consumed, "materials_consumed"),
        ),
        value_added: out.amount(b, "value_added", &value_added),
        personnel_costs: out.amount(b, "personnel_costs", &personnel),
        ebitda: out.amount(b, "ebitda", &ebitda),
        ebitda_definition: EBITDA_DEFINITION,
        depreciation_and_amortisation: out.amount(
            b,
            "depreciation_and_amortisation",
            &f(
                is.depreciation_and_amortisation,
                "depreciation_and_amortisation",
            ),
        ),
        operating_profit: out.amount(b, "operating_profit", &operating_profit),
        investing_result: out.amount(b, "investing_result", &investing_result),
        profit_before_financing_and_taxes: out.amount(
            b,
            "profit_before_financing_and_taxes",
            &pbft,
        ),
        non_recurring_result: out.amount(b, "non_recurring_result", &nr),
        financing_result: out.amount(b, "financing_result", &financing),
        interest_and_other_financial_charges: out.amount(
            b,
            "interest_and_other_financial_charges",
            &f(
                is.interest_and_other_financial_charges,
                "interest_and_other_financial_charges",
            ),
        ),
        profit_before_tax: out.amount(b, "profit_before_tax", &pbt),
        income_taxes: out.amount(b, "income_taxes", &taxes),
        net_income: out.amount(b, "net_income", &net_income),
        reconciliations: Reconciliations {
            operating_profit: out.list(
                b,
                "reconciliations/operating_profit",
                None::<Reconciliation>,
                &statutory,
            ),
            profit_before_financing_and_taxes: out.list(
                b,
                "reconciliations/profit_before_financing_and_taxes",
                None::<Reconciliation>,
                &statutory,
            ),
        },
    };
    Ok(IncomeStatementBody {
        income_statement,
        assignments: Vec::new(),
        adjustments_applied: Vec::new(),
        checks: checks.map,
        not_computed: out.finish().income_statement,
    })
}
