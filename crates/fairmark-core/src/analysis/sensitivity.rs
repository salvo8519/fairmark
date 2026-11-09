//! `analysis.sensitivity`: one output of any method across a one-way or two-way table of
//! inputs, the data table of spreadsheet models (SPEC-0008 R15 to R17).

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    base_case, base_value, output, pointer, recorded, with_values, BaseInput, Inner, InnerMethod,
    Outcome,
};
use crate::error::{FairmarkError, Found, Result};
use crate::explain::Explain;
use crate::method::Method;

/// One axis of the table: a variable and the values it takes.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Axis {
    /// RFC 6901 JSON Pointer to a decimal input of the inner method, for example
    /// `/discount_rate`.
    pub variable: String,
    /// Values of the variable, in the variable's units, in the order of the table; at least one.
    pub values: Vec<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SensitivityInput {
    /// Id of the method to evaluate, for example `dcf.fcff`. Analysis methods are not accepted.
    pub method: String,
    /// Input of that method, complete: the base case.
    pub base_input: BaseInput,
    /// RFC 6901 JSON Pointer to the decimal output reported in each cell, in the `value` of
    /// the inner result, for example `/enterprise_value`.
    pub output: String,
    /// The variable that changes along the rows, and its values.
    pub rows: Axis,
    /// Optional second variable, along the columns, different from the row variable. Absent:
    /// a one-way table (one column).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub columns: Option<Axis>,
}

/// Output of `analysis.sensitivity`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct SensitivityOutput {
    /// The method evaluated, at the version run.
    pub inner_method: InnerMethod,
    /// The output pointer reported in each cell.
    pub output: String,
    /// The output for `base_input` unchanged, in the output's units.
    pub base: Decimal,
    /// The row axis, as given.
    pub rows: Axis,
    /// The column axis, as given; absent for a one-way table.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub columns: Option<Axis>,
    /// `cells[i][j]`: the output with the row variable at `rows.values[i]` and the column
    /// variable at `columns.values[j]` (one column in a one-way table), as `{"value": ...}`, or
    /// `{"error": {"code", "message"}}` when the inner method refused that combination.
    pub cells: Vec<Vec<Outcome<Decimal>>>,
    /// Number of cells holding an error.
    pub failed_cells: u64,
}

fn check_axis(
    name: &str,
    axis: &Axis,
    base: &serde_json::Value,
    inner: Option<&Inner>,
    found: &mut Found,
) {
    let at = |e: FairmarkError| e.located(format!("/{name}/variable"));
    match inner {
        Some(inner) => found.check(inner.check_variable(base, &axis.variable).map_err(at)),
        None => found.check(
            pointer::read_decimal(base, &axis.variable)
                .map(|_| ())
                .map_err(at),
        ),
    };
    if axis.values.is_empty() {
        found.push(
            FairmarkError::InvalidInput(format!("{name}.values is empty"))
                .at(format!("/{name}/values"), crate::error::Reason::Length),
        );
    }
}

