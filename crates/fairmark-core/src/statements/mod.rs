//! Financial statements reorganised into an operating and financing view (SPEC-0005).
//!
//! One method, `statements.reorganise` version 1, preview (ADR 0012). It takes one or more
//! financial years, each in the `statutory` form (aggregates of art. 2424 and 2425) or in the
//! `reclassified` form (totals already reorganised), any subset of the totals, and returns per
//! year what they allow: best effort, with every missing output listed in `not_computed` with
//! the pointers of the missing inputs (R6), and every relation checked exactly when both sides
//! are given (R7).
//!
//! This build implements the balance sheet and the income statement (first and second pull
//! requests of the plan): the reorganised balance sheet in both forms, the net financial
//! position in the SPEC-0007 type, the bridge items in the `dcf.fcff` types, the current and
//! non-current block for SPEC-0007, and the reorganised income statement in both forms with the
//! subtotals aligned with IFRS 18 and its reconciliations with A − B. The changes between years
//! (R23) come with the third pull request.

mod balance_sheet;
mod checks;
mod income_statement;
pub mod input;
mod partial;
mod reclassified;
mod validate;

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::Serialize;

pub use checks::{Check, CheckStatus};
pub use input::*;

use crate::dates::Date;
use crate::dcf::BridgeClaim;
use crate::error::Result;
use crate::explain::{DefaultApplied, Explain};
use crate::method::Method;
use crate::ratios::{NetFinancialPosition, NFP_CONVENTION};
use crate::tv::Line;

/// The form a year was given in (R6: every output states its form).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Form {
    /// Aggregates of art. 2424 and 2425 (Appendix A).
    Statutory,
    /// Totals already reclassified (Appendix B).
    Reclassified,
}

/// Where an aggregate, or the part of it moved by an adjustment, goes (Appendix A, B, C).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Destination {
    /// Operating fixed assets, in invested capital.
    OperatingFixedAssets,
    /// Operating working capital, in invested capital.
    OperatingWorkingCapital,
    /// Operating provisions, deducted in invested capital.
    OperatingProvisions,
    /// Non-operating assets, outside invested capital, for the bridge.
    NonOperatingAssets,
    /// Gross debt of the net financial position.
    NetFinancialPositionGrossDebt,
    /// Cash of the net financial position.
    NetFinancialPositionCash,
    /// An other item of the net financial position, an asset (negative in it).
    NetFinancialPositionOtherFinancialAsset,
    /// An other item of the net financial position, a liability (positive in it).
    NetFinancialPositionOtherFinancialLiability,
    /// Quasi-equity items, among the sources next to equity.
    QuasiEquityItems,
    /// Debt-like items, for the bridge.
    DebtLikeItems,
    /// Equity.
    Equity,
    /// Deducted from equity (subscribed capital unpaid).
    DeductedFromEquity,
    /// Value of production, layer of the income statement (A1 to A5, core).
    ValueOfProduction,
    /// External costs, deducted in value added (B6, B7, B8, B11, B14, core; E1).
    ExternalCosts,
    /// Deducted in EBITDA (B9).
    Ebitda,
    /// Depreciation and amortisation, below EBITDA (B10 a and b, lease depreciation).
    DepreciationAndAmortisation,
    /// Other charges below EBITDA (B12, B13; B10 d with the treatment `operating`; E2).
    OtherChargesBelowEbitda,
    /// Investing result, below operating profit (A5 and B14 investing parts, C15, C16, D18,
    /// D19, B10 c, B10 d by default).
    InvestingResult,
    /// Non-recurring result (parts moved by `non_recurring` adjustments).
    NonRecurringResult,
    /// Financing result (C17, C17-bis).
    FinancingResult,
    /// Income taxes, item 20 as given.
    IncomeTaxes,
    /// Removed from its aggregate by the financial-method restatement of a lease and replaced
    /// by depreciation and interest (lease payments out of B8, R21).
    RemovedByRestatement,
}

