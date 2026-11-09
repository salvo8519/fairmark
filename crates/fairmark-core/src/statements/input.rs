//! Input of `statements.reorganise` version 1 (SPEC-0005 R1 to R5, R8 to R10, R17, R21, R22).
//!
//! The schema is complete in the first pull request of the spec: both forms (`statutory`,
//! `reclassified`), both statements, the treatments, the per-year overrides and every
//! adjustment kind of Appendix C. Later pull requests change the computation and the output,
//! never this input (plan, "The input schema is complete in the first pull request").
//!
//! Every amount is in the currency units of the statements, one unit per call, never scaled,
//! and entered as presented in the statutory layouts (R3): zero or more unless the field says
//! "signed". Every node is optional (R6): a missing input is never taken as zero.

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};

use crate::dates::Date;
use crate::dcf::BridgeClaim;
use crate::ratios::NetFinancialPosition;
use crate::tv::Line;

use super::Destination;

/// Input of `statements.reorganise` (version 1).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReorganiseInput {
    /// One or more financial years, oldest first (R2), each in the `statutory` or the
    /// `reclassified` form. When two consecutive entries both have a `date`, the later must be
    /// one financial year after the earlier.
    #[schemars(length(min = 1))]
    pub years: Vec<Year>,
    /// Treatments stated once for the call, applied to every statutory year (R8, R9, R17).
    /// A treatment is needed only for an aggregate given and not zero. Default: none, except
    /// `write_downs_of_receivables` and `write_downs_of_fixed_assets`, which default to
    /// `operating`, filled in and listed in `meta.defaults_applied` whenever a statutory year
    /// gives an income statement (A4). Each treatment not given reads as not given; the
    /// `null` values of the schema default mean "not given", not a choice.
    #[serde(default)]
    pub treatments: Treatments,
    /// Labelled adjustments of Appendix C, stated once for the call, each with one amount per
    /// year (R10). They apply to statutory entries only. Default: none.
    #[serde(default)]
    pub adjustments: Vec<Adjustment>,
}

/// One financial year (R2).
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct Year {
    /// Name of the year in the result, for example "2024". Optional; reported as given.
    pub label: Option<String>,
    /// Balance sheet date, ISO 8601 `YYYY-MM-DD`. Optional; when missing, the order given is
    /// taken as oldest first and the conventions say so.
    pub date: Option<Date>,
    /// The statutory aggregates of art. 2424 and 2425 (Appendix A). Give this or
    /// `reclassified`, not both.
    pub statutory: Option<Statutory>,
    /// Totals already reclassified (Appendix B). Give this or `statutory`, not both.
    pub reclassified: Option<Reclassified>,
    /// For this year only, another treatment of financial fixed assets (B.III) or of
    /// shareholder loans, or a split by amounts (R9). No other aggregate can be overridden.
    pub treatment_overrides: Option<TreatmentOverrides>,
}

/// The statutory layout, echoed in the result; no number depends on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Layout {
    /// Ordinary layout, art. 2424 and 2425 of the Civil Code.
    Ordinary,
    /// Abridged layout, art. 2435-bis.
    Abridged,
    /// Micro-entity layout, art. 2435-ter.
    MicroEntity,
}

/// A year in the statutory form.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct Statutory {
    /// `ordinary`, `abridged` or `micro_entity`; optional, echoed only.
    pub layout: Option<Layout>,
    /// Balance sheet aggregates (art. 2424; abridged art. 2435-bis).
    pub balance_sheet: Option<StatutoryBalanceSheet>,
    /// Income statement aggregates (art. 2425; groupings of art. 2435-bis).
    pub income_statement: Option<StatutoryIncomeStatement>,
}

/// A fixed asset shown net of accumulated amortisation, depreciation and write-downs.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct NetFixedAsset {
    /// Net carrying amount, in currency units. Checked against `gross − accumulated` when all
    /// three are given.
    pub total: Option<Decimal>,
    /// Gross amount (historical cost and revaluations), in currency units.
    pub gross: Option<Decimal>,
    /// Accumulated amortisation or depreciation and write-downs, in currency units.
    pub accumulated: Option<Decimal>,
}

/// An aggregate with its part due beyond the next financial year.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct WithMaturity {
    /// The aggregate, in currency units.
    pub total: Option<Decimal>,
    /// Of which due beyond the next financial year (non-current), in currency units.
    pub beyond_one_year: Option<Decimal>,
}

/// Receivables (C.II) with their "of which" parts.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct Receivables {
    /// Receivables C.II, in currency units, including the parts below.
    pub total: Option<Decimal>,
    /// Of which due beyond the next financial year, in currency units; needed for the current
    /// and non-current block.
    pub beyond_one_year: Option<Decimal>,
    /// Of which trade receivables, in currency units; needed for `trade_working_capital`.
    pub trade: Option<Decimal>,
    /// Of which subscribed capital unpaid (item A presented within C.II, art. 2435-bis), in
    /// currency units; deducted from equity. Not given: none within C.II (D2), listed in
    /// `meta.defaults_applied`.
    pub subscribed_capital_unpaid: Option<Decimal>,
    /// Of which prepayments and accrued income (assets D presented within C.II,
    /// art. 2435-bis), in currency units; when given without a separate line D, that line is
    /// zero. Not given: none within C.II (D2), listed in `meta.defaults_applied`.
    pub prepayments_and_accrued_income: Option<Decimal>,
}

/// Equity, liabilities A, as a total or by its Roman numerals.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct Equity {
    /// Total equity A, in currency units, signed (negative for a deficit).
    pub total: Option<Decimal>,
    /// A.I share capital, in currency units.
    pub share_capital: Option<Decimal>,
    /// A.II share premium reserve, in currency units.
    pub share_premium: Option<Decimal>,
    /// A.III revaluation reserves, in currency units.
    pub revaluation_reserves: Option<Decimal>,
    /// A.IV legal reserve, in currency units.
    pub legal_reserve: Option<Decimal>,
    /// A.V statutory reserves, in currency units.
    pub statutory_reserves: Option<Decimal>,
    /// A.VI other reserves, including the rounding reserve of statements in euro units, in
    /// currency units, signed.
    pub other_reserves: Option<Decimal>,
    /// A.VII cash flow hedge reserve, in currency units, signed.
    pub cash_flow_hedge_reserve: Option<Decimal>,
    /// A.VIII retained earnings (losses carried forward), in currency units, signed.
    pub retained_earnings: Option<Decimal>,
    /// A.IX profit (loss) for the year, in currency units, signed; checked against item 21.
    pub profit_or_loss_for_the_year: Option<Decimal>,
    /// A.X negative reserve for own shares held, in currency units, zero or negative.
    pub negative_reserve_for_own_shares: Option<Decimal>,
}