pub fn sensitivity(input: &SensitivityInput) -> Result<(SensitivityOutput, Explain)> {
    // Every input error of the request and of its base input is reported (ADR 0014 G5).
    let mut found = Found::new();
    let inner = found.take(Inner::new(&input.method));
    let base = base_value(&input.base_input);
    check_axis("rows", &input.rows, &base, inner.as_ref(), &mut found);
    if let Some(columns) = &input.columns {
        check_axis("columns", columns, &base, inner.as_ref(), &mut found);
        if columns.variable == input.rows.variable {
            found.push(
                FairmarkError::InvalidInput(format!(
                    "rows and columns use the same variable {}",
                    columns.variable
                ))
                .at("/columns/variable", crate::error::Reason::Conflict)
                .between(["/columns/variable", "/rows/variable"]),
            );
        }
    }
    found.check(pointer::check_syntax(&input.output).map_err(|e| e.located("/output")));
    if let Some(inner) = &inner {
        let n_rows = input.rows.values.len() as u64;
        let n_cols = input.columns.as_ref().map_or(1, |c| c.values.len() as u64);
        inner.check_input(
            &base,
            n_rows.saturating_mul(n_cols).saturating_add(1),
            &mut found,
        );
    }
    found.finish()?;
    let mut inner = inner.ok_or(FairmarkError::Internal("inner method not checked"))?;
    let n_rows = input.rows.values.len() as u64;
    let n_cols = input.columns.as_ref().map_or(1, |c| c.values.len() as u64);
    // R5: the base case plus one evaluation per cell, refused before the first evaluation.
    inner.reserve(n_rows.saturating_mul(n_cols).saturating_add(1), &base)?;

    let base_output = base_case(&mut inner, &base, &[input.output.as_str()], |_| {
        "/output".to_owned()
    })?[0];
    let mut ex = Explain::new(
        Sensitivity::ID,
        if input.columns.is_some() {
            "cell(i, j) = output(base input with rows.variable = rows.values[i] and \
            columns.variable = columns.values[j])"
        } else {
            "cell(i) = output(base input with rows.variable = rows.values[i])"
        },
    );
    ex.step(
        "Base case",
        format!("{} with base_input unchanged", input.output),
        base_output,
    );

    let mut cells = Vec::with_capacity(input.rows.values.len());
    let mut failed = 0u64;
    for (i, row_value) in input.rows.values.iter().enumerate() {
        let mut row = Vec::with_capacity(n_cols as usize);
        for j in 0..n_cols as usize {
            let mut set = vec![(input.rows.variable.as_str(), *row_value)];
            if let Some(columns) = &input.columns {
                set.push((columns.variable.as_str(), columns.values[j]));
            }
            let candidate = with_values(&base, &set)?;
            // An inner error is recorded in the cell (R16); an output pointer that does not
            // resolve is a wrong request, not a refused combination, so it ends it (R3).
            let cell = match recorded(inner.evaluate(&candidate))? {
                Outcome::Value { value } => Outcome::Value {
                    value: output(&value, &input.output).map_err(|e| e.located("/output"))?,
                },
                Outcome::Error { error } => Outcome::Error { error },
            };
            let label = set
                .iter()
                .map(|(p, v)| format!("{p} = {v}"))
                .collect::<Vec<_>>()
                .join(", ");
            match &cell {
                Outcome::Value { value } => ex.step(
                    format!("Cell ({}, {}): {label}", i + 1, j + 1),
                    format!("{} = {value}", input.output),
                    *value,
                ),
                Outcome::Error { .. } => failed += 1,
            }
            row.push(cell);
        }
        cells.push(row);
    }

    ex.convention(format!(
        "inner method {} version {}, evaluated through the registry",
        inner.id(),
        inner.info().version
    ));
    if let Some(note) = inner.status_convention() {
        ex.convention(note);
    }
    ex.convention(format!("each cell reports {}", input.output));
    ex.convention(
        "evaluation order: base case, then the cells row by row (row-major), columns in the \
        order given",
    );
    ex.convention(
        "a cell the inner method refuses holds its error code and message and the table goes \
        on; a defect in Fairmark (FM-E012) aborts the request",
    );
    ex.convention("the variables are written into the inner input as decimal strings");
    ex.convention(format!(
        "{} evaluations of the inner method; failed cells: {failed}",
        inner.evaluations()
    ));

    Ok((
        SensitivityOutput {
            inner_method: inner.info(),
            output: input.output.clone(),
            base: base_output,
            rows: input.rows.clone(),
            columns: input.columns.clone(),
            cells,
            failed_cells: failed,
        },
        ex,
    ))
}

// ---- method registrations

/// `analysis.sensitivity`.
pub struct Sensitivity;