impl Destination {
    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::OperatingFixedAssets => "operating_fixed_assets",
            Self::OperatingWorkingCapital => "operating_working_capital",
            Self::OperatingProvisions => "operating_provisions",
            Self::NonOperatingAssets => "non_operating_assets",
            Self::NetFinancialPositionGrossDebt => "net_financial_position_gross_debt",
            Self::NetFinancialPositionCash => "net_financial_position_cash",
            Self::NetFinancialPositionOtherFinancialAsset => {
                "net_financial_position_other_financial_asset"
            }
            Self::NetFinancialPositionOtherFinancialLiability => {
                "net_financial_position_other_financial_liability"
            }
            Self::QuasiEquityItems => "quasi_equity_items",
            Self::DebtLikeItems => "debt_like_items",
            Self::Equity => "equity",
            Self::DeductedFromEquity => "deducted_from_equity",
            Self::ValueOfProduction => "value_of_production",
            Self::ExternalCosts => "external_costs",
            Self::Ebitda => "ebitda",
            Self::DepreciationAndAmortisation => "depreciation_and_amortisation",
            Self::OtherChargesBelowEbitda => "other_charges_below_ebitda",
            Self::InvestingResult => "investing_result",
            Self::NonRecurringResult => "non_recurring_result",
            Self::FinancingResult => "financing_result",
            Self::IncomeTaxes => "income_taxes",
            Self::RemovedByRestatement => "removed_by_restatement",
        }
    }
}

/// Who chose a destination (R11).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    /// The fixed or default destination of Appendix A.
    Mapping,
    /// A treatment or an override stated by the analyst.
    Analyst,
}

impl Origin {
    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::Mapping => "mapping",
            Self::Analyst => "analyst",
        }
    }
}

/// One aggregate given, assigned to its destination before the adjustments (R11).
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Assignment {
    /// Statutory reference, for example "B.III" or "liabilities D, remainder".
    pub reference: &'static str,
    /// JSON pointer of the input aggregate.
    pub aggregate: String,
    /// Amount assigned, in currency units.
    pub amount: Decimal,
    /// Where the amount goes.
    pub destination: Destination,
    /// `mapping` (Appendix A) or `analyst` (treatment or override).
    pub origin: Origin,
}

/// One adjustment applied to a year (R11).
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct AppliedAdjustment {
    /// Position of the adjustment in `adjustments`.
    pub index: usize,
    /// The kind of Appendix C, for example `operating_cash`.
    pub kind: &'static str,
    /// The adjustment's label, as given.
    pub label: String,
    /// Amount moved, in currency units.
    pub amount: Decimal,
    /// The adjustment's reason, as given.
    pub reason: String,
    /// JSON pointer of the source aggregate.
    pub source: String,
    /// Where the amount goes.
    pub target: Destination,
    /// For a `lease` adjustment, which of its amounts this entry moves: `asset`, `liability`,
    /// `prepayment`, `equity_effect`, `payments`, `depreciation` or `interest`; omitted for
    /// the other kinds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<&'static str>,
}

/// The operating block (Appendix B.1).
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct OperatingBlock {
    /// Operating fixed assets, in currency units.
    pub operating_fixed_assets: Option<Decimal>,
    /// Operating working capital, in currency units, signed.
    pub operating_working_capital: Option<Decimal>,
    /// Trade receivables + inventories − trade payables, in currency units, signed.
    pub trade_working_capital: Option<Decimal>,
    /// Provisions and employee severance indemnity treated as operating, in currency units.
    pub operating_provisions: Option<Decimal>,
    /// Operating fixed assets + operating working capital − operating provisions, in currency
    /// units, signed.
    pub invested_capital: Option<Decimal>,
}

/// One component of the net financial position, with its source and maturities (R13).
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct NfpComponent {
    /// What the component is, for example "Cash (C.IV)".
    pub label: String,
    /// JSON pointer of the input it comes from (an aggregate or an adjustment).
    pub source: String,
    /// Amount added to the net financial position, in currency units, signed (negative for an
    /// asset).
    pub amount: Decimal,
    /// Part due within one year, in currency units, signed; null when its maturity split is not
    /// given.
    pub current: Option<Decimal>,
    /// Part due beyond one year, in currency units, signed; null when its maturity split is not
    /// given.
    pub non_current: Option<Decimal>,
}

