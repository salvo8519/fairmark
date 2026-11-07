//! Checked decimal arithmetic. Every operation on user-supplied values goes through these
//! helpers, so extreme inputs return `FM-E008` instead of panicking (constitution III).

use rust_decimal::Decimal;

use crate::error::{FairmarkError, Result};

pub(crate) fn add(a: Decimal, b: Decimal) -> Result<Decimal> {
    a.checked_add(b).ok_or(FairmarkError::Overflow("addition"))
}

pub(crate) fn sub(a: Decimal, b: Decimal) -> Result<Decimal> {
    a.checked_sub(b)
        .ok_or(FairmarkError::Overflow("subtraction"))
}

pub(crate) fn mul(a: Decimal, b: Decimal) -> Result<Decimal> {
    a.checked_mul(b)
        .ok_or(FairmarkError::Overflow("multiplication"))
}

pub(crate) fn div(a: Decimal, b: Decimal) -> Result<Decimal> {
    a.checked_div(b).ok_or(FairmarkError::Overflow("division"))
}

/// `1 + r`.
pub(crate) fn one_plus(r: Decimal) -> Result<Decimal> {
    add(Decimal::ONE, r)
}