impl Method for Sensitivity {
    const ID: &'static str = "analysis.sensitivity";
    const TITLE: &'static str = "Sensitivity table";
    const DESCRIPTION: &'static str =
        "One output of any method across a one-way or two-way table of input values, with the \
        base case; a combination the method refuses is recorded in its cell.";
    const VERSION: &'static str = "1";
    type Input = SensitivityInput;
    type Output = SensitivityOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        sensitivity(input)
    }

    fn evaluated_method(input: &Self::Input) -> Option<&str> {
        Some(&input.method)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    fn run(id: &str, input: &Value) -> crate::Result<Value> {
        crate::run(id, &input.to_string())
    }

    fn dcf() -> Value {
        json!({"stages": [{"periods": 5, "projection": "given", "flows": [100, 110, 120, 130, 140]}],
               "cash_flow_terms": "nominal", "discount_rate_terms": "nominal", "wacc": "0.09",
               "timing": "end_of_period",
               "terminal": {"tv.gordon": {"growth": "0.02", "first_flow": "last_flow_grown"}},
               "net_financial_position": {"gross_debt": 300, "cash": 0}})
    }

    fn table() -> Value {
        json!({
            "method": "dcf.fcff", "base_input": dcf(), "output": "/value_of_operations",
            "rows": {"variable": "/wacc", "values": ["0.07", "0.09", "0.11"]},
            "columns": {"variable": "/terminal/tv.gordon/growth", "values": ["0.01", "0.08"]}
        })
    }

    /// AC8: every cell equals a direct run of the inner method on the overridden input, and a
    /// one-way table equals the matching column of the two-way table.
    #[test]
    fn cells_equal_direct_runs_and_one_way_equals_a_column() {
        let out = run("analysis.sensitivity", &table()).unwrap()["value"].clone();
        for (i, r) in ["0.07", "0.09", "0.11"].iter().enumerate() {
            for (j, g) in ["0.01", "0.08"].iter().enumerate() {
                let mut input = dcf();
                input["wacc"] = json!(r);
                input["terminal"]["tv.gordon"]["growth"] = json!(g);
                match crate::run("dcf.fcff", &input.to_string()) {
                    Ok(direct) => assert_eq!(
                        out["cells"][i][j]["value"],
                        direct["value"]["value_of_operations"]
                    ),
                    Err(e) => assert_eq!(out["cells"][i][j]["error"]["code"], e.code()),
                }
            }
        }
        let mut one_way = table();
        one_way.as_object_mut().unwrap().remove("columns");
        one_way["base_input"]["terminal"]["tv.gordon"]["growth"] = json!("0.01");
        let col = run("analysis.sensitivity", &one_way).unwrap()["value"].clone();
        for i in 0..3 {
            assert_eq!(col["cells"][i][0], out["cells"][i][0]);
        }
        // Only (0.07, 0.08) has growth not below the rate.
        assert_eq!(out["failed_cells"], json!(1));
    }

    #[test]
    fn invalid_tables_are_refused_before_any_evaluation() {
        let mut same = table();
        same["columns"]["variable"] = json!("/wacc");
        assert_eq!(
            run("analysis.sensitivity", &same).unwrap_err().code(),
            "FM-E001"
        );
        let mut empty = table();
        empty["rows"]["values"] = json!([]);
        assert_eq!(
            run("analysis.sensitivity", &empty).unwrap_err().code(),
            "FM-E001"
        );
        // AC13: 101 × 100 cells plus the base case exceed the limit; the base input is invalid
        // for dcf.fcff, so any evaluation would return FM-E001 instead of FM-E016.
        let mut big = table();
        big["base_input"]["unknown"] = json!(1);
        big["rows"]["values"] = json!(vec!["0.09"; 101]);
        big["columns"]["values"] = json!(vec!["0.01"; 100]);
        assert_eq!(
            run("analysis.sensitivity", &big).unwrap_err().code(),
            "FM-E016"
        );
        let mut nested = table();
        nested["method"] = json!("analysis.tornado");
        assert_eq!(
            run("analysis.sensitivity", &nested).unwrap_err().code(),
            "FM-E017"
        );
        let mut pointer = table();
        pointer["columns"]["variable"] = json!("/timing");
        assert_eq!(
            run("analysis.sensitivity", &pointer).unwrap_err().code(),
            "FM-E015"
        );
    }

    /// The cost bound: evaluations × input values. A table well under 10,000 evaluations on a
    /// long input is refused before the first evaluation.
    #[test]
    fn work_is_bounded_by_evaluations_times_input_size() {
        let mut t = table();
        t["base_input"]["stages"][0]["flows"] = json!(vec![100; 300]);
        t["base_input"]["unknown"] = json!(1); // any evaluation would return FM-E001
        t["rows"]["values"] = json!(vec!["0.09"; 60]);
        t["columns"]["values"] = json!(vec!["0.01"; 60]);
        let err = run("analysis.sensitivity", &t).unwrap_err();
        assert_eq!(err.code(), "FM-E016", "{err}");
        assert!(err.to_string().contains("units of work"), "{err}");
    }

    /// R9: the base case runs first and its error is returned unchanged.
    #[test]
    fn a_failing_base_case_returns_its_error() {
        let mut t = table();
        t["base_input"]["terminal"]["tv.gordon"]["growth"] = json!("0.2");
        assert_eq!(
            run("analysis.sensitivity", &t).unwrap_err().code(),
            "FM-E004"
        );
    }

    /// AC14: reruns are byte-identical; key order in base_input changes neither value nor hash.
    #[test]
    fn deterministic_and_independent_of_key_order() {
        let a = run("analysis.sensitivity", &table()).unwrap();
        assert_eq!(
            a.to_string(),
            run("analysis.sensitivity", &table()).unwrap().to_string()
        );
        let mut b = table();
        b["base_input"] = json!({"net_financial_position": {"cash": 0, "gross_debt": 300},
                                 "terminal": {"tv.gordon": {"first_flow": "last_flow_grown",
                                                            "growth": "0.02"}},
                                 "timing": "end_of_period", "wacc": "0.09",
                                 "discount_rate_terms": "nominal", "cash_flow_terms": "nominal",
                                 "stages": [{"flows": [100, 110, 120, 130, 140],
                                             "projection": "given", "periods": 5}]});
        let b = run("analysis.sensitivity", &b).unwrap();
        assert_eq!(a["value"].to_string(), b["value"].to_string());
        assert_eq!(a["meta"]["input_hash"], b["meta"]["input_hash"]);
    }

    /// AC15: a base value given as a JSON number or as a string gives the same table.
    #[test]
    fn number_spelling_does_not_change_the_values() {
        let a = run("analysis.sensitivity", &table()).unwrap();
        let mut numeric = table();
        numeric["base_input"]["wacc"] = serde_json::from_str("0.09").unwrap();
        let b = run("analysis.sensitivity", &numeric).unwrap();
        assert_eq!(a["value"].to_string(), b["value"].to_string());
    }
}
