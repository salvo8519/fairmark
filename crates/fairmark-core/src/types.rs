//! Shared domain types.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// When cash flows arrive within each period.
///
/// Rates are always decimal fractions per period: `0.10` means 10%.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Timing {
    /// Cash flows at the end of each period (t = 1, 2, ..., n).
    #[default]
    EndOfPeriod,
    /// Cash flows spread evenly, discounted at mid period (t = 0.5, 1.5, ...).
    MidPeriod,
}

impl Timing {
    pub fn label(self) -> &'static str {
        match self {
            Self::EndOfPeriod => "end of period",
            Self::MidPeriod => "mid period",
        }
    }
}
