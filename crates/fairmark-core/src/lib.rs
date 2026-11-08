//! # fairmark-core
//!
//! Deterministic valuation engine. Every number is computed in exact decimal
//! arithmetic, every result carries its audit trail (`explain`) and a
//! reproducibility fingerprint (`meta.input_hash`).
//!
//! The core performs no I/O and opens no network connections.
//!
//! ```
//! let out = fairmark_core::run(
//!     "dcf.fcff",
//!     r#"{"free_cash_flows": [100, 110, 120], "discount_rate": 0.10,
//!         "terminal_growth": 0.02, "net_debt": 200}"#,
//! ).unwrap();
//! // Decimals are serialized as strings so that no consumer loses precision.
//! assert_eq!(out["value"]["terminal_value"], "1530");
//! ```

#![forbid(unsafe_code)]

pub mod analysis;
pub mod coc;
pub mod dates;
pub mod dcf;
pub mod error;
pub mod explain;
pub mod loans;
pub mod method;
mod num;
pub mod rates;
pub mod ratios;
pub mod registry;
pub mod solver;
pub mod tv;
pub mod tvm;
pub mod types;

pub use error::{FairmarkError, Result};
pub use method::Method;
pub use registry::{input_schema, method, run, MethodInfo, METHODS};

/// Version of the engine, stamped into every result.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
