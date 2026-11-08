//! `tv.liquidation`: assets realised and liabilities settled at the horizon (SPEC-0006 R7).

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{domain, invalid, on, Checked, Line, TerminalBasis, NOT_DISCOUNTED};
use crate::error::Result;
use crate::explain::Explain;
use crate::method::Method;
use crate::num::{add, sub};

/// Input of `tv.liquidation`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LiquidationInput {
    /// Assets with their realisable values at N, in currency units, each zero or positive; at
    /// least one.
    pub assets: Vec<Line>,
    /// Liabilities settled at N, in currency units, each zero or positive.
    pub liabilities: Vec<Line>,
    /// Costs of the liquidation (legal fees, closure, severance), in currency units, each zero
    /// or positive.
    pub liquidation_costs: Vec<Line>,
    /// Tax on the gains realised, as an amount in currency units, zero or positive.
    pub tax_on_gains: Decimal,
    /// `enterprise` (the liabilities listed exclude financial debt: the value is an enterprise
    /// value) or `equity` (financial debt included: the value is an equity value). Required, no
    /// default.
    pub basis: TerminalBasis,
}

/// Output of `tv.liquidation`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct LiquidationOutput {
    /// Net liquidation value at N, not discounted, in currency units; negative when the
    /// liabilities and costs exceed the realisable values.
    pub terminal_value: Decimal,
    /// Sum of the realisable values of the assets, in currency units.
    pub total_assets: Decimal,
    /// Sum of the liabilities, in currency units.
    pub total_liabilities: Decimal,
    /// Sum of the liquidation costs, in currency units.
    pub total_liquidation_costs: Decimal,
    /// Tax on gains, in currency units, as given.
    pub tax_on_gains: Decimal,
    /// `enterprise` or `equity`, as declared.
    pub basis: TerminalBasis,
}

/// The lines of one group: a label and a value of zero or more.
fn check_lines(field: &str, lines: &[Line]) -> Checked {
    for (i, line) in lines.iter().enumerate() {
        if line.label.trim().is_empty() {
            return Err(on(format!("{field}/{i}/label"))(invalid(format!(
                "every line of {field} needs a label"
            ))));
        }
        if line.value < Decimal::ZERO {
            return Err(on(format!("{field}/{i}/value"))(domain(format!(
                "{field} '{}' cannot be negative (got {})",
                line.label, line.value
            ))));
        }
    }
    Ok(())
}

/// Validates the input; each error names its field.
pub(crate) fn check(input: &LiquidationInput) -> Checked {
    if input.assets.is_empty() {
        return Err(on("assets")(invalid(
            "give at least one asset to liquidate",
        )));
    }
    let tax = input.tax_on_gains;
    if tax < Decimal::ZERO {
        return Err(on("tax_on_gains")(domain(format!(
            "the tax on gains cannot be negative (got {tax})"
        ))));
    }
    check_lines("assets", &input.assets)?;
    check_lines("liabilities", &input.liabilities)?;
    check_lines("liquidation_costs", &input.liquidation_costs)?;
    Ok(())
}

/// Adds the lines of one group, one explain step per line, then the total. The lines are
/// checked by [`check`].
fn total(ex: &mut Explain, group: &str, lines: &[Line]) -> Result<Decimal> {
    let mut sum = Decimal::ZERO;
    let mut terms = Vec::with_capacity(lines.len());
    for line in lines {
        ex.step(format!("{group}: {}", line.label), "given", line.value);
        sum = add(sum, line.value)?;
        terms.push(line.value.to_string());
    }
    let formula = if terms.is_empty() {
        "no lines".to_owned()
    } else {
        terms.join(" + ")
    };
    ex.step(format!("Total {}", group.to_lowercase()), formula, sum);
    Ok(sum)
}

