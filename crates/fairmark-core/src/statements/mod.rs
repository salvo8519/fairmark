//! Financial statements reorganised into an operating and financing view (SPEC-0005).
//!
//! One method, `statements.reorganise` version 1, preview (ADR 0012). It takes one or more
//! financial years, each in the `statutory` form (aggregates of art. 2424 and 2425) or in the
//! `reclassified` form (totals already reorganised), any subset of the totals, and returns per
//! year what they allow: best effort, with every missing output listed in `not_computed` with
//! the pointers of the missing inputs (R6), and every relation checked exactly when both sides
//! are given (R7).
//!
//! This build implements the balance sheet (first pull request of the plan): the reorganised
//! balance sheet in both forms, the net financial position in the SPEC-0007 type, the bridge
//! items in the `dcf.fcff` types and the current and non-current block for SPEC-0007. The
//! income statement inputs are validated and checked; their reorganisation is not returned yet.

mod balance_sheet;
mod checks;
pub mod input;
mod partial;
mod reclassified;
mod validate;

use std::collections::BTreeMap;

use rust_decimal::Decimal;
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Form {
    Statutory,
    Reclassified,
}

/// Where an aggregate, or the part of it moved by an adjustment, goes (Appendix A, B, C).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Destination {
    OperatingFixedAssets,
    OperatingWorkingCapital,
    OperatingProvisions,
    NonOperatingAssets,
    NetFinancialPositionGrossDebt,
    NetFinancialPositionCash,
    NetFinancialPositionOtherFinancialAsset,
    NetFinancialPositionOtherFinancialLiability,
    QuasiEquityItems,
    DebtLikeItems,
    Equity,
    DeductedFromEquity,
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
        }
    }
}

/// Who chose a destination (R11).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
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
#[derive(Debug, Clone, Serialize)]
pub struct Assignment {
    /// Statutory reference, for example "B.III" or "liabilities D, remainder".
    pub reference: &'static str,
    /// JSON pointer of the input aggregate.
    pub aggregate: String,
    /// Amount assigned, in currency units.
    pub amount: Decimal,
    pub destination: Destination,
    /// `mapping` (Appendix A) or `analyst` (treatment or override).
    pub origin: Origin,
}

/// One adjustment applied to a year (R11).
#[derive(Debug, Clone, Serialize)]
pub struct AppliedAdjustment {
    /// Position of the adjustment in `adjustments`.
    pub index: usize,
    pub kind: &'static str,
    pub label: String,
    /// Amount moved, in currency units.
    pub amount: Decimal,
    pub reason: String,
    /// JSON pointer of the source aggregate.
    pub source: String,
    pub target: Destination,
}

/// The operating block (Appendix B.1).
#[derive(Debug, Clone, Serialize)]
pub struct OperatingBlock {
    /// Operating fixed assets, in currency units.
    pub operating_fixed_assets: Option<Decimal>,
    /// Operating working capital, in currency units.
    pub operating_working_capital: Option<Decimal>,
    /// Trade receivables + inventories − trade payables, in currency units.
    pub trade_working_capital: Option<Decimal>,
    /// Provisions and employee severance indemnity treated as operating, in currency units.
    pub operating_provisions: Option<Decimal>,
    /// Operating fixed assets + operating working capital − operating provisions.
    pub invested_capital: Option<Decimal>,
}

/// One component of the net financial position, with its source and maturities (R13).
#[derive(Debug, Clone, Serialize)]
pub struct NfpComponent {
    pub label: String,
    /// JSON pointer of the input it comes from (an aggregate or an adjustment).
    pub source: String,
    /// Amount added to the net financial position, signed (negative for an asset).
    pub amount: Decimal,
    /// Part due within one year, signed; null when its maturity split is not given.
    pub current: Option<Decimal>,
    /// Part due beyond one year, signed; null when its maturity split is not given.
    pub non_current: Option<Decimal>,
}