/// Provisions for risks and charges, liabilities B.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct Provisions {
    /// Provisions B, in currency units, including the derivative part.
    pub total: Option<Decimal>,
    /// Of which due beyond the next financial year, in currency units. Not given: all
    /// non-current, reported.
    pub beyond_one_year: Option<Decimal>,
    /// Of which derivative liabilities (B.3), in currency units; needs the treatment
    /// `derivative_liabilities` when not zero. Not given: none within B (D2, D3), listed in
    /// `meta.defaults_applied`.
    pub of_which_derivative_liabilities: Option<Decimal>,
}

/// Payables, liabilities D, with their "of which" parts.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct Payables {
    /// Payables D, in currency units, including the parts below.
    pub total: Option<Decimal>,
    /// Of which due beyond the next financial year, in currency units; needed for the current
    /// and non-current block.
    pub beyond_one_year: Option<Decimal>,
    /// Of which financial debt (bonds, banks, other lenders: D.1, D.2, D.4, D.5), with its
    /// part beyond one year; needed for the net financial position (Decision 4).
    pub financial_debt: Option<WithMaturity>,
    /// Of which shareholder loans (D.3), with its part beyond one year; needed for the net
    /// financial position, and needs the treatment `shareholder_loans` when not zero.
    pub shareholder_loans: Option<WithMaturity>,
    /// Of which trade payables, in currency units; needed for `trade_working_capital`.
    pub trade: Option<Decimal>,
    /// Of which accruals and deferred income (liabilities E presented within D,
    /// art. 2435-bis), in currency units; when given without a separate line E, that line is
    /// zero. Not given: none within D (D2), listed in `meta.defaults_applied`.
    pub accruals_and_deferred_income: Option<Decimal>,
}

/// Balance sheet aggregates of art. 2424 (Appendix A.1). Each is optional; a node accepts a
/// total, its components or both.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct StatutoryBalanceSheet {
    /// Assets A, subscribed capital unpaid, in currency units; deducted from equity. Not given:
    /// zero (D1), listed in `meta.defaults_applied`; give it to have total assets checked.
    pub subscribed_capital_unpaid: Option<Decimal>,
    /// B.I intangible assets, net, or gross and accumulated amortisation.
    pub intangible_assets: Option<NetFixedAsset>,
    /// B.II tangible assets, net, or gross and accumulated depreciation.
    pub tangible_assets: Option<NetFixedAsset>,
    /// B.III financial fixed assets, in currency units; needs the treatment
    /// `financial_fixed_assets` when not zero.
    pub financial_fixed_assets: Option<Decimal>,
    /// Total B, fixed assets, in currency units; checked against B.I + B.II + B.III.
    pub fixed_assets: Option<Decimal>,
    /// C.I inventories, in currency units.
    pub inventories: Option<Decimal>,
    /// C.II receivables with their "of which" parts.
    pub receivables: Option<Receivables>,
    /// C.III current financial assets (financial assets that are not fixed assets), in
    /// currency units.
    pub current_financial_assets: Option<Decimal>,
    /// C.IV cash and cash equivalents, in currency units.
    pub cash: Option<Decimal>,
    /// Total C, current assets, in currency units; checked against C.I + C.II + C.III + C.IV.
    pub current_assets: Option<Decimal>,
    /// Assets D, prepayments and accrued income, with the part beyond one year. Not needed
    /// when assets D are given within C.II.
    pub prepayments_and_accrued_income: Option<WithMaturity>,
    /// Total assets, in currency units; checked against A + B + C + D.
    pub total_assets: Option<Decimal>,
    /// Liabilities A, equity, as a total or by its Roman numerals.
    pub equity: Option<Equity>,
    /// Liabilities B, provisions for risks and charges.
    pub provisions: Option<Provisions>,
    /// Liabilities C, employee severance indemnity (TFR), with the part beyond one year. Not
    /// given: all non-current, reported.
    pub employee_severance_indemnity: Option<WithMaturity>,
    /// Liabilities D, payables, with their "of which" parts.
    pub payables: Option<Payables>,
    /// Liabilities E, accruals and deferred income, with the part beyond one year. Not needed
    /// when liabilities E are given within D.
    pub accruals_and_deferred_income: Option<WithMaturity>,
    /// Total liabilities and equity, in currency units; checked against A + B + C + D + E and
    /// against total assets.
    pub total_liabilities_and_equity: Option<Decimal>,
}

/// B10 depreciation, amortisation and write-downs.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct DepreciationAndWriteDowns {
    /// B10 total, in currency units.
    pub total: Option<Decimal>,
    /// B10 a to c as one amount, as grouped by the abridged layout (art. 2435-bis), in
    /// currency units. With its B10 a, b part and without `write_downs_of_fixed_assets`,
    /// B10 c is the difference (D1-IS). With neither part, it is all in depreciation and
    /// amortisation under the treatment `operating`, and read as depreciation and
    /// amortisation with B10 c none under `investing`, reported (R17).
    pub value_adjustments_of_fixed_assets: Option<Decimal>,
    /// B10 a and b, amortisation of intangible and depreciation of tangible assets, in
    /// currency units.
    pub depreciation_and_amortisation: Option<Decimal>,
    /// B10 c, other write-downs of fixed assets, in currency units; with depreciation and
    /// amortisation below EBITDA by default, or in the investing result with
    /// `treatments.write_downs_of_fixed_assets: investing` (R17, P1).
    pub write_downs_of_fixed_assets: Option<Decimal>,
    /// B10 d, write-downs of current receivables and cash, in currency units; in the other
    /// charges below EBITDA by default, or in the investing result with
    /// `treatments.write_downs_of_receivables: investing` (R17, E3).
    pub write_downs_of_receivables: Option<Decimal>,
}

