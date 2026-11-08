//! Discounted cash flow (SPEC-0001): one shared engine and one method per basis.
//!
//! - [`engine`]: stages and projection, discount rates, terms, timing, terminal value; shared
//!   by every DCF method and called through typed Rust calls (ADR 0009).
//! - [`fcff`]: `dcf.fcff`, the enterprise DCF (FCFF at the WACC) with its bridge to equity
//!   value.

pub mod engine;
pub mod fcff;

pub use engine::{
    FirstTerminalFlow, GordonTerminal, PeriodRow, Projection, Stage, TechniqueOutput, Terminal,
    TerminalDiscountTime, TerminalOutput, Terms,
};
pub use fcff::{fcff, DcfFcffInput, DcfFcffOutput, Fcff, FcffBridge};
