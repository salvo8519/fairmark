//! The income statement by nature reorganised (Appendix A.2 and B.2; R16 to R21; E1 to E7 of
//! 2026-11-09): each aggregate assigned to its layer, the `investing`, `non_recurring` and
//! `lease` adjustments applied in the order given, then the layers of Appendix B.2 with the
//! subtotals aligned with IFRS 18 (`operating_profit`, `profit_before_financing_and_taxes`), the
//! write-down rule of Decision 17, and the reconciliations of both subtotals with A − B (R19).
//! The reclassified form takes the totals as given and derives what Appendix B.2 allows (R5).
//!
//! The statutory computation runs in four steps, one function each: [`assignments`],
//! [`adjustments`], [`layers`] and [`reconciliations`].

use std::collections::BTreeMap;

use rust_decimal::Decimal;

use super::balance_sheet::Outputs;
use super::checks::{Check, CheckStatus, Checks, Side};
use super::input::{
    Adjustment, FinancialFixedAssetsDestination, FinancialFixedAssetsOverride, InvestingSource,
    NonRecurringSource, ProvisionSource, ReorganiseInput, StatutoryIncomeStatement,
    WriteDownsOfFixedAssets, WriteDownsOfReceivables,
};
use super::partial::{total, Partial, Sign};
use super::{
    AppliedAdjustment, Assignment, Destination, IncomeStatement, NotComputedEntry, Notes, Origin,
    Reconciliation, ReconciliationLine, Reconciliations, Stated,
};
use crate::error::{FairmarkError, Result};
use crate::explain::Explain;

use Sign::{Minus, Plus};