/// The net financial position in the SPEC-0007 type, with its total, maturities and
/// composition (R13).
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct NetFinancialPositionBlock {
    /// The SPEC-0007 object, accepted unchanged by `ratios.*` and `dcf.fcff`.
    pub position: Option<NetFinancialPosition>,
    /// Gross debt − cash + other items, positive for net debt, in currency units, signed.
    pub total: Option<Decimal>,
    /// Part due within one year, in currency units, signed.
    pub current: Option<Decimal>,
    /// Part due beyond one year, in currency units, signed.
    pub non_current: Option<Decimal>,
    /// Every component by source (statutory form).
    pub composition: Option<Vec<NfpComponent>>,
}

/// The sources (Appendix B.1).
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Sources {
    /// Equity after subscribed capital unpaid and dividends declared, in currency units,
    /// signed.
    pub equity: Option<Decimal>,
    /// Quasi-equity items, each a label and an amount in currency units.
    pub quasi_equity_items: Option<Vec<Line>>,
    /// Σ quasi-equity items, in currency units.
    pub quasi_equity_items_total: Option<Decimal>,
    /// The net financial position (SPEC-0007 type) with its total, parts and composition.
    pub net_financial_position: NetFinancialPositionBlock,
    /// In the type of the `dcf.fcff` bridge, at carrying amount (`book_value`), amounts in
    /// currency units.
    pub debt_like_items: Option<Vec<BridgeClaim>>,
    /// Σ debt-like items, in currency units.
    pub debt_like_items_total: Option<Decimal>,
    /// Equity + quasi-equity + net financial position + debt-like items, in currency units,
    /// signed.
    pub total_sources: Option<Decimal>,
}

/// The reorganised balance sheet (R12).
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct BalanceSheet {
    /// The operating block, up to invested capital.
    pub operating: OperatingBlock,
    /// In the type `dcf.fcff` takes, never part of invested capital; amounts in currency
    /// units.
    pub non_operating_assets: Option<Vec<Line>>,
    /// Σ non-operating assets, in currency units.
    pub non_operating_assets_total: Option<Decimal>,
    /// Invested capital + Σ non-operating assets, in currency units, signed; equals
    /// `total_sources`.
    pub total_funds_invested: Option<Decimal>,
    /// Equity, quasi-equity, net financial position and debt-like items.
    pub sources: Sources,
    /// Cash of the net financial position: C.IV less operating cash, in currency units.
    pub excess_cash: Option<Decimal>,
}

/// Current and non-current amounts with the field names of the SPEC-0007 inputs (R15).
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct CurrentNonCurrent {
    /// Non-current assets (B, C.II and assets D beyond one year), in currency units.
    pub fixed_assets: Option<Decimal>,
    /// Current assets (C less C.II beyond one year and item A within it, plus assets D within
    /// one year), in currency units.
    pub current_assets: Option<Decimal>,
    /// Inventories C.I, in currency units.
    pub inventories: Option<Decimal>,
    /// Cash C.IV, in currency units.
    pub cash: Option<Decimal>,
    /// Fixed + current assets: total assets less subscribed capital unpaid, in currency units.
    pub total_assets: Option<Decimal>,
    /// Liabilities due within one year (B, C, D, E), in currency units.
    pub current_liabilities: Option<Decimal>,
    /// Liabilities due beyond one year (B, C, D, E), in currency units.
    pub non_current_liabilities: Option<Decimal>,
}

pub use crate::not_computed::{NotComputedEntry, Permitted};

/// The outputs not computed, one list per block, sorted by output pointer.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct NotComputed {
    /// Outputs of the balance sheet block.
    pub balance_sheet: Vec<NotComputedEntry>,
    /// Outputs of the income statement block.
    pub income_statement: Vec<NotComputedEntry>,
    /// Outputs of the current and non-current block.
    pub current_non_current: Vec<NotComputedEntry>,
}