/// Income statement aggregates of art. 2425 by nature (Appendix A.2). Each is optional.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct StatutoryIncomeStatement {
    /// Total A, value of production, in currency units, signed.
    pub value_of_production: Option<Decimal>,
    /// A1 revenue from sales and services, in currency units.
    pub revenue: Option<Decimal>,
    /// A2 and A3 changes in inventories of work in progress, semi-finished and finished
    /// products and in contract work in progress, in currency units, signed (positive for an
    /// increase).
    pub change_in_product_inventories: Option<Decimal>,
    /// A4 own work capitalised, in currency units.
    pub own_work_capitalised: Option<Decimal>,
    /// A5 other revenue and income, in currency units. Default destination: the value of
    /// production, core (Decision 7), listed in `meta.defaults_applied`; investing and
    /// non-recurring parts move only by adjustment.
    pub other_revenue_and_income: Option<Decimal>,
    /// Total B, costs of production, in currency units, signed.
    pub costs_of_production: Option<Decimal>,
    /// B6 raw materials, consumables and goods purchased, in currency units.
    pub purchases: Option<Decimal>,
    /// B7 services, in currency units.
    pub services: Option<Decimal>,
    /// B8 use of third-party assets, in currency units.
    pub use_of_third_party_assets: Option<Decimal>,
    /// B9 personnel costs, in currency units.
    pub personnel_costs: Option<Decimal>,
    /// B10 depreciation, amortisation and write-downs.
    pub depreciation_and_write_downs: Option<DepreciationAndWriteDowns>,
    /// B11 change in inventories of raw materials, consumables and goods, in currency units,
    /// signed (positive for a decrease, a cost).
    pub change_in_raw_material_inventories: Option<Decimal>,
    /// B12 provisions for risks, in currency units.
    pub provisions_for_risks: Option<Decimal>,
    /// B13 other provisions, in currency units.
    pub other_provisions: Option<Decimal>,
    /// B14 other operating expenses, in currency units. Default destination: external costs,
    /// core (Decision 7, E1), listed in `meta.defaults_applied`; investing and non-recurring
    /// parts move only by adjustment.
    pub other_operating_expenses: Option<Decimal>,
    /// A − B, difference between value and costs of production, in currency units, signed.
    pub difference_between_value_and_costs_of_production: Option<Decimal>,
    /// Total C, financial income and charges, in currency units, signed.
    pub financial_income_and_charges: Option<Decimal>,
    /// C15 income from equity investments, in currency units.
    pub income_from_equity_investments: Option<Decimal>,
    /// C16 other financial income, in currency units.
    pub other_financial_income: Option<Decimal>,
    /// C17 interest and other financial charges, in currency units.
    pub interest_and_other_financial_charges: Option<Decimal>,
    /// C17-bis exchange gains and losses, in currency units, signed (positive for a net gain).
    pub exchange_gains_and_losses: Option<Decimal>,
    /// Total D, value adjustments of financial assets, in currency units, signed.
    pub value_adjustments_of_financial_assets: Option<Decimal>,
    /// D18 revaluations of financial assets, in currency units.
    pub revaluations_of_financial_assets: Option<Decimal>,
    /// D19 write-downs of financial assets, in currency units.
    pub write_downs_of_financial_assets: Option<Decimal>,
    /// Profit (loss) before tax, A − B ± C ± D, in currency units, signed.
    pub profit_before_tax: Option<Decimal>,
    /// Item 20, income taxes for the year, current, deferred and prepaid, in currency units,
    /// signed (positive for a tax charge).
    pub income_taxes: Option<Decimal>,
    /// Item 21, profit (loss) for the year, in currency units, signed.
    pub profit_or_loss_for_the_year: Option<Decimal>,
}

/// A year in the reclassified form: totals already reorganised (Appendix B, R5).
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct Reclassified {
    /// Reclassified balance sheet totals (Appendix B.1).
    pub balance_sheet: Option<ReclassifiedBalanceSheet>,
    /// Reclassified income statement totals (Appendix B.2).
    pub income_statement: Option<ReclassifiedIncomeStatement>,
}

/// Operating working capital, as a total or by its components.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct OperatingWorkingCapital {
    /// Operating working capital, in currency units, signed; checked against its components.
    pub total: Option<Decimal>,
    /// Trade receivables, in currency units.
    pub trade_receivables: Option<Decimal>,
    /// Inventories, in currency units.
    pub inventories: Option<Decimal>,
    /// Other operating assets (other receivables, prepayments, operating cash), in currency
    /// units.
    pub other_operating_assets: Option<Decimal>,
    /// Trade payables, in currency units.
    pub trade_payables: Option<Decimal>,
    /// Other operating liabilities (other payables, accruals), in currency units.
    pub other_operating_liabilities: Option<Decimal>,
}

/// The net financial position of a reclassified balance sheet: a total, the SPEC-0007 object,
/// or both, and optionally its current and non-current parts.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct ReclassifiedNetFinancialPosition {
    /// Net financial position, in currency units, signed, positive for net debt.
    pub total: Option<Decimal>,
    /// The net financial position with its composition (SPEC-0007 R6): gross debt − cash +
    /// signed other items.
    pub position: Option<NetFinancialPosition>,
    /// Part due within the next financial year, in currency units, signed.
    pub current: Option<Decimal>,
    /// Part due beyond the next financial year, in currency units, signed.
    pub non_current: Option<Decimal>,
}

/// Reclassified balance sheet totals, the output fields of Appendix B.1 (R5). Each is
/// optional; every total its formula allows is derived from what is given and checked against
/// a total given.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct ReclassifiedBalanceSheet {
    /// Operating fixed assets, in currency units.
    pub operating_fixed_assets: Option<Decimal>,
    /// Operating working capital, as a total or by its components.
    pub operating_working_capital: Option<OperatingWorkingCapital>,
    /// Operating provisions (provisions and employee severance indemnity treated as
    /// operating), in currency units.
    pub operating_provisions: Option<Decimal>,
    /// Invested capital, operating fixed assets + operating working capital − operating
    /// provisions, in currency units, signed.
    pub invested_capital: Option<Decimal>,
    /// Non-operating assets, each a label and a carrying amount in currency units (zero or
    /// more). An empty list states that there are none.
    pub non_operating_assets: Option<Vec<Line>>,
    /// Total funds invested, invested capital + Σ non-operating assets, in currency units,
    /// signed.
    pub total_funds_invested: Option<Decimal>,
    /// Equity, in currency units, signed.
    pub equity: Option<Decimal>,
    /// Quasi-equity items (for example subordinated shareholder loans), each a label and an
    /// amount in currency units. An empty list states that there are none.
    pub quasi_equity_items: Option<Vec<Line>>,
    /// Net financial position, as a total, the SPEC-0007 object, or both.
    pub net_financial_position: Option<ReclassifiedNetFinancialPosition>,
    /// Debt-like items, each a label, an amount in currency units and its measurement basis
    /// (the `dcf.fcff` bridge type). An empty list states that there are none.
    pub debt_like_items: Option<Vec<BridgeClaim>>,
    /// Total sources, equity + Σ quasi-equity items + net financial position + Σ debt-like
    /// items, in currency units, signed.
    pub total_sources: Option<Decimal>,
}