/// EBITDA in words (R18), usable as `ebitda_definition` of SPEC-0007.
pub(crate) const EBITDA_DEFINITION: &str = concat!(
    "EBITDA (margine operativo lordo, MOL) = value added − personnel costs (B9): value of ",
    "production (A1 to A5, core) less raw materials, services, use of third-party assets, ",
    "change in raw material inventories and other operating expenses (B6, B7, B8, B11, B14, ",
    "core) and personnel costs (B9); before depreciation, amortisation and write-downs (B10), ",
    "provisions (B12, B13), the investing and financing results and the non-recurring items ",
    "(SPEC-0005 Appendix B.2)"
);

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
                concat!(
                    "depreciation_and_write_downs/",
                    "depreciation_and_amortisation"
                )
            }
            Self::WriteDownsOfFixedAssets => {
                concat!(
                    "depreciation_and_write_downs/",
                    "write_downs_of_fixed_assets"
                )
            }
            Self::WriteDownsOfReceivables => {
                concat!(
                    "depreciation_and_write_downs/",
                    "write_downs_of_receivables"
                )
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

/// B10 as read (Decision 8, D1-IS): B10 a and b, B10 c and B10 d, whether B10 a to c was given
/// as one amount with neither part (then depreciation and amortisation, B10 c none), and the
/// pointers actually read for B10 a and b and for B10 c (the group and B10 a, b when B10 c is
/// their difference).
struct B10 {
    ab: Partial,
    c: Partial,
    d: Partial,
    grouped: bool,
    ab_read: Vec<String>,
    c_read: Vec<String>,
}

/// B10 as read. B10 c, or without it B10 a and b, within the group is a relation of R7, checked
/// before (`FM-E026`), so neither difference is negative here.
fn b10(is: &StatutoryIncomeStatement, base: &str) -> Result<B10> {
    let at = format!("{base}/depreciation_and_write_downs");
    let Some(n) = &is.depreciation_and_write_downs else {
        let m = Partial::lacking(at.clone());
        return Ok(B10 {
            ab: m.clone(),
            c: m.clone(),
            d: m,
            grouped: false,
            ab_read: vec![at.clone()],
            c_read: vec![at],
        });
    };
    let va_at = format!("{at}/value_adjustments_of_fixed_assets");
    let ab_at = format!("{at}/depreciation_and_amortisation");
    let c_at = format!("{at}/write_downs_of_fixed_assets");
    let grouped = n.depreciation_and_amortisation.is_none()
        && n.write_downs_of_fixed_assets.is_none()
        && n.value_adjustments_of_fixed_assets.is_some();
    let (ab, ab_read) = match (
        n.depreciation_and_amortisation,
        n.value_adjustments_of_fixed_assets,
    ) {
        (Some(ab), _) => (Partial::Known(ab), vec![ab_at.clone()]),
        (None, Some(va)) => {
            let c = n.write_downs_of_fixed_assets.unwrap_or_default();
            if c > va {
                return Err(FairmarkError::Internal(
                    "statements: B10 c exceeds its group after the checks",
                ));
            }
            let mut read = vec![va_at.clone()];
            if n.write_downs_of_fixed_assets.is_some() {
                read.push(c_at.clone());
            }
            (Partial::Known(crate::num::sub(va, c)?), read)
        }
        (None, None) => (Partial::lacking(ab_at.clone()), vec![ab_at.clone()]),
    };
    let (c, c_read) = match (
        n.write_downs_of_fixed_assets,
        n.value_adjustments_of_fixed_assets,
        n.depreciation_and_amortisation,
    ) {
        (Some(c), _, _) => (Partial::Known(c), vec![c_at]),
        (None, Some(va), Some(ab)) => {
            // D1-IS: the group less its B10 a, b part, within the group after the checks.
            if ab > va {
                return Err(FairmarkError::Internal(
                    "statements: B10 a, b exceed their group after the checks",
                ));
            }
            (Partial::Known(crate::num::sub(va, ab)?), vec![va_at, ab_at])
        }
        (None, Some(_), None) => (Partial::ZERO, vec![va_at]),
        (None, None, _) => (Partial::lacking(c_at.clone()), vec![c_at]),
    };
    let d = Partial::of(n.write_downs_of_receivables, || {
        format!("{at}/write_downs_of_receivables")
    });
    Ok(B10 {
        ab,
        c,
        d,
        grouped,
        ab_read,
        c_read,
    })
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

/// The items of one statutory year as read.
struct Year<'a> {
    /// Position of the year in `years`.
    index: usize,
    is: StatutoryIncomeStatement,
    base: String,
    b10: B10,
    name: &'a str,
    /// Whether the year gives an income statement at all (E6).
    given: bool,
    /// B10 d below EBITDA, in the other charges below EBITDA (treatment `operating`, E3).
    operating_wdr: bool,
    /// B10 c below EBITDA, with depreciation and amortisation (treatment `operating`, P1).
    operating_wdfa: bool,
    /// Which treatments the analyst stated, not filled in by default.
    stated: Stated,
    /// Where C15 goes, following financial fixed assets (B.III) of the year (P2).
    c15: C15Place,
}

/// Where C15 goes for one year, following the treatment of B.III (P2, A3, D2-IS).
#[derive(Debug, Clone, PartialEq, Eq)]
enum C15Place {
    /// B.III given, not zero and wholly `operating`: income from operating investments, in
    /// operating profit.
    Operating,
    /// B.III given, not zero and split between destinations: the investing result, reported.
    Split,
    /// The treatment says `operating` but B.III is not given, zero, or the year has no balance
    /// sheet: no operating investment to follow, the investing result, reported (D2-IS).
    NoHolding,
    /// B.III `non_operating` or `financial_asset`, or zero or not given without a treatment, or
    /// C15 given as zero: the investing result.
    Investing,
    /// B.III and C15 given and not zero, the treatment of B.III missing: C15 waits for it (R8,
    /// A3); the pointer of the treatment.
    Waits(String),
    /// B.III given and not zero, its treatment missing, C15 not given: C15 is missing with its
    /// own pointer wherever it could go.
    Unknown,
}

impl C15Place {
    /// P2: from the override of the year, else the treatment, and B.III and C15 as given.
    fn of(input: &ReorganiseInput, i: usize) -> Self {
        let y = &input.years[i];
        let st = y.statutory.as_ref();
        let held = st
            .and_then(|s| s.balance_sheet.as_ref())
            .and_then(|b| b.financial_fixed_assets)
            .is_some_and(|v| !v.is_zero());
        let over = y
            .treatment_overrides
            .as_ref()
            .and_then(|o| o.financial_fixed_assets.as_ref());
        let (destination, pointer) = match over {
            Some(FinancialFixedAssetsOverride::Split(_)) => {
                return if held { Self::Split } else { Self::Investing };
            }
            Some(FinancialFixedAssetsOverride::Destination(d)) => (
                Some(*d),
                format!("/years/{i}/treatment_overrides/financial_fixed_assets"),
            ),
            None => (
                input.treatments.financial_fixed_assets,
                "/treatments/financial_fixed_assets".to_owned(),
            ),
        };
        let c15 = st
            .and_then(|s| s.income_statement.as_ref())
            .and_then(|is| is.income_from_equity_investments);
        match (destination, held) {
            (Some(FinancialFixedAssetsDestination::Operating), true) => Self::Operating,
            (Some(FinancialFixedAssetsDestination::Operating), false) => Self::NoHolding,
            (Some(_), _) | (None, false) => Self::Investing,
            (None, true) => match c15 {
                None => Self::Unknown,
                Some(v) if v.is_zero() => Self::Investing,
                Some(_) => Self::Waits(pointer),
            },
        }
    }
}

impl Year<'_> {
    fn p(&self, key: &str) -> String {
        format!("{}/{key}", self.base)
    }

    fn leaf(&self, v: Option<Decimal>, key: &str) -> Partial {
        Partial::of(v, || self.p(key))
    }

    /// The amount of an aggregate as given, or as read for B10 (missing with its pointer).
    fn given(&self, agg: Agg) -> Partial {
        let is = &self.is;
        let value = match agg {
            Agg::DepreciationAndAmortisation => return self.b10.ab.clone(),
            Agg::WriteDownsOfFixedAssets => return self.b10.c.clone(),
            Agg::WriteDownsOfReceivables => return self.b10.d.clone(),
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
        self.leaf(value, agg.key())
    }

    /// The pointers actually read for an aggregate (review of PR 2, blocking 2).
    fn read(&self, agg: Agg) -> Vec<String> {
        match agg {
            Agg::DepreciationAndAmortisation => self.b10.ab_read.clone(),
            Agg::WriteDownsOfFixedAssets => self.b10.c_read.clone(),
            other => vec![self.p(other.key())],
        }
    }
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

/// The adjustments of one year, applied (R10).
struct Moves {
    /// What remains of each aggregate.
    remaining: BTreeMap<Agg, Partial>,
    investing: Vec<Move>,
    non_recurring: Vec<Move>,
    leases: Vec<LeaseLines>,
    /// Interest costs of a debt-like employee severance indemnity moved out of B9 (P2).
    interest_costs: Vec<Move>,
    applied: Vec<AppliedAdjustment>,
    /// The check of the interest costs within B9 (D3-IS), when any is given.
    checks: BTreeMap<&'static str, Check>,
}

/// D3-IS: the interest cost of the employee severance indemnity within what remains of B9.
const INTEREST_COST_CHECK: &str = "interest_cost_within_personnel_costs";

/// The layers of Appendix B.2 for one year.
struct Layers {
    revenue: Partial,
    value_of_production: Partial,
    purchases: Partial,
    materials_consumed: Partial,
    external_costs: Partial,
    value_added: Partial,
    personnel_costs: Partial,
    ebitda: Partial,
    depreciation_and_amortisation: Partial,
    other_charges_below_ebitda: Partial,
    income_from_operating_investments: Partial,
    operating_profit: Partial,
    write_downs_of_fixed_assets: Partial,
    write_downs_of_receivables: Partial,
    income_from_equity_investments: Partial,
    other_financial_income: Partial,
    value_adjustments_of_financial_assets: Partial,
    investing_result: Partial,
    profit_before_financing_and_taxes: Partial,
    non_recurring_result: Partial,
    interest: Partial,
    financing_result: Partial,
    profit_before_tax: Partial,
    income_taxes: Partial,
    net_income: Partial,
}

/// What the income statement of one year adds to the year's output.
pub(crate) struct IncomeStatementBody {
    pub income_statement: IncomeStatement,
    pub assignments: Vec<Assignment>,
    pub adjustments_applied: Vec<AppliedAdjustment>,
    pub checks: BTreeMap<&'static str, Check>,
    pub not_computed: Vec<NotComputedEntry>,
}

/// The conventions of the income statement, stated once: those of both forms when any year
/// gives an income statement, those of the statutory form when a statutory year gives one.
pub(crate) fn conventions(input: &ReorganiseInput, ex: &mut Explain) {
    let statutory = input.years.iter().any(|y| {
        y.statutory
            .as_ref()
            .is_some_and(|s| s.income_statement.is_some())
    });
    let reclassified = input.years.iter().any(|y| {
        y.reclassified
            .as_ref()
            .is_some_and(|r| r.income_statement.is_some())
    });
    if !(statutory || reclassified) {
        return;
    }
    ex.convention(
        "income statement by nature reorganised into the layers of Appendix B.2 (R16): value of \
         production, external costs, value added, EBITDA, other charges below EBITDA, operating \
         profit, investing result, profit before financing and taxes, non-recurring result, \
         financing result, profit before tax, income taxes and net income; operating profit and \
         profit before financing and taxes correspond to the IFRS 18 subtotals \"operating \
         profit or loss\" and \"profit or loss before financing and income taxes\" under the \
         conventions stated here (IFRS 18 for IFRS preparers, art. 2425 for OIC preparers, both \
         mapped to these subtotals)",
    );
    ex.convention(format!(
        "ebitda_definition: {EBITDA_DEFINITION} (Decision 8, R18)"
    ));
    ex.convention(
        "operating_profit feeds `ebit` of dcf.free_cash_flows and ROI on invested capital; \
         profit_before_financing_and_taxes feeds `ebit` of the interest covers of \
         ratios.debt_service and ROA on total assets (R20, Decision 18)",
    );
    if !statutory {
        return;
    }
    ex.convention(
        "A5 and B14 are core by default (Decision 7, R18); investing and non-recurring parts \
         move only by labelled `investing` and `non_recurring` adjustments",
    );
    ex.convention(
        "external_costs = B6 + B7 + B8 + B11 + B14 and other_charges_below_ebitda = B12 + B13 \
         (+ B10 d when operating), each net of the parts moved by adjustments, B8 also net of \
         lease payments; depreciation_and_amortisation = B10 a and b (+ B10 c when operating) \
         + lease depreciation (E1, E2, P1)",
    );
    ex.convention(
        "operating_profit has the content of the IFRS 18 subtotal \"operating profit or loss\" \
         only under the stated conventions: items labelled non-recurring are excluded, and the \
         write-downs follow their treatments (R16 as amended by P1)",
    );
    let place = |investing: bool, operating: &str| {
        if investing {
            "in the investing result (treatment `investing`)".to_owned()
        } else {
            format!("{operating} (treatment `operating`, the default)")
        }
    };
    ex.convention(format!(
        "write-downs and revaluations (R17; Decision 17 as amended by E3 and P1): D18 and D19 in \
         the investing result; B10 c {}; B10 d {}; EBITDA is the same with either treatment; a \
         one-off impairment is isolated with a `non_recurring` adjustment",
        place(
            input.treatments.write_downs_of_fixed_assets
                == Some(WriteDownsOfFixedAssets::Investing),
            "below EBITDA with depreciation and amortisation",
        ),
        place(
            input.treatments.write_downs_of_receivables == Some(WriteDownsOfReceivables::Investing),
            "below EBITDA in other_charges_below_ebitda",
        ),
    ));
    ex.convention(
        "C15 in the investing result, or in income_from_operating_investments when financial \
         fixed assets (B.III) are wholly operating for the year (P2); C16 in the investing \
         result; C17 and C17-bis in the financing result (IFRS 18), with the interest cost of a \
         debt-like employee severance indemnity when given (P2); no adjustment moves D18 or D19 \
         (R17); C16 includes the interest on cash and on financial receivables of the net \
         financial position, so the financing result is gross of that interest",
    );
    ex.convention(
        "C17-bis is a net amount: a `non_recurring` adjustment of C17-bis moves a part of a net \
         exchange gain; a non-recurring exchange loss cannot be expressed in this version (E7, \
         issue #251)",
    );
    ex.convention(
        "income taxes: item 20 as given; no IRES, IRAP, deferred taxes or NOPAT are computed \
         (R19; SPEC-0003, SPEC-0001)",
    );
    ex.convention(
        "outputs named after a statutory item (revenue, purchases, materials_consumed, \
         personnel_costs, depreciation_and_amortisation, interest_and_other_financial_charges) \
         are core amounts, net of the parts moved by adjustments, with lease depreciation and \
         interest added (E5, R21): purchases feeds days payable of ratios.working_capital as \
         core purchases; cost_of_sales is not returned (Decision 13)",
    );
    if input
        .adjustments
        .iter()
        .any(|a| matches!(a, Adjustment::Lease(_)))
    {
        ex.convention(
            "leases restated by the financial method (R21, Decision 9; OIC 12): asset to \
             operating fixed assets, liability to gross debt of the net financial position with \
             its current part (P3; ESMA32-382-1138 paragraph 180), prepayment out of assets D, asset − liability − prepayment to equity; \
             payments out of B8, depreciation and interest added; no deferred taxes; profit \
             before tax and net income equal the statutory amounts except for payments − \
             depreciation − interest (E4): payments and depreciation are lines of both \
             reconciliations with A − B, interest is in the financing result",
        );
    }
}

/// P2: where C15 goes for the year, and an interest cost of a debt-like employee severance
/// indemnity that is not given, stated as conventions.
fn c15_conventions(y: &Year, input: &ReorganiseInput, i: usize, ex: &mut Explain) {
    let name = y.name;
    match &y.c15 {
        C15Place::Operating => ex.convention(format!(
            "year {name}: financial fixed assets (B.III) are operating: C15, income from equity \
             investments, is income_from_operating_investments, below EBITDA and in operating \
             profit (P2)"
        )),
        C15Place::Split => ex.convention(format!(
            "year {name}: financial fixed assets (B.III) are split between destinations and the \
             income of each part is not given: C15 stays in the investing result (P2)"
        )),
        C15Place::Waits(pointer) => ex.convention(format!(
            "year {name}: C15 follows financial fixed assets (B.III), given without a \
             treatment: the outputs C15 reaches wait for `{pointer}` (P2, R8)"
        )),
        C15Place::NoHolding => ex.convention(format!(
            "year {name}: financial fixed assets (B.III) are treated as operating but are not \
             given or zero: there is no operating investment to follow, C15 stays in the \
             investing result and income_from_operating_investments is zero (P2, D2-IS)"
        )),
        C15Place::Investing | C15Place::Unknown => {}
    }
    for (k, adj) in input.adjustments.iter().enumerate() {
        if let Adjustment::DebtLikeProvision(a) = adj {
            let applies = a.amounts.get(i).copied().flatten().is_some();
            let cost = a
                .interest_cost
                .as_ref()
                .and_then(|c| c.get(i).copied().flatten());
            if let (true, Some(cost)) = (
                a.aggregate == ProvisionSource::EmployeeSeveranceIndemnity && applies,
                cost,
            ) {
                ex.convention(format!(
                    "year {name}: adjustment {k} '{}' moves the interest cost of the employee \
                     severance indemnity ({cost}) from B9 to the financing result: \
                     financing_result = C17-bis − interest_and_other_financial_charges − the \
                     interest cost, because interest_and_other_financial_charges stays C17 plus \
                     lease interest (P2)",
                    a.label
                ));
            }
            if a.aggregate == ProvisionSource::EmployeeSeveranceIndemnity
                && applies
                && cost.is_none()
            {
                ex.convention(format!(
                    "year {name}: adjustment {k} '{}' treats the employee severance indemnity \
                     as debt-like without its interest cost: the revaluation stays in personnel \
                     costs (B9) and EBITDA; give `/adjustments/{k}/interest_cost/{i}` to move it \
                     to the financing result (P2)",
                    a.label
                ));
            }
        }
    }
}

/// R11: every aggregate given, with its layer and origin.
fn assignments(y: &Year, ex: &mut Explain) -> Vec<Assignment> {
    use Destination as Dn;
    let mut out = Vec::new();
    let mut assign = |reference: &'static str, at: String, amount: &Partial, d: Dn, o: Origin| {
        if let Some(v) = amount.value() {
            ex.step(
                format!(
                    "Assignment ({}): {reference} to {} ({})",
                    y.name,
                    d.id(),
                    o.id()
                ),
                format!("`{at}` = {v}"),
                v,
            );
            out.push(Assignment {
                reference,
                aggregate: at,
                amount: v,
                destination: d,
                origin: o,
            });
        }
    };
    let is = &y.is;
    let k = |v: Option<Decimal>| v.map_or(Partial::unavailable(), Partial::Known);
    let m = Origin::Mapping;
    let flat: [(&'static str, &str, Option<Decimal>, Dn); 14] = [
        ("A1", "revenue", is.revenue, Dn::ValueOfProduction),
        (
            "A2, A3",
            "change_in_product_inventories",
            is.change_in_product_inventories,
            Dn::ValueOfProduction,
        ),
        (
            "A4",
            "own_work_capitalised",
            is.own_work_capitalised,
            Dn::ValueOfProduction,
        ),
        (
            "A5",
            "other_revenue_and_income",
            is.other_revenue_and_income,
            Dn::ValueOfProduction,
        ),
        ("B6", "purchases", is.purchases, Dn::ExternalCosts),
        ("B7", "services", is.services, Dn::ExternalCosts),
        (
            "B8",
            "use_of_third_party_assets",
            is.use_of_third_party_assets,
            Dn::ExternalCosts,
        ),
        ("B9", "personnel_costs", is.personnel_costs, Dn::Ebitda),
        (
            "B11",
            "change_in_raw_material_inventories",
            is.change_in_raw_material_inventories,
            Dn::ExternalCosts,
        ),
        (
            "B12",
            "provisions_for_risks",
            is.provisions_for_risks,
            Dn::OtherChargesBelowEbitda,
        ),
        (
            "B13",
            "other_provisions",
            is.other_provisions,
            Dn::OtherChargesBelowEbitda,
        ),
        (
            "B14",
            "other_operating_expenses",
            is.other_operating_expenses,
            Dn::ExternalCosts,
        ),
        (
            "C16",
            "other_financial_income",
            is.other_financial_income,
            Dn::InvestingResult,
        ),
        (
            "C17",
            "interest_and_other_financial_charges",
            is.interest_and_other_financial_charges,
            Dn::FinancingResult,
        ),
    ];
    for (reference, key, v, d) in flat {
        assign(reference, y.p(key), &k(v), d, m);
    }
    // P2: C15 follows B.III; no destination while its treatment is missing (R8).
    let c15 = match y.c15 {
        C15Place::Operating => Some((Dn::IncomeFromOperatingInvestments, Origin::Analyst)),
        C15Place::Split | C15Place::NoHolding | C15Place::Investing => {
            Some((Dn::InvestingResult, m))
        }
        C15Place::Waits(_) | C15Place::Unknown => None,
    };
    if let Some((d, o)) = c15 {
        assign(
            "C15",
            y.p("income_from_equity_investments"),
            &k(is.income_from_equity_investments),
            d,
            o,
        );
    }
    assign(
        "C17-bis",
        y.p("exchange_gains_and_losses"),
        &k(is.exchange_gains_and_losses),
        Dn::FinancingResult,
        m,
    );
    let n = is.depreciation_and_write_downs.clone().unwrap_or_default();
    let reference = match (
        n.depreciation_and_amortisation.is_some(),
        n.value_adjustments_of_fixed_assets.is_some(),
    ) {
        (true, _) => Some("B10 a, b"),
        (false, true) if y.b10.grouped => Some("B10 a to c, grouped"),
        (false, true) => Some("B10 a, b (B10 a to c less B10 c)"),
        (false, false) => None,
    };
    if let Some(reference) = reference {
        assign(
            reference,
            y.b10.ab_read[0].clone(),
            &y.b10.ab,
            Dn::DepreciationAndAmortisation,
            m,
        );
    }
    let b10 = |key: &str| y.p(&format!("depreciation_and_write_downs/{key}"));
    assign(
        "B10 c",
        b10("write_downs_of_fixed_assets"),
        &k(n.write_downs_of_fixed_assets),
        if y.operating_wdfa {
            Dn::DepreciationAndAmortisation
        } else {
            Dn::InvestingResult
        },
        if y.stated.write_downs_of_fixed_assets {
            Origin::Analyst
        } else {
            m
        },
    );
    assign(
        "B10 d",
        b10("write_downs_of_receivables"),
        &k(n.write_downs_of_receivables),
        if y.operating_wdr {
            Dn::OtherChargesBelowEbitda
        } else {
            Dn::InvestingResult
        },
        if y.stated.write_downs_of_receivables {
            Origin::Analyst
        } else {
            m
        },
    );
    if is.revaluations_of_financial_assets.is_none() && is.write_downs_of_financial_assets.is_none()
    {
        assign(
            "total D",
            y.p("value_adjustments_of_financial_assets"),
            &k(is.value_adjustments_of_financial_assets),
            Dn::InvestingResult,
            m,
        );
    }
    assign(
        "D18",
        y.p("revaluations_of_financial_assets"),
        &k(is.revaluations_of_financial_assets),
        Dn::InvestingResult,
        m,
    );
    assign(
        "D19",
        y.p("write_downs_of_financial_assets"),
        &k(is.write_downs_of_financial_assets),
        Dn::InvestingResult,
        m,
    );
    assign(
        "20",
        y.p("income_taxes"),
        &k(is.income_taxes),
        Dn::IncomeTaxes,
        m,
    );
    out
}

/// The `investing`, `non_recurring` and `lease` adjustments of year `i`, in the order given
/// (R10): each moves at most what remains in its source (`FM-E026` otherwise); a source missing
/// leaves the part moved missing, and the conventions say so.
fn adjustments(
    y: &Year,
    input: &ReorganiseInput,
    i: usize,
    ex: &mut Explain,
    notes: &mut Notes,
) -> Result<Moves> {
    let checks = Checks::new(y.name);
    let mut mv = Moves {
        remaining: Agg::ALL.iter().map(|a| (*a, y.given(*a))).collect(),
        investing: Vec::new(),
        non_recurring: Vec::new(),
        leases: Vec::new(),
        interest_costs: Vec::new(),
        applied: Vec::new(),
        checks: BTreeMap::new(),
    };
    for (k, adj) in input.adjustments.iter().enumerate() {
        let (agg, amount, at, target, related) = match adj {
            Adjustment::Investing(a) => match a.amounts.get(i).copied().flatten() {
                Some(v) => (
                    Agg::from(a.aggregate),
                    v,
                    format!("/adjustments/{k}/amounts/{i}"),
                    Destination::InvestingResult,
                    a.related_non_operating_asset.clone(),
                ),
                None => continue,
            },
            Adjustment::NonRecurring(a) => match a.amounts.get(i).copied().flatten() {
                Some(v) => (
                    Agg::from(a.aggregate),
                    v,
                    format!("/adjustments/{k}/amounts/{i}"),
                    Destination::NonRecurringResult,
                    None,
                ),
                None => continue,
            },
            Adjustment::Lease(a) => match a.amounts.get(i).cloned().flatten() {
                Some(la) => (
                    Agg::UseOfThirdPartyAssets,
                    la.payments,
                    format!("/adjustments/{k}/amounts/{i}/payments"),
                    Destination::RemovedByRestatement,
                    None,
                ),
                None => continue,
            },
            // P2: the interest cost of a debt-like severance indemnity leaves B9.
            Adjustment::DebtLikeProvision(a)
                if a.aggregate == ProvisionSource::EmployeeSeveranceIndemnity =>
            {
                match a
                    .interest_cost
                    .as_ref()
                    .and_then(|c| c.get(i).copied().flatten())
                {
                    Some(v) => (
                        Agg::PersonnelCosts,
                        v,
                        format!("/adjustments/{k}/interest_cost/{i}"),
                        Destination::FinancingResult,
                        None,
                    ),
                    None => continue,
                }
            }
            _ => continue,
        };
        let read = y.read(agg);
        let rem = mv.remaining[&agg].clone();
        let lease = matches!(adj, Adjustment::Lease(_));
        let interest_cost = matches!(adj, Adjustment::DebtLikeProvision(_));
        let left = rem.value();
        if let Some(left) = left {
            if amount > left && !amount.is_zero() {
                return Err(checks.error(
                    &Side::new(
                        format!("adjustment {k} '{}'", adj.label()),
                        vec![at],
                        Some(amount),
                    ),
                    amount,
                    &Side::new("what remains in its source", read, Some(left)),
                    left,
                    "an adjustment moves at most what remains in its source after the \
                     adjustments listed before it (R10): lower the amount or check the \
                     source",
                )?);
            }
        }
        if interest_cost {
            // D3-IS: only this check needs B9; the interest cost moves without it.
            let status = match (left, mv.checks.get(INTEREST_COST_CHECK)) {
                (_, Some(c)) if c.status == CheckStatus::NotChecked => CheckStatus::NotChecked,
                (Some(_), _) => CheckStatus::Reconciled,
                (None, _) => CheckStatus::NotChecked,
            };
            let mut pointers = mv
                .checks
                .get(INTEREST_COST_CHECK)
                .map(|c| c.pointers.clone())
                .unwrap_or_default();
            pointers.push(at.clone());
            for p in &read {
                if !pointers.contains(p) {
                    pointers.push(p.clone());
                }
            }
            mv.checks
                .insert(INTEREST_COST_CHECK, Check { status, pointers });
        }
        let took = if left.is_some() || interest_cost {
            match left {
                Some(left) => {
                    mv.remaining
                        .insert(agg, Partial::Known(crate::num::sub(left, amount)?));
                }
                None => notes.convention(format!(
                    "adjustment {k} '{}' ({}): interest cost moved to the financing result; \
                     `{}` is missing, so personnel costs and EBITDA are not computed and the \
                     check that the interest cost is within B9 is not checked (D3-IS)",
                    adj.label(),
                    y.name,
                    read[0]
                )),
            }
            let line = if lease {
                Some("payments")
            } else {
                interest_cost.then_some("interest_cost")
            };
            let mut formula = format!("as given; reason: {}", adj.reason());
            if let Some(r) = &related {
                formula.push_str(&format!("; related non-operating asset: {r}"));
            }
            ex.step(
                format!(
                    "Adjustment {k} {} '{}' ({}): {} {} to {}",
                    adj.kind(),
                    adj.label(),
                    y.name,
                    agg.reference(),
                    line.unwrap_or("part"),
                    target.id()
                ),
                formula,
                amount,
            );
            mv.applied.push(AppliedAdjustment {
                index: k,
                kind: adj.kind(),
                label: adj.label().to_owned(),
                amount,
                reason: adj.reason().to_owned(),
                source: read[0].clone(),
                target,
                line,
            });
            Partial::Known(amount)
        } else {
            notes.convention(if lease {
                format!(
                    "adjustment {k} '{}' ({}): lease payments not removed from B8: `{}` \
                     missing; value added and the layers below are not computed",
                    adj.label(),
                    y.name,
                    read[0]
                )
            } else {
                format!(
                    "adjustment {k} '{}' not applied to {}: its source `{}` is missing; the \
                     outputs it reaches are not computed",
                    adj.label(),
                    y.name,
                    read[0]
                )
            });
            rem
        };
        let label = adj.label().to_owned();
        match adj {
            Adjustment::Investing(_) => mv.investing.push(Move {
                index: k,
                label,
                agg,
                amount: took,
            }),
            Adjustment::NonRecurring(_) => mv.non_recurring.push(Move {
                index: k,
                label,
                agg,
                amount: took,
            }),
            Adjustment::DebtLikeProvision(_) => mv.interest_costs.push(Move {
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
                        Destination::DepreciationAndAmortisation,
                    ),
                    ("interest", la.interest, Destination::FinancingResult),
                ] {
                    ex.step(
                        format!(
                            "Adjustment {k} lease '{}' ({}): {line} to {}",
                            adj.label(),
                            y.name,
                            target.id()
                        ),
                        format!("as given; reason: {}", adj.reason()),
                        v,
                    );
                    mv.applied.push(AppliedAdjustment {
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
                mv.leases.push(LeaseLines {
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
    Ok(mv)
}

/// A term of a step, owned.
type Term = (Sign, String, Partial);

fn t(sign: Sign, label: &str, a: &Partial) -> Term {
    (sign, label.to_owned(), a.clone())
}

/// `partial::total` on owned terms: a step with every term substituted.
fn step(ex: &mut Explain, label: String, terms: &[Term]) -> Result<Partial> {
    let borrowed: Vec<(Sign, &str, &Partial)> =
        terms.iter().map(|(s, l, a)| (*s, l.as_str(), a)).collect();
    total(ex, &label, &borrowed)
}

impl Moves {
    fn core(&self, agg: Agg) -> Partial {
        self.remaining[&agg].clone()
    }

    fn moved(&self, from_investing: bool, aggs: &[Agg]) -> Vec<(String, Partial)> {
        let list = if from_investing {
            &self.investing
        } else {
            &self.non_recurring
        };
        list.iter()
            .filter(|m| aggs.contains(&m.agg))
            .map(|m| (m.label.clone(), m.amount.clone()))
            .collect()
    }

    /// The core amount of an aggregate (E5): as given, or, when adjustments moved parts out of
    /// it, a step "given − parts moved".
    fn core_step(&self, y: &Year, ex: &mut Explain, agg: Agg, name: &str) -> Result<Partial> {
        let moves: Vec<(String, Partial)> = self
            .investing
            .iter()
            .chain(&self.non_recurring)
            .filter(|m| m.agg == agg)
            .map(|m| (m.label.clone(), m.amount.clone()))
            .chain(
                self.interest_costs
                    .iter()
                    .filter(|m| m.agg == agg)
                    .map(|m| (format!("{} (interest cost)", m.label), m.amount.clone())),
            )
            .chain(
                self.leases
                    .iter()
                    .filter(|_| agg == Agg::UseOfThirdPartyAssets)
                    .map(|l| (format!("{} (lease payments)", l.label), l.payments.clone())),
            )
            .collect();
        if moves.is_empty() {
            return Ok(self.core(agg));
        }
        let mut terms = vec![t(Plus, agg.reference(), &y.given(agg))];
        terms.extend(moves.iter().map(|(l, a)| t(Minus, l, a)));
        step(ex, format!("{name}, core ({})", y.name), &terms)
    }
}

/// The layers of Appendix B.2 (R16 to R18, E1, E2, E6), each a step.
fn layers(y: &Year, mv: &Moves, ex: &mut Explain) -> Result<Layers> {
    let name = y.name;
    let is = &y.is;
    let revenue = mv.core_step(y, ex, Agg::Revenue, "Revenue")?;
    let a5_inv = mv.moved(true, &[Agg::OtherRevenueAndIncome]);
    let a_nr = mv.moved(false, &[Agg::Revenue, Agg::OtherRevenueAndIncome]);
    let mut vop_terms = match is.value_of_production {
        Some(v) => vec![t(Plus, "total A", &Partial::Known(v))],
        None => vec![
            t(Plus, "A1", &y.given(Agg::Revenue)),
            t(
                Plus,
                "A2 and A3",
                &y.leaf(
                    is.change_in_product_inventories,
                    "change_in_product_inventories",
                ),
            ),
            t(
                Plus,
                "A4",
                &y.leaf(is.own_work_capitalised, "own_work_capitalised"),
            ),
            t(Plus, "A5", &y.given(Agg::OtherRevenueAndIncome)),
        ],
    };
    vop_terms.extend(a5_inv.iter().map(|(l, a)| t(Minus, l, a)));
    vop_terms.extend(a_nr.iter().map(|(l, a)| t(Minus, l, a)));
    let value_of_production = step(ex, format!("Value of production ({name})"), &vop_terms)?;
    let purchases = mv.core_step(y, ex, Agg::Purchases, "Purchases")?;
    let b11 = y.leaf(
        is.change_in_raw_material_inventories,
        "change_in_raw_material_inventories",
    );
    let materials_consumed = step(
        ex,
        format!("Materials consumed ({name})"),
        &[t(Plus, "B6", &purchases), t(Plus, "B11", &b11)],
    )?;
    let services = mv.core_step(y, ex, Agg::Services, "Services")?;
    let third = mv.core_step(
        y,
        ex,
        Agg::UseOfThirdPartyAssets,
        "Use of third-party assets",
    )?;
    let b14 = mv.core_step(
        y,
        ex,
        Agg::OtherOperatingExpenses,
        "Other operating expenses",
    )?;
    let external_costs = step(
        ex,
        format!("External costs ({name})"),
        &[
            t(Plus, "materials consumed (B6 + B11)", &materials_consumed),
            t(Plus, "B7", &services),
            t(Plus, "B8", &third),
            t(Plus, "B14", &b14),
        ],
    )?;
    let value_added = step(
        ex,
        format!("Value added ({name})"),
        &[
            t(Plus, "value of production", &value_of_production),
            t(Minus, "external costs", &external_costs),
        ],
    )?;
    let personnel_costs = mv.core_step(y, ex, Agg::PersonnelCosts, "Personnel costs")?;
    let ebitda = step(
        ex,
        format!("EBITDA ({name})"),
        &[
            t(Plus, "value added", &value_added),
            t(Minus, "B9", &personnel_costs),
        ],
    )?;
    let ab = mv.core_step(
        y,
        ex,
        Agg::DepreciationAndAmortisation,
        "Depreciation and amortisation (B10 a, b)",
    )?;
    let c10 = mv.core_step(
        y,
        ex,
        Agg::WriteDownsOfFixedAssets,
        "Write-downs of fixed assets (B10 c)",
    )?;
    let depreciation_and_amortisation = if mv.leases.is_empty() && !y.operating_wdfa {
        ab
    } else {
        let mut terms = vec![t(Plus, "B10 a, b", &ab)];
        if y.operating_wdfa {
            // P1: B10 c with depreciation and amortisation, below EBITDA.
            terms.push(t(Plus, "B10 c (treatment operating)", &c10));
        }
        terms.extend(mv.leases.iter().map(|l| {
            t(
                Plus,
                &format!("{} (lease depreciation)", l.label),
                &l.depreciation,
            )
        }));
        step(
            ex,
            format!("Depreciation and amortisation ({name})"),
            &terms,
        )?
    };
    let b12 = mv.core_step(y, ex, Agg::ProvisionsForRisks, "Provisions for risks")?;
    let b13 = mv.core_step(y, ex, Agg::OtherProvisions, "Other provisions")?;
    let d10 = mv.core_step(
        y,
        ex,
        Agg::WriteDownsOfReceivables,
        "Write-downs of receivables (B10 d)",
    )?;
    let mut other_terms = vec![t(Plus, "B12", &b12), t(Plus, "B13", &b13)];
    if y.operating_wdr {
        other_terms.push(t(Plus, "B10 d (treatment operating)", &d10));
    }
    let other_charges_below_ebitda = step(
        ex,
        format!("Other charges below EBITDA ({name})"),
        &other_terms,
    )?;
    let c15 = mv.core_step(
        y,
        ex,
        Agg::IncomeFromEquityInvestments,
        "Income from equity investments",
    )?;
    // P2: C15 follows B.III; waiting for its treatment, it is missing wherever it could go.
    let c15 = match &y.c15 {
        C15Place::Waits(pointer) => {
            Partial::sum(&[(Plus, &c15), (Plus, &Partial::lacking(pointer.clone()))])?
        }
        _ => c15,
    };
    let income_from_operating_investments = match &y.c15 {
        C15Place::Operating => step(
            ex,
            format!("Income from operating investments ({name})"),
            &[t(Plus, "C15 (financial fixed assets operating, P2)", &c15)],
        )?,
        C15Place::Waits(_) | C15Place::Unknown => c15.clone(),
        C15Place::Split | C15Place::NoHolding | C15Place::Investing => Partial::ZERO,
    };
    let operating_profit = step(
        ex,
        format!("Operating profit ({name})"),
        &[
            t(Plus, "EBITDA", &ebitda),
            t(
                Minus,
                "depreciation and amortisation",
                &depreciation_and_amortisation,
            ),
            t(
                Minus,
                "other charges below EBITDA",
                &other_charges_below_ebitda,
            ),
            t(
                Plus,
                "income from operating investments",
                &income_from_operating_investments,
            ),
        ],
    )?;
    let value_adjustments_of_financial_assets = match is.value_adjustments_of_financial_assets {
        Some(v) => Partial::Known(v),
        None => Partial::sum(&[
            (
                Plus,
                &y.leaf(
                    is.revaluations_of_financial_assets,
                    "revaluations_of_financial_assets",
                ),
            ),
            (
                Minus,
                &y.leaf(
                    is.write_downs_of_financial_assets,
                    "write_downs_of_financial_assets",
                ),
            ),
        ])?,
    };
    let c16 = mv.core_step(y, ex, Agg::OtherFinancialIncome, "Other financial income")?;
    let b14_inv = mv.moved(true, &[Agg::OtherOperatingExpenses]);
    let mut inv_terms: Vec<Term> = a5_inv.iter().map(|(l, a)| t(Plus, l, a)).collect();
    inv_terms.extend(b14_inv.iter().map(|(l, a)| t(Minus, l, a)));
    if y.c15 != C15Place::Operating {
        inv_terms.push(t(Plus, "C15", &c15));
    }
    inv_terms.extend([
        t(Plus, "C16", &c16),
        t(Plus, "D18 − D19", &value_adjustments_of_financial_assets),
    ]);
    if !y.operating_wdfa {
        inv_terms.push(t(Minus, "B10 c", &c10));
    }
    if !y.operating_wdr {
        inv_terms.push(t(Minus, "B10 d", &d10));
    }
    let investing_result = step(ex, format!("Investing result ({name})"), &inv_terms)?;
    let profit_before_financing_and_taxes = step(
        ex,
        format!("Profit before financing and taxes ({name})"),
        &[
            t(Plus, "operating profit", &operating_profit),
            t(Plus, "investing result", &investing_result),
        ],
    )?;
    let nr_terms: Vec<Term> = mv
        .non_recurring
        .iter()
        .map(|m| {
            (
                if m.agg.is_income() { Plus } else { Minus },
                format!("{} ({})", m.label, m.agg.reference()),
                m.amount.clone(),
            )
        })
        .collect();
    let non_recurring_result = if y.given {
        if nr_terms.is_empty() {
            ex.convention(format!(
                "year {name}: no item labelled non-recurring: the non-recurring result is zero \
                 (E6)"
            ));
        }
        step(ex, format!("Non-recurring result ({name})"), &nr_terms)?
    } else {
        // E6: a year that gives no income statement has no non-recurring result.
        let borrowed: Vec<(Sign, &Partial)> = nr_terms.iter().map(|(s, _, a)| (*s, a)).collect();
        let nr = Partial::sum(&borrowed)?;
        Partial::sum(&[(Plus, &nr), (Plus, &Partial::lacking(y.base.clone()))])?
    };
    let c17 = mv.core_step(
        y,
        ex,
        Agg::InterestAndOtherFinancialCharges,
        "Interest and other financial charges (C17)",
    )?;
    let interest = if mv.leases.is_empty() {
        c17
    } else {
        let mut terms = vec![t(Plus, "C17", &c17)];
        terms.extend(
            mv.leases
                .iter()
                .map(|l| t(Plus, &format!("{} (lease interest)", l.label), &l.interest)),
        );
        step(
            ex,
            format!("Interest and other financial charges ({name})"),
            &terms,
        )?
    };
    let c17_bis = mv.core_step(
        y,
        ex,
        Agg::ExchangeGainsAndLosses,
        "Exchange gains and losses",
    )?;
    let mut fin_terms = vec![
        t(Plus, "C17-bis", &c17_bis),
        t(Minus, "interest and other financial charges", &interest),
    ];
    fin_terms.extend(mv.interest_costs.iter().map(|m| {
        t(
            Minus,
            &format!(
                "{} (interest cost of the employee severance indemnity, P2)",
                m.label
            ),
            &m.amount,
        )
    }));
    let financing_result = step(ex, format!("Financing result ({name})"), &fin_terms)?;
    let profit_before_tax = step(
        ex,
        format!("Profit before tax ({name})"),
        &[
            t(
                Plus,
                "profit before financing and taxes",
                &profit_before_financing_and_taxes,
            ),
            t(Plus, "non-recurring result", &non_recurring_result),
            t(Plus, "financing result", &financing_result),
        ],
    )?;
    let income_taxes = y.leaf(is.income_taxes, "income_taxes");
    let net_income = step(
        ex,
        format!("Net income ({name})"),
        &[
            t(Plus, "profit before tax", &profit_before_tax),
            t(Minus, "income taxes (item 20)", &income_taxes),
        ],
    )?;
    Ok(Layers {
        revenue,
        value_of_production,
        purchases,
        materials_consumed,
        external_costs,
        value_added,
        personnel_costs,
        ebitda,
        depreciation_and_amortisation,
        other_charges_below_ebitda,
        income_from_operating_investments,
        operating_profit,
        write_downs_of_fixed_assets: c10,
        write_downs_of_receivables: d10,
        income_from_equity_investments: c15,
        other_financial_income: c16,
        value_adjustments_of_financial_assets,
        investing_result,
        profit_before_financing_and_taxes,
        non_recurring_result,
        interest,
        financing_result,
        profit_before_tax,
        income_taxes,
        net_income,
    })
}

/// A line of a reconciliation: adjustment index (0 for an item), label, pointers read, amount.
type Line = (usize, String, Vec<String>, Partial);

/// R19: the reconciliations of operating profit and of profit before financing and taxes with
/// A − B, each `None` with its missing pointers when a side is missing.
fn reconciliations(
    y: &Year,
    mv: &Moves,
    l: &Layers,
    ex: &mut Explain,
) -> Result<[(Option<Reconciliation>, Partial); 2]> {
    let a_b = a_minus_b(&y.is, &y.base)?;
    let mut nr_lines: Vec<Line> = Vec::new();
    for m in mv.non_recurring.iter().filter(|m| m.agg.in_a_or_b()) {
        let sign = if m.agg.is_income() { Minus } else { Plus };
        nr_lines.push((
            m.index,
            format!("{} (non-recurring part of {})", m.label, m.agg.reference()),
            vec![format!("/adjustments/{}/amounts/{}", m.index, y.index)],
            Partial::sum(&[(sign, &m.amount)])?,
        ));
    }
    let mut lease_lines: Vec<Line> = Vec::new();
    for ls in &mv.leases {
        // The amounts actually read (review of round 2).
        let at = format!("/adjustments/{}/amounts/{}", ls.index, y.index);
        lease_lines.push((
            ls.index,
            format!("{} (lease payments removed from B8)", ls.label),
            vec![format!("{at}/payments")],
            ls.payments.clone(),
        ));
        lease_lines.push((
            ls.index,
            format!("{} (lease depreciation)", ls.label),
            vec![format!("{at}/depreciation")],
            ls.depreciation.negated()?,
        ));
    }
    for m in &mv.interest_costs {
        lease_lines.push((
            m.index,
            format!(
                "{} (interest cost of the employee severance indemnity removed from B9)",
                m.label
            ),
            vec![format!(
                "/adjustments/{}/interest_cost/{}",
                m.index, y.index
            )],
            m.amount.clone(),
        ));
    }
    let mut c_label = "B10 c, write-downs of fixed assets (investing result)".to_owned();
    if y.b10.grouped {
        c_label.push_str(": read as none, give the B10 c part (Decision 8)");
    }
    let mut op_lines: Vec<Line> = Vec::new();
    if !y.operating_wdfa {
        op_lines.push((
            0,
            c_label,
            y.read(Agg::WriteDownsOfFixedAssets),
            l.write_downs_of_fixed_assets.clone(),
        ));
    }
    if !y.operating_wdr {
        op_lines.push((
            0,
            "B10 d, write-downs of receivables (investing result)".to_owned(),
            y.read(Agg::WriteDownsOfReceivables),
            l.write_downs_of_receivables.clone(),
        ));
    }
    let c15_operating = y.c15 == C15Place::Operating;
    let c15_label = format!(
        "C15, income from equity investments ({})",
        if c15_operating {
            "operating investments, P2"
        } else {
            "investing result"
        }
    );
    if c15_operating {
        op_lines.push((
            0,
            c15_label.clone(),
            y.read(Agg::IncomeFromEquityInvestments),
            l.income_from_equity_investments.clone(),
        ));
    }
    let mut moves: Vec<Line> = Vec::new();
    for m in &mv.investing {
        let sign = if m.agg.is_income() { Minus } else { Plus };
        moves.push((
            m.index,
            format!("{} (investing part of {})", m.label, m.agg.reference()),
            vec![format!("/adjustments/{}/amounts/{}", m.index, y.index)],
            Partial::sum(&[(sign, &m.amount)])?,
        ));
    }
    moves.extend(nr_lines.iter().cloned());
    moves.extend(lease_lines.iter().cloned());
    moves.sort_by_key(|m| m.0);
    op_lines.extend(moves);
    let mut pbft_lines: Vec<Line> = nr_lines.into_iter().chain(lease_lines).collect();
    pbft_lines.sort_by_key(|m| m.0);
    let d_read = if y.is.value_adjustments_of_financial_assets.is_some() {
        vec![y.p("value_adjustments_of_financial_assets")]
    } else {
        vec![
            y.p("revaluations_of_financial_assets"),
            y.p("write_downs_of_financial_assets"),
        ]
    };
    pbft_lines.extend([
        (
            0,
            c15_label,
            y.read(Agg::IncomeFromEquityInvestments),
            l.income_from_equity_investments.clone(),
        ),
        (
            0,
            "C16, other financial income (investing result)".to_owned(),
            y.read(Agg::OtherFinancialIncome),
            l.other_financial_income.clone(),
        ),
        (
            0,
            "D18 − D19, value adjustments of financial assets (investing result)".to_owned(),
            d_read,
            l.value_adjustments_of_financial_assets.clone(),
        ),
    ]);
    Ok([
        reconciliation(
            ex,
            &format!("Reconciliation of operating profit with A − B ({})", y.name),
            &a_b,
            &op_lines,
            &l.operating_profit,
        )?,
        reconciliation(
            ex,
            &format!(
                "Reconciliation of profit before financing and taxes with A − B ({})",
                y.name
            ),
            &a_b,
            &pbft_lines,
            &l.profit_before_financing_and_taxes,
        )?,
    ])
}

/// The reorganised income statement of a statutory year `i` (or of an entry with no form).
/// `stated`: the treatments of the write-downs the analyst gave.
pub(crate) fn statutory(
    input: &ReorganiseInput,
    i: usize,
    name: &str,
    stated: Stated,
    ex: &mut Explain,
    notes: &mut Notes,
) -> Result<IncomeStatementBody> {
    let given = input.years[i]
        .statutory
        .as_ref()
        .and_then(|s| s.income_statement.clone());
    let base = format!("/years/{i}/statutory/income_statement");
    let is = given.clone().unwrap_or_default();
    let b10 = b10(&is, &base)?;
    let y = Year {
        b10,
        is,
        base,
        name,
        given: given.is_some(),
        operating_wdr: input.treatments.write_downs_of_receivables
            != Some(WriteDownsOfReceivables::Investing),
        operating_wdfa: input.treatments.write_downs_of_fixed_assets
            != Some(WriteDownsOfFixedAssets::Investing),
        stated,
        c15: C15Place::of(input, i),
        index: i,
    };
    if y.b10.grouped && !y.operating_wdfa {
        ex.convention(format!(
            "year {name}: B10 a to c given as one amount without its B10 c part: read as \
             depreciation and amortisation, B10 c none; give the B10 c part to have it in the \
             investing result (Decision 8, R17)"
        ));
    }
    if y.given {
        c15_conventions(&y, input, i, ex);
    }
    let assignments = assignments(&y, ex);
    let mv = adjustments(&y, input, i, ex, notes)?;
    let l = layers(&y, &mv, ex)?;
    statutory_identity(
        &y.is,
        &y.base,
        &l.profit_before_tax,
        &l.net_income,
        &mv.leases,
    )?;
    let [op, pbft] = reconciliations(&y, &mv, &l, ex)?;

    let mut permits = BTreeMap::new();
    if let C15Place::Waits(pointer) = &y.c15 {
        permits.insert(
            pointer.clone(),
            vec!["operating", "non_operating", "financial_asset"],
        );
    }
    let mut out = Outputs::new(i, permits);
    let b = "income_statement";
    let income_statement = IncomeStatement {
        revenue: out.amount(b, "revenue", &l.revenue),
        value_of_production: out.amount(b, "value_of_production", &l.value_of_production),
        purchases: out.amount(b, "purchases", &l.purchases),
        materials_consumed: out.amount(b, "materials_consumed", &l.materials_consumed),
        external_costs: out.amount(b, "external_costs", &l.external_costs),
        value_added: out.amount(b, "value_added", &l.value_added),
        personnel_costs: out.amount(b, "personnel_costs", &l.personnel_costs),
        ebitda: out.amount(b, "ebitda", &l.ebitda),
        ebitda_definition: EBITDA_DEFINITION,
        depreciation_and_amortisation: out.amount(
            b,
            "depreciation_and_amortisation",
            &l.depreciation_and_amortisation,
        ),
        other_charges_below_ebitda: out.amount(
            b,
            "other_charges_below_ebitda",
            &l.other_charges_below_ebitda,
        ),
        income_from_operating_investments: out.amount(
            b,
            "income_from_operating_investments",
            &l.income_from_operating_investments,
        ),
        operating_profit: out.amount(b, "operating_profit", &l.operating_profit),
        investing_result: out.amount(b, "investing_result", &l.investing_result),
        profit_before_financing_and_taxes: out.amount(
            b,
            "profit_before_financing_and_taxes",
            &l.profit_before_financing_and_taxes,
        ),
        non_recurring_result: out.amount(b, "non_recurring_result", &l.non_recurring_result),
        financing_result: out.amount(b, "financing_result", &l.financing_result),
        interest_and_other_financial_charges: out.amount(
            b,
            "interest_and_other_financial_charges",
            &l.interest,
        ),
        profit_before_tax: out.amount(b, "profit_before_tax", &l.profit_before_tax),
        income_taxes: out.amount(b, "income_taxes", &l.income_taxes),
        net_income: out.amount(b, "net_income", &l.net_income),
        reconciliations: Reconciliations {
            operating_profit: out.list(b, "reconciliations/operating_profit", op.0, &op.1),
            profit_before_financing_and_taxes: out.list(
                b,
                "reconciliations/profit_before_financing_and_taxes",
                pbft.0,
                &pbft.1,
            ),
        },
    };
    Ok(IncomeStatementBody {
        income_statement,
        assignments,
        adjustments_applied: mv.applied,
        checks: mv.checks,
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
    lines: &[Line],
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
                .map(|(_, label, sources, a)| ReconciliationLine {
                    label: label.clone(),
                    sources: sources.clone(),
                    amount: a.value().unwrap_or_default(),
                })
                .collect(),
            end: e,
        }),
        end.clone(),
    ))
}

/// R19, E4: the reorganised profit before tax equals the statutory A − B ± C ± D, and net
/// income item 21, when given, except for the lease restatements (payments − depreciation −
/// interest). The layers are built from the same items as the statutory totals, which R7 has
/// checked, so a difference is a defect (`FM-E012`).
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

/// The reorganised income statement of a reclassified year `i` (R5, E1, E2): the totals as
/// given; value added, EBITDA, operating profit, profit before financing and taxes, profit
/// before tax and net income derived by Appendix B.2 when not given and checked exactly when
/// given (`FM-E026`).
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
    let vop = f(is.value_of_production, "value_of_production");
    let external = f(is.external_costs, "external_costs");
    let (value_added, va_used) = given_or_derived(
        ex,
        "value_added_total",
        "value_added",
        is.value_added,
        "Value added",
        &[
            (Plus, "value of production", &vop),
            (Minus, "external costs", &external),
        ],
        vec![p("value_of_production"), p("external_costs")],
    )?;
    let personnel = f(is.personnel_costs, "personnel_costs");
    let mut ebitda_components = va_used;
    ebitda_components.push(p("personnel_costs"));
    let (ebitda, ebitda_used) = given_or_derived(
        ex,
        "ebitda_total",
        "ebitda",
        is.ebitda,
        "EBITDA",
        &[
            (Plus, "value added", &value_added),
            (Minus, "personnel costs", &personnel),
        ],
        ebitda_components,
    )?;
    let dna = f(
        is.depreciation_and_amortisation,
        "depreciation_and_amortisation",
    );
    let other = f(is.other_charges_below_ebitda, "other_charges_below_ebitda");
    let operating_investments = f(
        is.income_from_operating_investments,
        "income_from_operating_investments",
    );
    let mut op_components = ebitda_used;
    op_components.extend([
        p("depreciation_and_amortisation"),
        p("other_charges_below_ebitda"),
        p("income_from_operating_investments"),
    ]);
    let (operating_profit, op_used) = given_or_derived(
        ex,
        "operating_profit_total",
        "operating_profit",
        is.operating_profit,
        "Operating profit",
        &[
            (Plus, "EBITDA", &ebitda),
            (Minus, "depreciation and amortisation", &dna),
            (Minus, "other charges below EBITDA", &other),
            (
                Plus,
                "income from operating investments",
                &operating_investments,
            ),
        ],
        op_components,
    )?;
    let investing_result = f(is.investing_result, "investing_result");
    let mut pbft_components = op_used;
    pbft_components.push(p("investing_result"));
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
        pbft_components,
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
        value_of_production: out.amount(b, "value_of_production", &vop),
        purchases: out.amount(b, "purchases", &f(is.purchases, "purchases")),
        materials_consumed: out.amount(
            b,
            "materials_consumed",
            &f(is.materials_consumed, "materials_consumed"),
        ),
        external_costs: out.amount(b, "external_costs", &external),
        value_added: out.amount(b, "value_added", &value_added),
        personnel_costs: out.amount(b, "personnel_costs", &personnel),
        ebitda: out.amount(b, "ebitda", &ebitda),
        ebitda_definition: EBITDA_DEFINITION,
        depreciation_and_amortisation: out.amount(b, "depreciation_and_amortisation", &dna),
        other_charges_below_ebitda: out.amount(b, "other_charges_below_ebitda", &other),
        income_from_operating_investments: out.amount(
            b,
            "income_from_operating_investments",
            &operating_investments,
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