/// One amount moved between A − B and a subtotal (R19).
#[derive(Debug, Clone, Serialize)]
pub struct ReconciliationLine {
    /// What the amount is, for example "B10 c, write-downs of fixed assets (investing result)".
    pub label: String,
    /// JSON pointers of the inputs actually read for it: an aggregate (both D18 and D19 when
    /// both are read; the B10 a to c group when B10 c is read as none) or an adjustment.
    pub sources: Vec<String>,
    /// Amount added to A − B, in currency units, signed.
    pub amount: Decimal,
}

/// A subtotal reconciled with the statutory difference between value and costs of production
/// (R19): `start` + Σ `lines` = `end`, exactly.
#[derive(Debug, Clone, Serialize)]
pub struct Reconciliation {
    /// A − B of the statutory income statement, in currency units, signed.
    pub start: Decimal,
    /// Every amount moved, in the order: write-downs, then adjustments in the order given, then
    /// the investing items.
    pub lines: Vec<ReconciliationLine>,
    /// The subtotal reconciled, in currency units, signed.
    pub end: Decimal,
}

/// The reconciliations of the two IFRS 18 aligned subtotals with A − B (R19).
#[derive(Debug, Clone, Serialize)]
pub struct Reconciliations {
    /// A − B to `operating_profit`.
    pub operating_profit: Option<Reconciliation>,
    /// A − B to `profit_before_financing_and_taxes`.
    pub profit_before_financing_and_taxes: Option<Reconciliation>,
}

/// The reorganised income statement (Appendix B.2; R16 to R20). Amounts named after a
/// statutory item are the core amounts that enter the layers: less the parts moved by
/// adjustments, with lease depreciation and interest added (R21).
#[derive(Debug, Clone, Serialize)]
pub struct IncomeStatement {
    /// Revenue (A1, core), in currency units.
    pub revenue: Option<Decimal>,
    /// Value of production (A1 to A5, core), in currency units, signed.
    pub value_of_production: Option<Decimal>,
    /// Purchases (B6, core), in currency units.
    pub purchases: Option<Decimal>,
    /// Materials consumed (B6 + B11, core), in currency units, signed.
    pub materials_consumed: Option<Decimal>,
    /// External costs: B6 + B7 + B8 + B11 + B14, core (E1), in currency units, signed.
    pub external_costs: Option<Decimal>,
    /// Value added: value of production − external costs, in currency units, signed.
    pub value_added: Option<Decimal>,
    /// Personnel costs (B9, core), in currency units.
    pub personnel_costs: Option<Decimal>,
    /// EBITDA (margine operativo lordo, MOL): value added − personnel costs, in currency units,
    /// signed.
    pub ebitda: Option<Decimal>,
    /// The definition of `ebitda` in words, usable as `ebitda_definition` of SPEC-0007 (R18).
    pub ebitda_definition: &'static str,
    /// Depreciation and amortisation (B10 a and b, core, plus lease depreciation), in currency
    /// units; the field of `FreeCashFlowLines`.
    pub depreciation_and_amortisation: Option<Decimal>,
    /// Other charges below EBITDA: B12 + B13 (+ B10 d with the treatment `operating`), core
    /// (E2), in currency units.
    pub other_charges_below_ebitda: Option<Decimal>,
    /// Operating profit of the core business (reddito operativo della gestione caratteristica,
    /// MON; IFRS 18 "operating profit or loss"), in currency units, signed.
    pub operating_profit: Option<Decimal>,
    /// Investing result (risultato della gestione accessoria), in currency units, signed.
    pub investing_result: Option<Decimal>,
    /// Operating profit + investing result (reddito operativo aziendale, EBIT in Italian
    /// practice; IFRS 18 "profit or loss before financing and income taxes"), in currency
    /// units, signed.
    pub profit_before_financing_and_taxes: Option<Decimal>,
    /// Σ non-recurring income − Σ non-recurring charges, in currency units, signed; zero when no
    /// item is labelled non-recurring, null when the year gives no income statement (E6).
    pub non_recurring_result: Option<Decimal>,
    /// C17-bis − C17 (core, with lease interest), in currency units, signed.
    pub financing_result: Option<Decimal>,
    /// C17 (core, plus lease interest), in currency units.
    pub interest_and_other_financial_charges: Option<Decimal>,
    /// Profit before financing and taxes + non-recurring result + financing result, in
    /// currency units, signed.
    pub profit_before_tax: Option<Decimal>,
    /// Item 20 as given, in currency units, signed (positive for a tax charge).
    pub income_taxes: Option<Decimal>,
    /// Profit before tax − income taxes, in currency units, signed.
    pub net_income: Option<Decimal>,
    /// The reconciliations of `operating_profit` and `profit_before_financing_and_taxes` with
    /// A − B (statutory form only).
    pub reconciliations: Reconciliations,
}