/// Reclassified income statement totals, the output fields of Appendix B.2 (R5). Each is
/// optional.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct ReclassifiedIncomeStatement {
    /// Revenue (A1), in currency units.
    pub revenue: Option<Decimal>,
    /// Value of production (A, core), in currency units, signed.
    pub value_of_production: Option<Decimal>,
    /// Purchases (B6), in currency units.
    pub purchases: Option<Decimal>,
    /// Materials consumed (B6 + B11), in currency units, signed.
    pub materials_consumed: Option<Decimal>,
    /// External costs, B6 + B7 + B8 + B11 + B14 net of the parts moved by adjustments (B8 also
    /// net of lease payments), in currency units, signed (E1 of 2026-11-09).
    pub external_costs: Option<Decimal>,
    /// Value added, value of production − external costs, in currency units, signed; derived
    /// from them when not given and checked against them when given.
    pub value_added: Option<Decimal>,
    /// Personnel costs (B9), in currency units.
    pub personnel_costs: Option<Decimal>,
    /// EBITDA (MOL), value added − personnel costs, in currency units, signed.
    pub ebitda: Option<Decimal>,
    /// Depreciation and amortisation: B10 a, b, plus B10 c when it is operating (P1) and lease
    /// depreciation, in currency units.
    pub depreciation_and_amortisation: Option<Decimal>,
    /// Other charges below EBITDA, B12 + B13 (+ B10 d when it is treated as operating) net of
    /// moved parts, in currency units, zero or more (E2 of 2026-11-09).
    pub other_charges_below_ebitda: Option<Decimal>,
    /// Income from operating investments, C15 of financial fixed assets treated as operating,
    /// in currency units, zero or more (P2); needed to derive operating profit.
    pub income_from_operating_investments: Option<Decimal>,
    /// Operating profit (reddito operativo, MON), EBITDA − depreciation and amortisation −
    /// other charges below EBITDA + income from operating investments, in currency units,
    /// signed; derived when not given and checked when given.
    pub operating_profit: Option<Decimal>,
    /// Investing result (accessory area), in currency units, signed.
    pub investing_result: Option<Decimal>,
    /// Profit before financing and income taxes, operating profit + investing result, in
    /// currency units, signed.
    pub profit_before_financing_and_taxes: Option<Decimal>,
    /// Non-recurring result, in currency units, signed.
    pub non_recurring_result: Option<Decimal>,
    /// Financing result, C17-bis − C17, in currency units, signed.
    pub financing_result: Option<Decimal>,
    /// Interest and other financial charges (C17), in currency units.
    pub interest_and_other_financial_charges: Option<Decimal>,
    /// Profit before tax, in currency units, signed.
    pub profit_before_tax: Option<Decimal>,
    /// Income taxes (item 20), in currency units, signed.
    pub income_taxes: Option<Decimal>,
    /// Net income (item 21), in currency units, signed.
    pub net_income: Option<Decimal>,
}

/// Destination of financial fixed assets (B.III), Decision 10.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FinancialFixedAssetsDestination {
    /// Strategic holdings used by the business: operating fixed assets.
    Operating,
    /// Surplus holdings: non-operating assets, outside invested capital, for the bridge.
    NonOperating,
    /// Financial investments: an other financial asset of the net financial position.
    FinancialAsset,
}

/// Destination of shareholder loans (D.3), Decision 6.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ShareholderLoansDestination {
    /// Gross debt of the net financial position.
    FinancialDebt,
    /// Quasi-equity, among the sources next to equity.
    QuasiEquity,
}

/// Destination of the derivative liabilities within provisions B, Decision 5.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DerivativeLiabilitiesDestination {
    /// An other financial liability of the net financial position.
    FinancialLiability,
    /// Operating provisions, with the other provisions B.
    OperatingProvision,
}

/// Where write-downs of receivables (B10 d) go (R17, Decision 17 as amended by E3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WriteDownsOfReceivables {
    /// Investing (extra-characteristic) result, below operating profit.
    Investing,
    /// Operating profit, below EBITDA, in the other charges below EBITDA (IFRS 18 practice
    /// for trade receivables). The default (E3 of 2026-11-09).
    Operating,
}

/// Where write-downs of tangible and intangible fixed assets (B10 c) go (R17, P1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WriteDownsOfFixedAssets {
    /// Investing (extra-characteristic) result, below operating profit.
    Investing,
    /// Operating profit, below EBITDA, with depreciation and amortisation (IFRS 18; OIC 12
    /// paragraph 75). The default (P1 of 2026-11-09).
    Operating,
}

/// Treatments stated once for the call (R8, R9, R17).
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct Treatments {
    /// Financial fixed assets (B.III): `operating`, `non_operating` or `financial_asset`.
    /// Needed when B.III is given and not zero. Default: none.
    pub financial_fixed_assets: Option<FinancialFixedAssetsDestination>,
    /// Shareholder loans (D.3): `financial_debt` or `quasi_equity`. Needed when the loans are
    /// given and not zero. Default: none.
    pub shareholder_loans: Option<ShareholderLoansDestination>,
    /// Derivative liabilities within provisions B: `financial_liability` or
    /// `operating_provision`. Needed when the derivative part is given and not zero.
    /// Default: none.
    pub derivative_liabilities: Option<DerivativeLiabilitiesDestination>,
    /// Write-downs of receivables (B10 d): `operating` (other charges below EBITDA) or
    /// `investing`. Default: `operating`, filled in and reported in `meta.defaults_applied`
    /// whenever a statutory year gives an income statement (E3, A4); `null` or absent means not
    /// given.
    pub write_downs_of_receivables: Option<WriteDownsOfReceivables>,
    /// Write-downs of fixed assets (B10 c): `operating` (with depreciation and amortisation) or
    /// `investing`. Default: `operating`, filled in and reported in `meta.defaults_applied`
    /// whenever a statutory year gives an income statement (P1, A4); `null` or absent means not
    /// given. Omitted from the canonical input when not given, an additive field of version 1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub write_downs_of_fixed_assets: Option<WriteDownsOfFixedAssets>,
}

/// One part of a split of financial fixed assets for one year.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FinancialFixedAssetsPart {
    /// Destination of this part.
    pub destination: FinancialFixedAssetsDestination,
    /// Amount of this part, in currency units, zero or more. The parts sum to B.III.
    pub amount: Decimal,
    /// What the part is, for example "Shares in a listed company"; not blank. Used as the
    /// label of the non-operating asset or of the item of the net financial position.
    pub label: String,
}

/// One part of a split of shareholder loans for one year.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ShareholderLoansPart {
    /// Destination of this part.
    pub destination: ShareholderLoansDestination,
    /// Amount of this part, in currency units, zero or more. The parts sum to the loans.
    pub amount: Decimal,
    /// What the part is, for example "Subordinated loan"; not blank.
    pub label: String,
    /// For a `financial_debt` part: of which due beyond the next financial year, in currency
    /// units; needed for the current and non-current parts of the net financial position.
    #[serde(default)]
    pub beyond_one_year: Option<Decimal>,
}

/// Another treatment of financial fixed assets for one year: one destination, or a split.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FinancialFixedAssetsOverride {
    /// One destination for the whole of B.III this year.
    Destination(FinancialFixedAssetsDestination),
    /// Parts with their destinations, summing to B.III of the year.
    Split(Vec<FinancialFixedAssetsPart>),
}

