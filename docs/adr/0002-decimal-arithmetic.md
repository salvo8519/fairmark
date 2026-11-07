# ADR 0002 — Decimal arithmetic everywhere

Status: accepted · 2026-11-07

## Context
Valuations are reviewed by humans against spreadsheets and must be reproducible across
platforms. Binary floating point introduces representation error (0.1 + 0.2 ≠ 0.3) and
platform-dependent results for transcendental functions.

## Decision
Use `rust_decimal` (96-bit mantissa) for every calculation, parse JSON numbers exactly
(`arbitrary_precision`) and serialize decimals as strings.

## Consequences
+ Bit-for-bit reproducibility; values match hand calculations.
− Slower than `f64` (irrelevant at valuation scale) and limited to ~28 significant digits.
− Fractional powers rely on decimal `powd`; verified by golden tests against a 50-digit reference.