/// One financial year reorganised.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct YearOutput {
    /// The year's label, as given.
    pub label: Option<String>,
    /// The balance sheet date, as given.
    pub date: Option<Date>,
    /// `statutory`, `reclassified`, or null when the entry gives neither.
    pub form: Option<Form>,
    /// The statutory layout, echoed.
    pub layout: Option<Layout>,
    /// The reorganised balance sheet (R12).
    pub balance_sheet: BalanceSheet,
    /// The reorganised income statement (R16).
    pub income_statement: IncomeStatement,
    /// Current and non-current amounts for SPEC-0007 (R15).
    pub current_non_current: CurrentNonCurrent,
    /// Every aggregate given, with its destination (R11).
    pub assignments: Vec<Assignment>,
    /// Every adjustment applied to the year (R11).
    pub adjustments_applied: Vec<AppliedAdjustment>,
    /// Every relation of R7 and R12, by id.
    pub checks: BTreeMap<&'static str, Check>,
    /// The outputs not computed, with the inputs they lack (R6).
    pub not_computed: NotComputed,
}

/// Output of `statements.reorganise` (version 1).
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ReorganiseOutput {
    /// One entry per input year, in the same order.
    pub years: Vec<YearOutput>,
}

/// What `balance_sheet` and `reclassified` compute for one year.
pub(crate) struct YearBody {
    pub balance_sheet: BalanceSheet,
    pub current_non_current: CurrentNonCurrent,
    pub assignments: Vec<Assignment>,
    pub adjustments_applied: Vec<AppliedAdjustment>,
    pub checks: BTreeMap<&'static str, Check>,
    pub not_computed: NotComputed,
}

/// Conventions written while a year is computed.
pub(crate) struct Notes<'a>(&'a mut Vec<String>);

impl Notes<'_> {
    pub(crate) fn convention(&mut self, text: String) {
        if !self.0.contains(&text) {
            self.0.push(text);
        }
    }
}

const FORMULA: &str = "invested capital = operating fixed assets + operating working capital − \
    operating provisions; total funds invested = invested capital + Σ non-operating assets = \
    equity + Σ quasi-equity items + net financial position + Σ debt-like items; net financial \
    position = gross debt − cash + other items";

/// The name of year `i` in messages: its label, its date, or its position.
fn year_name(input: &ReorganiseInput, i: usize) -> String {
    let y = &input.years[i];
    match (&y.label, y.date) {
        (Some(l), _) => l.clone(),
        (None, Some(d)) => d.to_string(),
        (None, None) => format!("entry {i}"),
    }
}

/// `statements.reorganise` through a typed call (ADR 0009): the documented defaults are filled
/// in and reported first in the conventions, as through the registry.
pub fn reorganise(input: &ReorganiseInput) -> Result<(ReorganiseOutput, Explain)> {
    let mut input = input.clone();
    Reorganise::run_typed(&mut input).map(|(output, explain, _)| (output, explain))
}

/// Which treatments of the write-downs were stated by the analyst (their origin in
/// `assignments` is then `analyst`), not filled in by default.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Stated {
    pub write_downs_of_receivables: bool,
    pub write_downs_of_fixed_assets: bool,
}