/// Net liquidation value = Σ assets − Σ liabilities − Σ liquidation costs − tax on gains.
pub fn liquidation(input: &LiquidationInput) -> Result<(LiquidationOutput, Explain)> {
    check(input).map_err(|e| e.error)?;
    let tax = input.tax_on_gains;
    let mut ex = Explain::new(
        "tv.liquidation",
        "TV_N = Σ realisable values − Σ liabilities − Σ liquidation costs − tax on gains",
    );
    ex.convention(NOT_DISCOUNTED);
    ex.convention("asset values realisable at the horizon N, liabilities settled at N");
    ex.convention("tax on gains given as an amount (not computed from a rate)");
    ex.convention("a negative net value is returned as is (no floor at zero)");
    ex.convention(match input.basis {
        TerminalBasis::Enterprise => {
            "basis: enterprise (declared): the liabilities listed exclude financial debt, the \
            value is an enterprise value"
        }
        TerminalBasis::Equity => {
            "basis: equity (declared): the liabilities listed include financial debt, the value \
            is an equity value"
        }
    });
    let a = total(&mut ex, "Assets", &input.assets)?;
    let l = total(&mut ex, "Liabilities", &input.liabilities)?;
    let c = total(&mut ex, "Liquidation costs", &input.liquidation_costs)?;
    ex.step("Tax on gains", "given", tax);
    let tv = sub(sub(sub(a, l)?, c)?, tax)?;
    ex.step(
        "Net liquidation value at N",
        format!("{a} − {l} − {c} − {tax}"),
        tv,
    );
    Ok((
        LiquidationOutput {
            terminal_value: tv,
            total_assets: a,
            total_liabilities: l,
            total_liquidation_costs: c,
            tax_on_gains: tax,
            basis: input.basis,
        },
        ex,
    ))
}

/// `tv.liquidation`: see the methodology page.
pub struct Liquidation;

impl Method for Liquidation {
    const ID: &'static str = "tv.liquidation";
    const TITLE: &'static str = "Terminal value, liquidation";
    const DESCRIPTION: &'static str =
        "Net liquidation value at the horizon N: realisable values of labelled assets less \
        labelled liabilities, liquidation costs and the tax on gains, every line in explain.";
    const VERSION: &'static str = "1";
    type Input = LiquidationInput;
    type Output = LiquidationOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        liquidation(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tv::d;

    fn line(label: &str, v: &str) -> Line {
        Line {
            label: label.to_owned(),
            value: d(v),
        }
    }

    fn input() -> LiquidationInput {
        LiquidationInput {
            assets: vec![line("Land", "800"), line("Plant", "300")],
            liabilities: vec![line("Bank loan", "600")],
            liquidation_costs: vec![],
            tax_on_gains: d("30"),
            basis: TerminalBasis::Equity,
        }
    }

    #[test]
    fn every_line_is_an_explain_step() {
        let (out, ex) = liquidation(&input()).unwrap();
        assert_eq!(out.terminal_value, d("470"));
        let labels: Vec<_> = ex.steps.iter().map(|s| s.label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "Assets: Land",
                "Assets: Plant",
                "Total assets",
                "Liabilities: Bank loan",
                "Total liabilities",
                "Total liquidation costs",
                "Tax on gains",
                "Net liquidation value at N",
            ]
        );
    }

    #[test]
    fn negative_lines_and_empty_labels_are_refused() {
        let mut i = input();
        i.liabilities[0].value = d("-1");
        assert_eq!(liquidation(&i).unwrap_err().code(), "FM-E018");
        let mut i = input();
        i.tax_on_gains = d("-1");
        assert_eq!(liquidation(&i).unwrap_err().code(), "FM-E018");
        let mut i = input();
        i.assets[1].label = " ".to_owned();
        assert_eq!(liquidation(&i).unwrap_err().code(), "FM-E001");
        let mut i = input();
        i.assets.clear();
        assert_eq!(liquidation(&i).unwrap_err().code(), "FM-E001");
    }

    #[test]
    fn a_negative_value_is_returned_as_is() {
        let mut i = input();
        i.liabilities[0].value = d("2000");
        let (out, _) = liquidation(&i).unwrap();
        assert_eq!(out.terminal_value, d("-930"));
    }
}