/// The net financial position in the SPEC-0007 type, with its total, maturities and
/// composition (R13).
#[derive(Debug, Clone, Serialize)]
pub struct NetFinancialPositionBlock {
    /// The SPEC-0007 object, accepted unchanged by `ratios.*` and `dcf.fcff`.
    pub position: Option<NetFinancialPosition>,
    /// Gross debt − cash + other items, positive for net debt, in currency units.
    pub total: Option<Decimal>,
    /// Part due within one year, in currency units.
    pub current: Option<Decimal>,
    /// Part due beyond one year, in currency units.
    pub non_current: Option<Decimal>,
    /// Every component by source (statutory form).
    pub composition: Option<Vec<NfpComponent>>,
}

/// The sources (Appendix B.1).
#[derive(Debug, Clone, Serialize)]
pub struct Sources {
    /// Equity after subscribed capital unpaid and dividends declared, in currency units.
    pub equity: Option<Decimal>,
    pub quasi_equity_items: Option<Vec<Line>>,
    pub quasi_equity_items_total: Option<Decimal>,
    pub net_financial_position: NetFinancialPositionBlock,
    /// In the type of the `dcf.fcff` bridge, at carrying amount (`book_value`).
    pub debt_like_items: Option<Vec<BridgeClaim>>,
    pub debt_like_items_total: Option<Decimal>,
    /// Equity + quasi-equity + net financial position + debt-like items.
    pub total_sources: Option<Decimal>,
}

/// The reorganised balance sheet (R12).
#[derive(Debug, Clone, Serialize)]
pub struct BalanceSheet {
    pub operating: OperatingBlock,
    /// In the type `dcf.fcff` takes, never part of invested capital.
    pub non_operating_assets: Option<Vec<Line>>,
    pub non_operating_assets_total: Option<Decimal>,
    /// Invested capital + Σ non-operating assets; equals `total_sources`.
    pub total_funds_invested: Option<Decimal>,
    pub sources: Sources,
    /// Cash of the net financial position: C.IV less operating cash.
    pub excess_cash: Option<Decimal>,
}

/// Current and non-current amounts with the field names of the SPEC-0007 inputs (R15).
#[derive(Debug, Clone, Serialize)]
pub struct CurrentNonCurrent {
    /// Non-current assets, in currency units.
    pub fixed_assets: Option<Decimal>,
    pub current_assets: Option<Decimal>,
    pub inventories: Option<Decimal>,
    pub cash: Option<Decimal>,
    /// Fixed + current assets: total assets less subscribed capital unpaid.
    pub total_assets: Option<Decimal>,
    pub current_liabilities: Option<Decimal>,
    pub non_current_liabilities: Option<Decimal>,
}

/// A missing treatment and the destinations it permits (R8).
#[derive(Debug, Clone, Serialize)]
pub struct Permitted {
    pub treatment: String,
    pub destinations: Vec<&'static str>,
}

/// An output not computed, with the pointers of its missing inputs (R6).
#[derive(Debug, Clone, Serialize)]
pub struct NotComputedEntry {
    /// JSON pointer of the output in the result value.
    pub output: String,
    /// JSON pointers of the inputs it needs and lacks.
    pub missing: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub permitted: Vec<Permitted>,
}

/// The outputs not computed, one list per block, sorted by output pointer.
#[derive(Debug, Clone, Serialize)]
pub struct NotComputed {
    pub balance_sheet: Vec<NotComputedEntry>,
    pub current_non_current: Vec<NotComputedEntry>,
}

/// One financial year reorganised.
#[derive(Debug, Clone, Serialize)]
pub struct YearOutput {
    pub label: Option<String>,
    pub date: Option<Date>,
    /// `statutory`, `reclassified`, or null when the entry gives neither.
    pub form: Option<Form>,
    pub layout: Option<Layout>,
    pub balance_sheet: BalanceSheet,
    pub current_non_current: CurrentNonCurrent,
    pub assignments: Vec<Assignment>,
    pub adjustments_applied: Vec<AppliedAdjustment>,
    /// Every relation of R7 and R12, by id.
    pub checks: BTreeMap<&'static str, Check>,
    pub not_computed: NotComputed,
}

