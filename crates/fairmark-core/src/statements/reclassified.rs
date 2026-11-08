//! The reclassified form of the balance sheet (R5, R12): totals already reorganised, each
//! given or derived by its formula of Appendix B.1 from what is given, every given total
//! checked exactly against its derived value, and the identity of R12 checked when both sides
//! are known (`FM-E026`). Treatments and adjustments do not apply here.

use rust_decimal::Decimal;

use super::balance_sheet::Outputs;
use super::checks::{Checks, Side};
use super::input::{ReclassifiedBalanceSheet, ReorganiseInput};
use super::partial::{total, Partial, Sign};
use super::{
    BalanceSheet, CurrentNonCurrent, NetFinancialPositionBlock, OperatingBlock, Sources, YearBody,
};
use crate::error::Result;
use crate::explain::Explain;

use Sign::{Minus, Plus};

const FIX: &str = "correct the total or its components so that the reclassified statement \
    adds up, or leave out the total to have it derived";

/// A total given, checked against its derived value; the given value prevails when both exist.
fn given_or_derived(
    checks: &mut Checks,
    relation: &'static str,
    name: &str,
    given: (Option<Decimal>, String),
    derived: &Partial,
    components: Vec<String>,
) -> Result<Partial> {
    checks.equal(
        relation,
        Side::new(name.to_owned(), vec![given.1.clone()], given.0),
        Side::new(
            format!("{name} derived by Appendix B.1"),
            components,
            derived.value(),
        ),
        FIX,
    )?;
    Ok(match given.0 {
        Some(v) => Partial::Known(v),
        None if derived.is_known() => derived.clone(),
        None => Partial::lacking(given.1),
    })
}