/// The calculation, on an input whose defaults were filled in.
fn compute(input: &ReorganiseInput, stated: Stated) -> Result<(ReorganiseOutput, Explain)> {
    validate::validate(input)?;
    let mut ex = Explain::new(Reorganise::ID, FORMULA);
    let statutory = input.years.iter().any(|y| y.statutory.is_some());
    ex.convention(
        "missing inputs are never taken as zero: an output whose inputs are missing is null and \
         listed in `not_computed` with the pointers of the missing inputs (R6)",
    );
    ex.convention(
        "checks are exact, with no tolerance; a relation with a side missing is reported as \
         `not_checked` (R7, Decision 2)",
    );
    if statutory {
        ex.convention(
            "amounts as presented in the statutory layouts, in the currency units of the \
             statements, never scaled (R3, Decision 3)",
        );
    }
    income_statement::conventions(input, &mut ex);
    ex.convention(NFP_CONVENTION);
    if statutory {
        ex.convention(
            "net financial position: C.III and all of C.IV inside, operating cash out only by an \
             `operating_cash` adjustment (Decision 11); financial debt and shareholder loans \
             treated as financial debt are its gross debt",
        );
        ex.convention(
            "bridge items at their carrying amount: non-operating assets as labelled amounts, \
             debt-like items with measurement basis `book_value` (Decision 15)",
        );
        ex.convention(
            "subscribed capital unpaid (assets A, and item A presented within C.II) deducted from \
             equity (Decision 14); not given, it is zero (D1)",
        );
        ex.convention(
            "derivative liabilities within provisions B treated as `financial_liability` are \
             non-current in the net financial position (D3)",
        );
        ex.convention(
            "derivative assets are not given separately and follow their aggregate (D3): those \
             within B.III (B.III.4) follow the treatment of B.III and are non-current in the net \
             financial position; those within C.III (C.III.5) are current financial assets of \
             the net financial position",
        );
        ex.convention(
            "current and non-current: receivables (C.II) and payables (D) split by their \
             statutory amounts beyond one year, read as twelve months (R15); the adjustments of \
             the reorganisation do not change the statutory maturities",
        );
    }
    let reclassified: Vec<String> = (0..input.years.len())
        .filter(|&i| input.years[i].reclassified.is_some())
        .map(|i| year_name(input, i))
        .collect();
    if !reclassified.is_empty() {
        ex.convention(format!(
            "treatments and adjustments do not apply to the reclassified entries: {}",
            reclassified.join(", ")
        ));
    }

    let mut later = Vec::new();
    let mut years = Vec::with_capacity(input.years.len());
    for (i, y) in input.years.iter().enumerate() {
        let name = year_name(input, i);
        let form = match (&y.statutory, &y.reclassified) {
            (Some(_), _) => Some(Form::Statutory),
            (None, Some(_)) => Some(Form::Reclassified),
            (None, None) => None,
        };
        ex.convention(match form {
            Some(Form::Statutory) => format!("year {name}: statutory form"),
            Some(Form::Reclassified) => format!("year {name}: reclassified form"),
            None => format!(
                "year {name}: no form given: nothing is computed; every output is listed in \
                 `not_computed`"
            ),
        });
        if y.date.is_none() {
            ex.convention(format!(
                "year {name}: no date given: the order of `years` is taken as oldest first (R2)"
            ));
        }
        let layout = y.statutory.as_ref().and_then(|s| s.layout);
        if let Some(l) = layout {
            let id = match l {
                Layout::Ordinary => "ordinary",
                Layout::Abridged => "abridged",
                Layout::MicroEntity => "micro_entity",
            };
            ex.convention(format!(
                "year {name}: layout `{id}` echoed; no number depends on it"
            ));
        }
        let (mut body, income) = match form {
            Some(Form::Reclassified) => (
                reclassified::reclassified(input, i, &name, &mut ex)?,
                income_statement::reclassified(input, i, &name, &mut ex)?,
            ),
            _ => {
                zero_treatment_conventions(input, i, &name, &mut ex);
                let mut notes = Notes(&mut later);
                let bs = balance_sheet::statutory(input, i, &name, &mut ex, &mut notes)?;
                (
                    bs,
                    income_statement::statutory(input, i, &name, stated, &mut ex, &mut notes)?,
                )
            }
        };
        body.assignments.extend(income.assignments);
        body.adjustments_applied.extend(income.adjustments_applied);
        body.adjustments_applied.sort_by_key(|a| a.index);
        body.checks.extend(income.checks);
        body.not_computed.income_statement = income.not_computed;
        years.push(YearOutput {
            label: y.label.clone(),
            date: y.date,
            form,
            layout,
            balance_sheet: body.balance_sheet,
            income_statement: income.income_statement,
            current_non_current: body.current_non_current,
            assignments: body.assignments,
            adjustments_applied: body.adjustments_applied,
            checks: body.checks,
            not_computed: body.not_computed,
        });
    }
    for text in later {
        ex.convention(text);
    }
    Ok((ReorganiseOutput { years }, ex))
}

