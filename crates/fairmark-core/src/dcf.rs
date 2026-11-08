//! Discounted cash flow (SPEC-0001): one shared engine and one method per basis.
//!
//! - [`engine`]: stages and projection, discount rates, terms, timing, terminal value; shared
//!   by every DCF method and called through typed Rust calls (ADR 0009).
//! - [`bridge`]: the lines from the discounted value to equity value (non-operating assets,
//!   debt-like items, non-controlling interests), shared by both bases.
//! - [`terminal`]: the terminal technique (`tv.*`), its checks and its discounting; called by
//!   the engine.
//! - [`fcff`]: `dcf.fcff`, the enterprise DCF (FCFF at the WACC) with its bridge to equity
//!   value.
//! - [`fcfe`]: `dcf.fcfe`, the equity DCF (FCFE at the cost of equity), equity value directly.
//! - [`free_cash_flows`]: `dcf.free_cash_flows`, the build-up of FCFF and FCFE from line items.

pub mod bridge;
pub mod engine;
pub mod fcfe;
pub mod fcff;
pub mod free_cash_flows;
pub mod terminal;

pub use bridge::{BridgeClaim, MeasurementBasis};
pub use engine::{
    DiscountedValue, PeriodRow, Projection, RateConversion, Stage, TerminalDiscountTime, Terms,
    TermsOutput,
};
pub use fcfe::{fcfe, DcfFcfeInput, DcfFcfeOutput, Fcfe, FcfeBridge};
pub use fcff::{fcff, DcfFcffInput, DcfFcffOutput, Fcff, FcffBridge};
pub use free_cash_flows::{
    free_cash_flows, FreeCashFlows, FreeCashFlowsInput, FreeCashFlowsOutput,
};
pub use terminal::{
    ExitMultipleTerminal, FirstTerminalFlow, GordonTerminal, HModelFirstFlow, HModelTerminal,
    LimitedHorizonTerminal, LiquidationTerminal, TechniqueOutput, Terminal, TerminalOutput,
    ValueDriverTerminal,
};