/// The reorganised balance sheet of a reclassified year `i`.
pub(crate) fn reclassified(
    input: &ReorganiseInput,
    i: usize,
    year: &str,
    ex: &mut Explain,
) -> Result<YearBody> {
    let bs: ReclassifiedBalanceSheet = input.years[i]
        .reclassified
        .as_ref()
        .and_then(|r| r.balance_sheet.clone())
        .unwrap_or_default();
    let base = format!("/years/{i}/reclassified/balance_sheet");
    let p = |k: &str| format!("{base}/{k}");
    let mut checks = Checks::new(year);

    let owc_at = p("operating_working_capital");
    let (owc_given, owc_derived, twc) = match &bs.operating_working_capital {
        None => {
            let m = Partial::lacking(owc_at.clone());
            (None, m.clone(), m)
        }
        Some(n) => {
            let part = |v: Option<Decimal>, k: &str| Partial::of(v, || format!("{owc_at}/{k}"));
            let tr = part(n.trade_receivables, "trade_receivables");
            let inv = part(n.inventories, "inventories");
            let oa = part(n.other_operating_assets, "other_operating_assets");
            let tp = part(n.trade_payables, "trade_payables");
            let ol = part(n.other_operating_liabilities, "other_operating_liabilities");
            let derived = total(
                ex,
                &format!("Operating working capital from its components ({year})"),
                &[
                    (Plus, "trade receivables", &tr),
                    (Plus, "inventories", &inv),
                    (Plus, "other operating assets", &oa),
                    (Minus, "trade payables", &tp),
                    (Minus, "other operating liabilities", &ol),
                ],
            )?;
            let twc = total(
                ex,
                &format!("Trade working capital ({year})"),
                &[
                    (Plus, "trade receivables", &tr),
                    (Plus, "inventories", &inv),
                    (Minus, "trade payables", &tp),
                ],
            )?;
            (n.total, derived, twc)
        }
    };
    let owc_total_at = if bs.operating_working_capital.is_some() {
        format!("{owc_at}/total")
    } else {
        owc_at.clone()
    };
    let owc = given_or_derived(
        &mut checks,
        "operating_working_capital_total",
        "operating working capital",
        (owc_given, owc_total_at),
        &owc_derived,
        vec![owc_at.clone()],
    )?;
    let ofa = Partial::of(bs.operating_fixed_assets, || p("operating_fixed_assets"));
    let op_prov = Partial::of(bs.operating_provisions, || p("operating_provisions"));
    let ic_derived = total(
        ex,
        &format!("Invested capital ({year})"),
        &[
            (Plus, "operating fixed assets", &ofa),
            (Plus, "operating working capital", &owc),
            (Minus, "operating provisions", &op_prov),
        ],
    )?;
    let ic = given_or_derived(
        &mut checks,
        "invested_capital_total",
        "invested capital",
        (bs.invested_capital, p("invested_capital")),
        &ic_derived,
        vec![
            p("operating_fixed_assets"),
            owc_at.clone(),
            p("operating_provisions"),
        ],
    )?;
    // R11: the total of each list is a step with every line substituted.
    let lines_total = |ex: &mut Explain,
                       lines: Option<Vec<(String, Decimal)>>,
                       key: &str,
                       title: &str|
     -> Result<Partial> {
        let Some(lines) = lines else {
            return Ok(Partial::lacking(p(key)));
        };
        let amounts: Vec<(String, Partial)> = lines
            .into_iter()
            .map(|(label, v)| (label, Partial::Known(v)))
            .collect();
        let terms: Vec<(Sign, &str, &Partial)> = amounts
            .iter()
            .map(|(label, a)| (Plus, label.as_str(), a))
            .collect();
        total(ex, &format!("{title} ({year})"), &terms)
    };
    let noa_total = lines_total(
        ex,
        bs.non_operating_assets
            .as_ref()
            .map(|l| l.iter().map(|x| (x.label.clone(), x.value)).collect()),
        "non_operating_assets",
        "Non-operating assets",
    )?;
    let tfi_derived = total(
        ex,
        &format!("Total funds invested ({year})"),
        &[
            (Plus, "invested capital", &ic),
            (Plus, "non-operating assets", &noa_total),
        ],
    )?;
    let tfi = given_or_derived(
        &mut checks,
        "total_funds_invested_total",
        "total funds invested",
        (bs.total_funds_invested, p("total_funds_invested")),
        &tfi_derived,
        vec![p("invested_capital"), p("non_operating_assets")],
    )?;
    let equity = Partial::of(bs.equity, || p("equity"));
    let q_total = lines_total(
        ex,
        bs.quasi_equity_items
            .as_ref()
            .map(|l| l.iter().map(|x| (x.label.clone(), x.value)).collect()),
        "quasi_equity_items",
        "Quasi-equity items",
    )?;
    let dl_total = lines_total(
        ex,
        bs.debt_like_items
            .as_ref()
            .map(|l| l.iter().map(|x| (x.label.clone(), x.value)).collect()),
        "debt_like_items",
        "Debt-like items",
    )?;
    let nfp_at = p("net_financial_position");
    let (position, nfp_total, current, non_current, position_why) = match &bs.net_financial_position
    {
        None => {
            let m = Partial::lacking(nfp_at.clone());
            (None, m.clone(), m.clone(), m.clone(), m)
        }
        Some(n) => {
            let from_position = match &n.position {
                Some(pos) => Partial::Known(pos.total(ex, year)?),
                None => Partial::lacking(format!("{nfp_at}/position")),
            };
            let nfp_total = given_or_derived(
                &mut checks,
                "net_financial_position_total",
                "net financial position",
                (n.total, format!("{nfp_at}/total")),
                &from_position,
                vec![format!("{nfp_at}/position")],
            )?;
            let current = Partial::of(n.current, || format!("{nfp_at}/current"));
            let non_current = Partial::of(n.non_current, || format!("{nfp_at}/non_current"));
            let parts = Partial::sum(&[(Plus, &current), (Plus, &non_current)])?;
            checks.equal(
                "net_financial_position_maturity",
                Side::new(
                    "net financial position",
                    vec![format!("{nfp_at}/total")],
                    nfp_total.value(),
                ),
                Side::new(
                    "its current + non-current parts",
                    vec![format!("{nfp_at}/current"), format!("{nfp_at}/non_current")],
                    parts.value(),
                ),
                FIX,
            )?;
            (
                n.position.clone(),
                nfp_total,
                current,
                non_current,
                from_position,
            )
        }
    };
    let ts_derived = total(
        ex,
        &format!("Total sources ({year})"),
        &[
            (Plus, "equity", &equity),
            (Plus, "quasi-equity items", &q_total),
            (Plus, "net financial position", &nfp_total),
            (Plus, "debt-like items", &dl_total),
        ],
    )?;
    let ts = given_or_derived(
        &mut checks,
        "total_sources_total",
        "total sources",
        (bs.total_sources, p("total_sources")),
        &ts_derived,
        vec![
            p("equity"),
            p("quasi_equity_items"),
            nfp_at.clone(),
            p("debt_like_items"),
        ],
    )?;
    checks.equal(
        "funds_invested_equals_sources",
        Side::new(
            "total funds invested",
            vec![p("total_funds_invested")],
            tfi.value(),
        ),
        Side::new("total sources", vec![p("total_sources")], ts.value()),
        "in a reorganised balance sheet the funds invested equal the sources (R12): correct \
         the total that differs from the statement",
    )?;
    let excess_cash = match &position {
        Some(pos) => total(
            ex,
            &format!("Excess cash ({year})"),
            &[(
                Plus,
                "cash of the net financial position",
                &Partial::Known(pos.cash),
            )],
        )?,
        None => position_why.clone(),
    };

    let statutory = Partial::lacking(format!("/years/{i}/statutory/balance_sheet"));
    let mut out = Outputs::new(i, Default::default());
    let b = "balance_sheet";
    let c = "current_non_current";
    let balance_sheet = BalanceSheet {
        operating: OperatingBlock {
            operating_fixed_assets: out.amount(b, "operating/operating_fixed_assets", &ofa),
            operating_working_capital: out.amount(b, "operating/operating_working_capital", &owc),
            trade_working_capital: out.amount(b, "operating/trade_working_capital", &twc),
            operating_provisions: out.amount(b, "operating/operating_provisions", &op_prov),
            invested_capital: out.amount(b, "operating/invested_capital", &ic),
        },
        non_operating_assets: out.list(
            b,
            "non_operating_assets",
            bs.non_operating_assets.clone(),
            &noa_total,
        ),
        non_operating_assets_total: out.amount(b, "non_operating_assets_total", &noa_total),
        total_funds_invested: out.amount(b, "total_funds_invested", &tfi),
        sources: Sources {
            equity: out.amount(b, "sources/equity", &equity),
            quasi_equity_items: out.list(
                b,
                "sources/quasi_equity_items",
                bs.quasi_equity_items.clone(),
                &q_total,
            ),
            quasi_equity_items_total: out.amount(b, "sources/quasi_equity_items_total", &q_total),
            net_financial_position: NetFinancialPositionBlock {
                position: out.list(
                    b,
                    "sources/net_financial_position/position",
                    position,
                    &position_why,
                ),
                total: out.amount(b, "sources/net_financial_position/total", &nfp_total),
                current: out.amount(b, "sources/net_financial_position/current", &current),
                non_current: out.amount(
                    b,
                    "sources/net_financial_position/non_current",
                    &non_current,
                ),
                composition: out.list(
                    b,
                    "sources/net_financial_position/composition",
                    None,
                    &statutory,
                ),
            },
            debt_like_items: out.list(
                b,
                "sources/debt_like_items",
                bs.debt_like_items.clone(),
                &dl_total,
            ),
            debt_like_items_total: out.amount(b, "sources/debt_like_items_total", &dl_total),
            total_sources: out.amount(b, "sources/total_sources", &ts),
        },
        excess_cash: out.amount(b, "excess_cash", &excess_cash),
    };
    let current_non_current = CurrentNonCurrent {
        fixed_assets: out.amount(c, "fixed_assets", &statutory),
        current_assets: out.amount(c, "current_assets", &statutory),
        inventories: out.amount(c, "inventories", &statutory),
        cash: out.amount(c, "cash", &statutory),
        total_assets: out.amount(c, "total_assets", &statutory),
        current_liabilities: out.amount(c, "current_liabilities", &statutory),
        non_current_liabilities: out.amount(c, "non_current_liabilities", &statutory),
    };
    Ok(YearBody {
        balance_sheet,
        current_non_current,
        assignments: Vec::new(),
        adjustments_applied: Vec::new(),
        checks: checks.map,
        not_computed: out.finish(),
    })
}