/// R8: a required treatment of an aggregate given as zero is not needed, and the conventions
/// say so.
fn zero_treatment_conventions(input: &ReorganiseInput, i: usize, name: &str, ex: &mut Explain) {
    let y = &input.years[i];
    let Some(bs) = y.statutory.as_ref().and_then(|s| s.balance_sheet.as_ref()) else {
        return;
    };
    let overrides = y.treatment_overrides.as_ref();
    let zero = |v: Option<Decimal>| v.is_some_and(|v| v.is_zero());
    if zero(bs.financial_fixed_assets)
        && input.treatments.financial_fixed_assets.is_none()
        && overrides
            .and_then(|o| o.financial_fixed_assets.as_ref())
            .is_none()
    {
        ex.convention(format!(
            "year {name}: financial fixed assets (B.III) are zero: no treatment is needed"
        ));
    }
    let sl = bs
        .payables
        .as_ref()
        .and_then(|p| p.shareholder_loans.as_ref())
        .and_then(|n| n.total);
    if zero(sl)
        && input.treatments.shareholder_loans.is_none()
        && overrides
            .and_then(|o| o.shareholder_loans.as_ref())
            .is_none()
    {
        ex.convention(format!(
            "year {name}: shareholder loans (D) are zero: no treatment is needed"
        ));
    }
    let deriv = bs
        .provisions
        .as_ref()
        .and_then(|p| p.of_which_derivative_liabilities);
    if zero(deriv) && input.treatments.derivative_liabilities.is_none() {
        ex.convention(format!(
            "year {name}: derivative liabilities within provisions (B) are zero: no treatment \
             is needed"
        ));
    }
}

/// `statements.reorganise` (SPEC-0005).
pub struct Reorganise;

impl Method for Reorganise {
    const ID: &'static str = "statements.reorganise";
    const TITLE: &'static str = "Financial statements — operating and financing reorganisation";
    const DESCRIPTION: &'static str = "Reorganises statutory aggregates (art. 2424, 2425, \
        abridged art. 2435-bis) or totals already reclassified, for one or more financial years, \
        into invested capital, non-operating assets, equity, quasi-equity, the net financial \
        position (SPEC-0007 type) and debt-like items, plus current and non-current amounts for \
        the ratios, and the income statement by nature reorganised into value of production, \
        value added, EBITDA, operating profit, investing result, profit before financing and \
        taxes (subtotals aligned with IFRS 18), non-recurring and financing results, profit \
        before tax and net income, with its reconciliations with A − B; best effort: each \
        output is computed when its inputs are given, otherwise null and listed in \
        `not_computed` with the missing input pointers; totals are checked exactly (FM-E026).";
    const VERSION: &'static str = "1";

    type Input = ReorganiseInput;
    type Output = ReorganiseOutput;

    fn compute(input: &ReorganiseInput) -> Result<(ReorganiseOutput, Explain)> {
        compute(
            input,
            Stated {
                write_downs_of_receivables: input.treatments.write_downs_of_receivables.is_some(),
                write_downs_of_fixed_assets: input.treatments.write_downs_of_fixed_assets.is_some(),
            },
        )
    }

