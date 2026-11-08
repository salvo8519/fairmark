//! Discounted cash flow (SPEC-0001): one shared engine and one method per basis.
//!
//! - [`engine`]: stages and projection, discount rates, terms, timing, terminal value; shared
//!   by every DCF method and called through typed Rust calls (ADR 0009).
//! - [`fcff`]: `dcf.fcff`, the enterprise DCF (FCFF at the WACC) with its bridge to equity
//!   value.
//! - [`free_cash_flows`]: `dcf.free_cash_flows`, the build-up of FCFF and FCFE from line items.

pub mod engine;
pub mod fcff;
pub mod free_cash_flows;

pub use engine::{
    FirstTerminalFlow, GordonTerminal, PeriodRow, Projection, Stage, TechniqueOutput, Terminal,
    TerminalDiscountTime, TerminalOutput, Terms,
};
pub use fcff::{fcff, DcfFcffInput, DcfFcffOutput, Fcff, FcffBridge};
pub use free_cash_flows::{
    free_cash_flows, FreeCashFlows, FreeCashFlowsInput, FreeCashFlowsOutput,
};