/// Another treatment of shareholder loans for one year: one destination, or a split.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ShareholderLoansOverride {
    /// One destination for the whole of the loans this year.
    Destination(ShareholderLoansDestination),
    /// Parts with their destinations, summing to the loans of the year.
    Split(Vec<ShareholderLoansPart>),
}

/// Per-year overrides (R9): only financial fixed assets and shareholder loans.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct TreatmentOverrides {
    /// Financial fixed assets (B.III) of this year: `{"destination": ...}` or
    /// `{"split": [...]}`.
    pub financial_fixed_assets: Option<FinancialFixedAssetsOverride>,
    /// Shareholder loans of this year: `{"destination": ...}` or `{"split": [...]}`.
    pub shareholder_loans: Option<ShareholderLoansOverride>,
}

/// Maturity of a financial receivable or of tax instalments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Maturity {
    /// Due within the next financial year (current).
    WithinOneYear,
    /// Due beyond the next financial year (non-current).
    BeyondOneYear,
}

/// Source of a `non_operating_asset` adjustment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NonOperatingSource {
    /// B.I.
    IntangibleAssets,
    /// B.II.
    TangibleAssets,
    /// C.II.
    Receivables,
    /// C.III.
    CurrentFinancialAssets,
}

/// Source of a `debt_like_provision` adjustment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProvisionSource {
    /// Provisions B treated as operating.
    Provisions,
    /// Employee severance indemnity C.
    EmployeeSeveranceIndemnity,
}

/// Source of an `investing` adjustment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InvestingSource {
    /// A5.
    OtherRevenueAndIncome,
    /// B14.
    OtherOperatingExpenses,
}

/// Source of a `non_recurring` adjustment: an income statement aggregate of Appendix A.2 that
/// accepts it. D18 and D19 are fixed in the investing result and cannot be moved (R17).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NonRecurringSource {
    /// A1.
    Revenue,
    /// A5.
    OtherRevenueAndIncome,
    /// B6.
    Purchases,
    /// B7.
    Services,
    /// B8.
    UseOfThirdPartyAssets,
    /// B9.
    PersonnelCosts,
    /// B10 a, b.
    DepreciationAndAmortisation,
    /// B10 c.
    WriteDownsOfFixedAssets,
    /// B10 d.
    WriteDownsOfReceivables,
    /// B12.
    ProvisionsForRisks,
    /// B13.
    OtherProvisions,
    /// B14.
    OtherOperatingExpenses,
    /// C15.
    IncomeFromEquityInvestments,
    /// C16.
    OtherFinancialIncome,
    /// C17.
    InterestAndOtherFinancialCharges,
    /// C17-bis.
    ExchangeGainsAndLosses,
}

/// Amounts of a `lease` adjustment for one year (R21, financial method).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LeaseAmounts {
    /// Right-of-use asset added to operating fixed assets, in currency units.
    pub asset: Decimal,
    /// Lease liability added to the net financial position, in currency units.
    pub liability: Decimal,
    /// Of which due within the next financial year, in currency units.
    pub liability_current: Decimal,
    /// Prepayment (maxicanone) removed from prepayments, in currency units.
    pub prepayment: Decimal,
    /// Lease payments of the year removed from B8, in currency units.
    pub payments: Decimal,
    /// Depreciation of the right-of-use asset for the year, in currency units.
    pub depreciation: Decimal,
    /// Interest on the lease liability for the year, added to C17, in currency units.
    pub interest: Decimal,
}