    /// A treatment of B10 c or B10 d filled in by default is not the analyst's: its assignment
    /// keeps the origin `mapping`.
    fn compute_with_defaults(
        input: &ReorganiseInput,
        defaults: &[DefaultApplied],
    ) -> Result<(ReorganiseOutput, Explain)> {
        let filled = |field: &str| defaults.iter().any(|d| d.field == field);
        compute(
            input,
            Stated {
                write_downs_of_receivables: input.treatments.write_downs_of_receivables.is_some()
                    && !filled("/treatments/write_downs_of_receivables"),
                write_downs_of_fixed_assets: input.treatments.write_downs_of_fixed_assets.is_some()
                    && !filled("/treatments/write_downs_of_fixed_assets"),
            },
        )
    }

    /// R8 as clarified on 2026-11-08, R17, R18, D1, D2, E3, F1, P1: by year, the `default`
    /// destination of every statutory aggregate given (balance sheet, then A5, B14, B10 c and
    /// B10 d of the income statement; one entry each, with the pointer of the item, the
    /// destination and the reason; B10 c and B10 d only while their treatment is not given),
    /// then the input defaults `write_downs_of_receivables = operating` and
    /// `write_downs_of_fixed_assets = operating`, each filled in and listed only when a
    /// statutory year gives its aggregate (F1), then every item taken as zero because it is not
    /// given.
    fn apply_defaults(input: &mut ReorganiseInput) -> Vec<DefaultApplied> {
        let mut out = Vec::new();
        let b10 = |input: &ReorganiseInput, f: fn(&DepreciationAndWriteDowns) -> bool| {
            input.years.iter().any(|y| {
                y.statutory
                    .as_ref()
                    .and_then(|s| s.income_statement.as_ref())
                    .and_then(|is| is.depreciation_and_write_downs.as_ref())
                    .is_some_and(f)
            })
        };
        let b10d = b10(input, |n| n.write_downs_of_receivables.is_some());
        let b10c = b10(input, |n| n.write_downs_of_fixed_assets.is_some());
        for (i, y) in input.years.iter().enumerate() {
            let Some(st) = &y.statutory else { continue };
            if let Some(bs) = &st.balance_sheet {
                for row in BALANCE_SHEET_DEFAULTS {
                    if (row.given)(bs).is_some() {
                        out.push(DefaultApplied::destination(
                            format!("/years/{i}/statutory/balance_sheet/{}", row.key),
                            row.destination.id(),
                            row.reason,
                        ));
                    }
                }
            }
            if let Some(is) = &st.income_statement {
                for row in INCOME_STATEMENT_DEFAULTS {
                    if (row.given)(is).is_some() && !(row.stated)(&input.treatments) {
                        out.push(DefaultApplied::destination(
                            format!("/years/{i}/statutory/income_statement/{}", row.key),
                            row.destination.id(),
                            row.reason,
                        ));
                    }
                }
            }
        }
        if b10d && input.treatments.write_downs_of_receivables.is_none() {
            input.treatments.write_downs_of_receivables = Some(WriteDownsOfReceivables::Operating);
            out.push(DefaultApplied::new(
                "/treatments/write_downs_of_receivables",
                serde_json::json!("operating"),
                WRITE_DOWNS_OF_RECEIVABLES_DEFAULT,
            ));
        }
        if b10c && input.treatments.write_downs_of_fixed_assets.is_none() {
            input.treatments.write_downs_of_fixed_assets = Some(WriteDownsOfFixedAssets::Operating);
            out.push(DefaultApplied::new(
                "/treatments/write_downs_of_fixed_assets",
                serde_json::json!("operating"),
                WRITE_DOWNS_OF_FIXED_ASSETS_DEFAULT,
            ));
        }
        // D1, D2: every item of a statutory balance sheet taken as zero because it is not
        // given, also when its parent node is absent.
        for (i, y) in input.years.iter().enumerate() {
            let Some(bs) = y.statutory.as_ref().and_then(|s| s.balance_sheet.as_ref()) else {
                continue;
            };
            for row in TAKEN_AS_ZERO.iter().filter(|row| (row.applies)(bs)) {
                out.push(DefaultApplied::new(
                    format!("/years/{i}/statutory/balance_sheet/{}", row.key),
                    serde_json::json!("0"),
                    row.reason,
                ));
            }
        }
        out
    }
}