/// Output of `statements.reorganise` (version 1).
#[derive(Debug, Clone, Serialize)]
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

/// The calculation, on an input whose defaults were filled in.
fn compute(input: &ReorganiseInput) -> Result<(ReorganiseOutput, Explain)> {
    validate::validate(input)?;
    let mut ex = Explain::new(Reorganise::ID, FORMULA);
    let statutory = input.years.iter().any(|y| y.reclassified.is_none());
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
        ex.convention(
            "the income statement is not reorganised in this build: its inputs are validated \
             and checked (R3, R7) and no income statement block is returned",
        );
    }
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
        ex.convention("subscribed capital unpaid (assets A) deducted from equity (Decision 14)");
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
        let body = match form {
            Some(Form::Reclassified) => reclassified::reclassified(input, i, &name, &mut ex)?,
            _ => {
                zero_treatment_conventions(input, i, &name, &mut ex);
                balance_sheet::statutory(input, i, &name, &mut ex, &mut Notes(&mut later))?
            }
        };
        years.push(YearOutput {
            label: y.label.clone(),
            date: y.date,
            form,
            layout,
            balance_sheet: body.balance_sheet,
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
        the ratios; best effort: each output is computed when its inputs are given, otherwise \
        null and listed in `not_computed` with the missing input pointers; totals are checked \
        exactly (FM-E026). This build returns the balance sheet.";
    const VERSION: &'static str = "1";

    type Input = ReorganiseInput;
    type Output = ReorganiseOutput;

    fn compute(input: &ReorganiseInput) -> Result<(ReorganiseOutput, Explain)> {
        compute(input)
    }

    /// R8 as clarified on 2026-11-08, R17: the `default` destination of every statutory
    /// aggregate given (one entry each, with the pointer of the item, the destination and the
    /// reason), then the input default `write_downs_of_receivables = investing`.
    fn apply_defaults(input: &mut ReorganiseInput) -> Vec<DefaultApplied> {
        let mut out = Vec::new();
        let write_downs_given = input.treatments.write_downs_of_receivables.is_some();
        for (i, y) in input.years.iter().enumerate() {
            let Some(st) = &y.statutory else { continue };
            if let Some(bs) = &st.balance_sheet {
                for (key, destination, reason) in BALANCE_SHEET_DEFAULTS {
                    if bs.has(key) {
                        out.push(DefaultApplied::destination(
                            format!("/years/{i}/statutory/balance_sheet/{key}"),
                            destination,
                            reason,
                        ));
                    }
                }
            }
            if let Some(is) = &st.income_statement {
                let base = format!("/years/{i}/statutory/income_statement");
                for (key, destination, reason) in INCOME_STATEMENT_DEFAULTS {
                    if is.has(key) {
                        out.push(DefaultApplied::destination(
                            format!("{base}/{key}"),
                            destination,
                            reason,
                        ));
                    }
                }
                let b10d = is
                    .depreciation_and_write_downs
                    .as_ref()
                    .and_then(|b| b.write_downs_of_receivables);
                if !write_downs_given && b10d.is_some() {
                    out.push(DefaultApplied::destination(
                        format!("{base}/depreciation_and_write_downs/write_downs_of_receivables"),
                        "investing_result",
                        WRITE_DOWNS_OF_RECEIVABLES_DEFAULT,
                    ));
                }
            }
        }
        if !write_downs_given {
            input.treatments.write_downs_of_receivables = Some(WriteDownsOfReceivables::Investing);
            out.push(DefaultApplied::new(
                "/treatments/write_downs_of_receivables",
                serde_json::json!("investing"),
                WRITE_DOWNS_OF_RECEIVABLES_DEFAULT,
            ));
        }
        out
    }
}