/// Generates the struct of one adjustment kind: `label`, `reason`, the fields of the kind and
/// `amounts`, one entry per year.
macro_rules! adjustment {
    ($(#[$doc:meta])* $name:ident, $amount:ty, { $($(#[$fdoc:meta])* $field:ident: $ty:ty,)* }) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
        #[serde(deny_unknown_fields)]
        pub struct $name {
            /// What the adjustment is, for example "Idle warehouse"; not blank.
            pub label: String,
            /// Why it is made; not blank (R10).
            pub reason: String,
            $($(#[$fdoc])* pub $field: $ty,)*
            /// One entry per year, in the order of `years`: the amount in currency units, zero
            /// or more, or null where the adjustment does not apply (reported). Only for
            /// statutory entries.
            pub amounts: Vec<Option<$amount>>,
        }
    };
}

adjustment!(
    /// Moves a labelled part of B.I, B.II, C.II or C.III to the non-operating assets.
    NonOperatingAssetAdjustment, Decimal, {
        /// Source aggregate: `intangible_assets`, `tangible_assets`, `receivables` or
        /// `current_financial_assets`.
        aggregate: NonOperatingSource,
    }
);
adjustment!(
    /// Moves a labelled part of C.II to the net financial position as a financial asset.
    FinancialReceivableAdjustment, Decimal, {
        /// `within_one_year` or `beyond_one_year`, for the parts of the net financial position.
        maturity: Maturity,
    }
);
adjustment!(
    /// Moves operating cash out of the net financial position into operating working capital
    /// (Decision 11).
    OperatingCashAdjustment, Decimal, {}
);
adjustment!(
    /// Moves accrued interest out of E into the net financial position (current).
    AccruedInterestAdjustment, Decimal, {}
);
adjustment!(
    /// Moves tax payables settled by instalments out of payables D into debt-like items.
    TaxInstalmentsAdjustment, Decimal, {
        /// `within_one_year` or `beyond_one_year`, reported with the item.
        maturity: Maturity,
    }
);
adjustment!(
    /// Moves trade payables with extended terms out of payables D into the net financial
    /// position (non-current).
    NonCurrentTradePayablesAdjustment, Decimal, {}
);
adjustment!(
    /// Moves a provision B or the employee severance indemnity C into debt-like items.
    DebtLikeProvisionAdjustment, Decimal, {
        /// Source aggregate: `provisions` or `employee_severance_indemnity`.
        aggregate: ProvisionSource,
        /// Optional, only with `aggregate` `employee_severance_indemnity` (P2): one entry per
        /// year, in the order of `years`, the interest cost (revaluation) of the employee
        /// severance indemnity included in B9, in currency units, zero or more and at most what
        /// remains of B9; null where not given, and only where `amounts` has an entry. It leaves
        /// personnel costs and EBITDA and enters the financing result. Not given: nothing moves
        /// and a convention says so.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        interest_cost: Option<Vec<Option<Decimal>>>,
    }
);
adjustment!(
    /// Moves dividends declared from equity to debt-like items (Decision 14).
    DividendsDeclaredAdjustment, Decimal, {}
);
adjustment!(
    /// Restates a lease by the financial method (R21, Decision 9): asset, liability, equity
    /// effect, and the income statement lines; no deferred taxes.
    LeaseAdjustment, LeaseAmounts, {}
);
adjustment!(
    /// Moves a labelled part of A5 or B14 to the investing result (Decision 7).
    InvestingAdjustment, Decimal, {
        /// Source aggregate: `other_revenue_and_income` or `other_operating_expenses`.
        aggregate: InvestingSource,
        /// Label of the non-operating asset the income or expense relates to, if any;
        /// reported only.
        #[serde(default)]
        related_non_operating_asset: Option<String>,
    }
);
adjustment!(
    /// Moves a labelled part of an income statement aggregate to the non-recurring result
    /// (Decision 7; art. 2427 n. 13).
    NonRecurringAdjustment, Decimal, {
        /// Source aggregate of Appendix A.2; D18 and D19 are not accepted (R17).
        aggregate: NonRecurringSource,
    }
);

/// One labelled adjustment of Appendix C, tagged by `kind` (R10).
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Adjustment {
    /// B.I, B.II, C.II or C.III to non-operating assets.
    NonOperatingAsset(NonOperatingAssetAdjustment),
    /// C.II to the net financial position (other financial asset).
    FinancialReceivable(FinancialReceivableAdjustment),
    /// Cash of the net financial position to operating working capital.
    OperatingCash(OperatingCashAdjustment),
    /// E to the net financial position (other financial liability).
    AccruedInterest(AccruedInterestAdjustment),
    /// Payables D to debt-like items.
    TaxInstalments(TaxInstalmentsAdjustment),
    /// Payables D to the net financial position (other financial liability).
    NonCurrentTradePayables(NonCurrentTradePayablesAdjustment),
    /// Provisions B or C to debt-like items.
    DebtLikeProvision(DebtLikeProvisionAdjustment),
    /// Equity to debt-like items.
    DividendsDeclared(DividendsDeclaredAdjustment),
    /// Financial-method restatement of a lease.
    Lease(LeaseAdjustment),
    /// A5 or B14 to the investing result.
    Investing(InvestingAdjustment),
    /// An income statement aggregate to the non-recurring result.
    NonRecurring(NonRecurringAdjustment),
}

/// The values of `kind`, in the order of Appendix C.
pub(crate) const KINDS: &[&str] = &[
    "non_operating_asset",
    "financial_receivable",
    "operating_cash",
    "accrued_interest",
    "tax_instalments",
    "non_current_trade_payables",
    "debt_like_provision",
    "dividends_declared",
    "lease",
    "investing",
    "non_recurring",
];

impl Adjustment {
    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Self::NonOperatingAsset(_) => KINDS[0],
            Self::FinancialReceivable(_) => KINDS[1],
            Self::OperatingCash(_) => KINDS[2],
            Self::AccruedInterest(_) => KINDS[3],
            Self::TaxInstalments(_) => KINDS[4],
            Self::NonCurrentTradePayables(_) => KINDS[5],
            Self::DebtLikeProvision(_) => KINDS[6],
            Self::DividendsDeclared(_) => KINDS[7],
            Self::Lease(_) => KINDS[8],
            Self::Investing(_) => KINDS[9],
            Self::NonRecurring(_) => KINDS[10],
        }
    }

    pub(crate) fn label(&self) -> &str {
        self.common().0
    }

    pub(crate) fn reason(&self) -> &str {
        self.common().1
    }

    /// Number of entries of `amounts`.
    pub(crate) fn len(&self) -> usize {
        match self {
            Self::Lease(a) => a.amounts.len(),
            other => other.amounts().map_or(0, <[_]>::len),
        }
    }

    /// The amounts of a kind with decimal amounts; `None` for `lease`.
    pub(crate) fn amounts(&self) -> Option<&[Option<Decimal>]> {
        Some(match self {
            Self::NonOperatingAsset(a) => &a.amounts,
            Self::FinancialReceivable(a) => &a.amounts,
            Self::OperatingCash(a) => &a.amounts,
            Self::AccruedInterest(a) => &a.amounts,
            Self::TaxInstalments(a) => &a.amounts,
            Self::NonCurrentTradePayables(a) => &a.amounts,
            Self::DebtLikeProvision(a) => &a.amounts,
            Self::DividendsDeclared(a) => &a.amounts,
            Self::Investing(a) => &a.amounts,
            Self::NonRecurring(a) => &a.amounts,
            Self::Lease(_) => return None,
        })
    }

    /// Whether the entry of year `i` is given (not null).
    pub(crate) fn applies_to(&self, i: usize) -> bool {
        match self {
            Self::Lease(a) => a.amounts.get(i).is_some_and(Option::is_some),
            other => other
                .amounts()
                .and_then(|a| a.get(i))
                .is_some_and(Option::is_some),
        }
    }

    fn common(&self) -> (&str, &str) {
        match self {
            Self::NonOperatingAsset(a) => (&a.label, &a.reason),
            Self::FinancialReceivable(a) => (&a.label, &a.reason),
            Self::OperatingCash(a) => (&a.label, &a.reason),
            Self::AccruedInterest(a) => (&a.label, &a.reason),
            Self::TaxInstalments(a) => (&a.label, &a.reason),
            Self::NonCurrentTradePayables(a) => (&a.label, &a.reason),
            Self::DebtLikeProvision(a) => (&a.label, &a.reason),
            Self::DividendsDeclared(a) => (&a.label, &a.reason),
            Self::Lease(a) => (&a.label, &a.reason),
            Self::Investing(a) => (&a.label, &a.reason),
            Self::NonRecurring(a) => (&a.label, &a.reason),
        }
    }
}

/// Reads an adjustment tagged by `kind`. The object is buffered as a JSON value (numbers keep
/// their exact text) and the fields of the kind are read from it, so that every error keeps the
/// JSON pointer of its field (issue #174), as `tv::compare` does for techniques.
impl<'de> Deserialize<'de> for Adjustment {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        use serde_json::Value;

        let mut map = serde_json::Map::<String, Value>::deserialize(deserializer)?;
        let Some(kind) = map.remove("kind") else {
            return Err(D::Error::missing_field("kind"));
        };
        let relative = |e: serde_path_to_error::Error<serde_json::Error>, at: Vec<String>| {
            D::Error::custom(crate::registry::nested_error(&at, e))
        };
        let kind: String =
            serde_path_to_error::deserialize(kind).map_err(|e| relative(e, vec!["kind".into()]))?;
        let rest = Value::Object(map);
        fn read<T: serde::de::DeserializeOwned>(
            v: Value,
        ) -> std::result::Result<T, serde_path_to_error::Error<serde_json::Error>> {
            serde_path_to_error::deserialize(v)
        }
        let parsed = match kind.as_str() {
            "non_operating_asset" => read(rest).map(Self::NonOperatingAsset),
            "financial_receivable" => read(rest).map(Self::FinancialReceivable),
            "operating_cash" => read(rest).map(Self::OperatingCash),
            "accrued_interest" => read(rest).map(Self::AccruedInterest),
            "tax_instalments" => read(rest).map(Self::TaxInstalments),
            "non_current_trade_payables" => read(rest).map(Self::NonCurrentTradePayables),
            "debt_like_provision" => read(rest).map(Self::DebtLikeProvision),
            "dividends_declared" => read(rest).map(Self::DividendsDeclared),
            "lease" => read(rest).map(Self::Lease),
            "investing" => read(rest).map(Self::Investing),
            "non_recurring" => {
                // R17: D18 and D19 are fixed in the investing result; the refusal names why,
                // not only the values accepted.
                if let Some(
                    fixed
                    @ ("revaluations_of_financial_assets" | "write_downs_of_financial_assets"),
                ) = rest.get("aggregate").and_then(Value::as_str)
                {
                    let message = format!(
                        "`{fixed}` cannot be moved by an adjustment: D18 and D19 are fixed in \
                         the investing result (SPEC-0005 R17, Decision 17); remove the \
                         adjustment, or name the aggregate of Appendix A.2 the non-recurring \
                         part belongs to"
                    );
                    return Err(D::Error::custom(crate::registry::nested_error_text(
                        &["aggregate".into()],
                        &message,
                    )));
                }
                read(rest).map(Self::NonRecurring)
            }
            other => {
                let list: Vec<String> = KINDS.iter().map(|k| format!("`{k}`")).collect();
                let message = format!(
                    "unknown variant `{other}`, expected one of {}",
                    list.join(", ")
                );
                return Err(D::Error::custom(crate::registry::nested_error_text(
                    &["kind".into()],
                    &message,
                )));
            }
        };
        parsed.map_err(|e| relative(e, Vec::new()))
    }
}

