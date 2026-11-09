//! Thin PyO3 bridge. Data crosses the boundary as JSON strings, so decimals
//! stay exact end to end; all ergonomics live in the pure-Python layer.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

pyo3::create_exception!(_engine, EngineError, PyValueError);

/// Run a method on a JSON input. Returns the JSON result envelope.
/// Raises `EngineError` whose message is a JSON `{"code", "message"}` report.
#[pyfunction]
fn run(method: &str, input_json: &str) -> PyResult<String> {
    fairmark_core::run(method, input_json)
        .map(|v| v.to_string())
        .map_err(|e| {
            EngineError::new_err(serde_json::to_string(&e.to_report()).unwrap_or_default())
        })
}

/// JSON list of available methods.
#[pyfunction]
fn methods() -> String {
    serde_json::to_string(fairmark_core::METHODS).unwrap_or_default()
}

/// JSON Schema of a method's input.
#[pyfunction]
fn input_schema(method: &str) -> PyResult<String> {
    fairmark_core::input_schema(method)
        .map(|v| v.to_string())
        .map_err(|e| {
            EngineError::new_err(serde_json::to_string(&e.to_report()).unwrap_or_default())
        })
}

/// JSON Schema of a method's output (the `value` of the result envelope).
#[pyfunction]
fn output_schema(method: &str) -> PyResult<String> {
    fairmark_core::output_schema(method)
        .map(|v| v.to_string())
        .map_err(|e| {
            EngineError::new_err(serde_json::to_string(&e.to_report()).unwrap_or_default())
        })
}

/// JSON Schema of a method's whole result envelope.
#[pyfunction]
fn result_schema(method: &str) -> PyResult<String> {
    fairmark_core::result_schema(method)
        .map(|v| v.to_string())
        .map_err(|e| {
            EngineError::new_err(serde_json::to_string(&e.to_report()).unwrap_or_default())
        })
}

#[pymodule]
fn _engine(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("ENGINE_VERSION", fairmark_core::ENGINE_VERSION)?;
    m.add("EngineError", m.py().get_type::<EngineError>())?;
    m.add_function(wrap_pyfunction!(run, m)?)?;
    m.add_function(wrap_pyfunction!(methods, m)?)?;
    m.add_function(wrap_pyfunction!(input_schema, m)?)?;
    m.add_function(wrap_pyfunction!(output_schema, m)?)?;
    m.add_function(wrap_pyfunction!(result_schema, m)?)?;
    Ok(())
}