// ---------------------------------------------------------------- Appendix A, choice D

/// A statutory aggregate whose destination is a `default` of Appendix A (choice D).
pub(crate) struct DefaultDestination {
    /// Pointer of the aggregate below the balance sheet.
    pub key: &'static str,
    pub destination: Destination,
    /// The Appendix A row and its decision.
    pub reason: &'static str,
    /// The aggregate's amount when it is given (its total, or for B.I and B.II gross less
    /// accumulated); `None` when it is not, even if its node holds a part of it.
    pub given: fn(&StatutoryBalanceSheet) -> Option<Decimal>,
}

fn net_given(node: &Option<NetFixedAsset>) -> Option<Decimal> {
    let n = node.as_ref()?;
    n.total.or_else(|| n.gross?.checked_sub(n.accumulated?))
}

/// The `default` destinations of Appendix A.1, one row per aggregate, read by `apply_defaults`
/// for `meta.defaults_applied` (R8). Typed rows: a misspelt key or destination cannot compile
/// or silently drop a default.
pub(crate) const BALANCE_SHEET_DEFAULTS: &[DefaultDestination] = &[
    DefaultDestination {
        key: "intangible_assets",
        destination: Destination::OperatingFixedAssets,
        reason: "Appendix A.1, B.I: operating fixed assets by default; a part moves to \
                 non-operating assets only by a `non_operating_asset` adjustment (SPEC-0005 R8)",
        given: |bs| net_given(&bs.intangible_assets),
    },
    DefaultDestination {
        key: "tangible_assets",
        destination: Destination::OperatingFixedAssets,
        reason: "Appendix A.1, B.II: operating fixed assets by default; a part moves only by a \
                 `non_operating_asset` or `lease` adjustment (SPEC-0005 R8)",
        given: |bs| net_given(&bs.tangible_assets),
    },
    DefaultDestination {
        key: "receivables",
        destination: Destination::OperatingWorkingCapital,
        reason: "Appendix A.1, C.II: operating working capital by default; financial or \
                 non-operating parts move only by a `financial_receivable` or \
                 `non_operating_asset` adjustment (SPEC-0005 R8)",
        given: |bs| bs.receivables.as_ref()?.total,
    },
    DefaultDestination {
        key: "current_financial_assets",
        destination: Destination::NetFinancialPositionOtherFinancialAsset,
        reason: "Appendix A.1, C.III: an other financial asset of the net financial position by \
                 default (Decision 11); a part moves only by a `non_operating_asset` adjustment",
        given: |bs| bs.current_financial_assets,
    },
    DefaultDestination {
        key: "cash",
        destination: Destination::NetFinancialPositionCash,
        reason: "Appendix A.1, C.IV: all cash in the net financial position (Decision 11); \
                 operating cash moves only by an `operating_cash` adjustment",
        given: |bs| bs.cash,
    },
    DefaultDestination {
        key: "prepayments_and_accrued_income",
        destination: Destination::OperatingWorkingCapital,
        reason: "Appendix A.1, assets D: operating working capital by default (SPEC-0005 R8)",
        given: |bs| bs.prepayments_and_accrued_income.as_ref()?.total,
    },
    DefaultDestination {
        key: "provisions",
        destination: Destination::OperatingProvisions,
        reason: "Appendix A.1, provisions B: operating provisions by default, never in the net \
                 financial position (Decision 5; ESMA32-382-1138 guideline 39); debt-like only \
                 by a `debt_like_provision` adjustment",
        given: |bs| bs.provisions.as_ref()?.total,
    },
    DefaultDestination {
        key: "employee_severance_indemnity",
        destination: Destination::OperatingProvisions,
        reason: "Appendix A.1, employee severance indemnity C: operating provisions by default \
                 (Decision 5); debt-like only by a `debt_like_provision` adjustment",
        given: |bs| bs.employee_severance_indemnity.as_ref()?.total,
    },
    DefaultDestination {
        key: "payables",
        destination: Destination::OperatingWorkingCapital,
        reason: "Appendix A.1, payables D less financial debt and shareholder loans: operating \
                 working capital by default; parts move only by a `tax_instalments` or \
                 `non_current_trade_payables` adjustment (SPEC-0005 R8)",
        given: |bs| bs.payables.as_ref()?.total,
    },
    DefaultDestination {
        key: "accruals_and_deferred_income",
        destination: Destination::OperatingWorkingCapital,
        reason: "Appendix A.1, liabilities E: operating working capital by default; accrued \
                 interest moves only by an `accrued_interest` adjustment (SPEC-0005 R8)",
        given: |bs| bs.accruals_and_deferred_income.as_ref()?.total,
    },
];

/// An income statement aggregate whose destination is a `default` of Appendix A.2 (choice D).
pub(crate) struct IncomeStatementDefault {
    /// Pointer of the aggregate below the income statement.
    pub key: &'static str,
    pub destination: Destination,
    /// The Appendix A.2 row and its decision.
    pub reason: &'static str,
    /// The aggregate's amount when it is given.
    pub given: fn(&StatutoryIncomeStatement) -> Option<Decimal>,
    /// Whether the analyst stated the treatment that governs this aggregate (B10 c, B10 d): a
    /// treatment stated is not a default, so the row is not listed.
    pub stated: fn(&Treatments) -> bool,
}

/// The `default` destinations of Appendix A.2, read by `apply_defaults` for
/// `meta.defaults_applied` (R8, R17, R18). Typed rows, as for the balance sheet.
pub(crate) const INCOME_STATEMENT_DEFAULTS: &[IncomeStatementDefault] = &[
    IncomeStatementDefault {
        key: "other_revenue_and_income",
        destination: Destination::ValueOfProduction,
        reason: "Appendix A.2, A5: core, in the value of production by default (Decision 7, \
                 R18); investing or non-recurring parts move only by an `investing` or \
                 `non_recurring` adjustment",
        given: |is| is.other_revenue_and_income,
        stated: |_| false,
    },
    IncomeStatementDefault {
        key: "other_operating_expenses",
        destination: Destination::ExternalCosts,
        reason: "Appendix A.2, B14: core, in external costs by default (Decision 7, R18, E1); \
                 investing or non-recurring parts move only by an `investing` or \
                 `non_recurring` adjustment",
        given: |is| is.other_operating_expenses,
        stated: |_| false,
    },
    IncomeStatementDefault {
        key: "depreciation_and_write_downs/write_downs_of_fixed_assets",
        destination: Destination::DepreciationAndAmortisation,
        reason: WRITE_DOWNS_OF_FIXED_ASSETS_DEFAULT,
        given: |is| {
            is.depreciation_and_write_downs
                .as_ref()?
                .write_downs_of_fixed_assets
        },
        stated: |t| t.write_downs_of_fixed_assets.is_some(),
    },
    IncomeStatementDefault {
        key: "depreciation_and_write_downs/write_downs_of_receivables",
        destination: Destination::OtherChargesBelowEbitda,
        reason: WRITE_DOWNS_OF_RECEIVABLES_DEFAULT,
        given: |is| {
            is.depreciation_and_write_downs
                .as_ref()?
                .write_downs_of_receivables
        },
        stated: |t| t.write_downs_of_receivables.is_some(),
    },
];

/// R17, E3: the default destination of B10 d.
pub(crate) const WRITE_DOWNS_OF_RECEIVABLES_DEFAULT: &str = concat!(
    "Appendix A.2, B10 d: operating by default, in the other charges below EBITDA (E3 of ",
    "2026-11-09; IFRS 18 practice for the impairment of trade receivables); the treatment ",
    "`write_downs_of_receivables: investing` places them in the investing result"
);

/// R17, P1: the default destination of B10 c.
pub(crate) const WRITE_DOWNS_OF_FIXED_ASSETS_DEFAULT: &str = concat!(
    "Appendix A.2, B10 c: operating by default, below EBITDA with depreciation and ",
    "amortisation (P1 of 2026-11-09; IFRS 18; OIC 12 paragraph 75); the treatment ",
    "`write_downs_of_fixed_assets: investing` places them in the investing result; a one-off ",
    "impairment is isolated with a `non_recurring` adjustment"
);

/// An item of a statutory balance sheet that the computation takes as zero when it is not
/// given: item A (D1), an "of which" part (D2, D3), or a separate line presented within another
/// aggregate (assets D within C.II, liabilities E within D, art. 2435-bis).
pub(crate) struct TakenAsZero {
    /// Pointer of the item below the balance sheet, as it would be written in the input.
    pub key: &'static str,
    /// Whether the item is taken as zero in this balance sheet: not given, and, for a separate
    /// line, presented within another aggregate.
    pub applies: fn(&StatutoryBalanceSheet) -> bool,
    /// Why, citing the rule.
    pub reason: &'static str,
    /// A separate line of the balance sheet (item A, line D, line E), not an "of which" part:
    /// only such an item can make the funds invested differ from the sources (M1), because an
    /// "of which" part moves an amount within one side.
    pub separate_line: bool,
}

/// Every item taken as zero when not given, in the order of art. 2424.
pub(crate) const TAKEN_AS_ZERO: &[TakenAsZero] = &[
    TakenAsZero {
        key: "subscribed_capital_unpaid",
        applies: |bs| bs.subscribed_capital_unpaid.is_none(),
        reason: "assets A, subscribed capital unpaid, not given: zero (SPEC-0005 D1); the same \
                 zero is deducted from equity and from total assets",
        separate_line: true,
    },
    TakenAsZero {
        key: "receivables/subscribed_capital_unpaid",
        applies: |bs| {
            bs.receivables
                .as_ref()
                .and_then(|n| n.subscribed_capital_unpaid)
                .is_none()
        },
        reason: "\"of which\" part not given: none of item A is presented within receivables \
                 C.II (SPEC-0005 D2)",
        separate_line: false,
    },
    TakenAsZero {
        key: "receivables/prepayments_and_accrued_income",
        applies: |bs| {
            bs.receivables
                .as_ref()
                .and_then(|n| n.prepayments_and_accrued_income)
                .is_none()
        },
        reason: "\"of which\" part not given: none of assets D is presented within receivables \
                 C.II (SPEC-0005 D2)",
        separate_line: false,
    },
    TakenAsZero {
        key: "prepayments_and_accrued_income/total",
        applies: |bs| {
            bs.prepayments_and_accrued_income.is_none()
                && bs
                    .receivables
                    .as_ref()
                    .is_some_and(|n| n.prepayments_and_accrued_income.is_some())
        },
        reason: "assets D presented within receivables C.II (art. 2435-bis), as given in \
                 `receivables/prepayments_and_accrued_income`: no separate line D",
        separate_line: true,
    },
    TakenAsZero {
        key: "provisions/of_which_derivative_liabilities",
        applies: |bs| {
            bs.provisions
                .as_ref()
                .and_then(|n| n.of_which_derivative_liabilities)
                .is_none()
        },
        reason: "\"of which\" part not given: no derivative liabilities within provisions B \
                 (SPEC-0005 D2, D3)",
        separate_line: false,
    },
    TakenAsZero {
        key: "payables/accruals_and_deferred_income",
        applies: |bs| {
            bs.payables
                .as_ref()
                .and_then(|n| n.accruals_and_deferred_income)
                .is_none()
        },
        reason: "\"of which\" part not given: none of liabilities E is presented within \
                 payables D (SPEC-0005 D2)",
        separate_line: false,
    },
    TakenAsZero {
        key: "accruals_and_deferred_income/total",
        applies: |bs| {
            bs.accruals_and_deferred_income.is_none()
                && bs
                    .payables
                    .as_ref()
                    .is_some_and(|n| n.accruals_and_deferred_income.is_some())
        },
        reason: "liabilities E presented within payables D (art. 2435-bis), as given in \
                 `payables/accruals_and_deferred_income`: no separate line E",
        separate_line: true,
    },
];
