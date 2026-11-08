# ADR 0013 — Input errors name the field by its JSON pointer, through `serde_path_to_error`

Status: proposed · 2026-11-08 (issue #174)

## Context
SPEC-0001 R27 requires every error to name the offending field by its JSON pointer and to
state the fix. Validation errors raised by the methods already do. Errors raised while the
registry reads the input did not: `serde_json` reports a missing field, an unknown field, a
wrong type or an unknown enumeration value with the line and column where the reader
stopped, for example "missing field `first_flow` at line 1 column 215". Fairmark is used
mainly by AI agents, which send the input as one line of JSON: a column number does not tell
them which field to correct, and inside an array it does not tell them which element.

`serde` does not track the path of the value being read. Three ways to obtain it were
considered:

1. **`serde_path_to_error`**, a crate that wraps any `serde` deserialiser and records the
   path (object keys, array indexes, enumeration variants) of the value that failed;
2. **our own wrapper** with the same design: a deserialiser, visitor, sequence, map and
   enumeration access that each push and pop one segment, about a thousand lines that would
   repeat the crate;
3. **recovering the path from the line and column**, by scanning the input text up to the
   position of the error. It depends on where `serde_json` stops reading for each kind of
   error, which is not part of its contract, and it fails for values read in two passes.

## Decision
1. **The registry reads every input through `serde_path_to_error`.** It is the only place
   that deserialises method inputs (`registry::parse`), so every method, every adapter and
   every analysis that evaluates another method get the pointer with no change to the
   methods.
2. **Messages.** A field that cannot be read is reported as ``invalid input at `<pointer>`:
   <detail>`` (`FM-E001`) or ``invalid date at `<pointer>`: <detail>`` (`FM-E009`). The pointer
   follows RFC 6901 and names the field itself: for a missing or unknown field, the field
   appended to the object that holds it. The detail keeps what `serde` says the input should
   be ("expected one of `given`, `constant_growth`, `linear_decline`") and adds the fix for
   a missing field ("add it") and an unknown field ("remove or rename it"). Codes never
   change. A malformed JSON text (a syntax error, a truncated input, trailing characters)
   has no field and keeps the line and column.
3. **Two variants, existing codes.** `FairmarkError::InvalidInputAt` and `InvalidDateAt`
   carry the pointer and report `FM-E001` and `FM-E009`, as `DiscountRateOutOfRange` reports
   `FM-E003` beside `RateOutOfRange`.
4. **Values read in one pass.** A type that buffered its input in a `serde_json::Value`
   before reading it (`tv::compare::TechniqueEntry`) loses the path below it; it is read
   from the input directly instead.

### The dependency
`serde_path_to_error` 0.1.20 (September 2025) by the maintainer of `serde`; licence
`MIT OR Apache-2.0`, on the allow-list of `deny.toml` and of the *Security* workflow; minimum
Rust 1.61, below the workspace minimum of 1.88; about 360 million downloads; it depends only
on `serde_core` and `itoa`, both already in the build, so `Cargo.lock` gains one crate. It
performs no I/O and has no network, file-system or runtime dependency; `cargo deny` passes.
Its last release is fourteen months old: the crate has a fixed scope and follows `serde`
(its 0.1.18 to 0.1.20 releases adopted `serde_core`). The policy's rule of a release in the
last twelve months applies to adapters and tooling; this decision records the exception for
the core, and the weekly maintenance run and `cargo deny check advisories` keep watching it.

## Consequences
+ Agents correct an input from the message alone: the pointer, the values expected and the
  fix, without reading the schema.
+ One place, no change to the methods; a new method gets pointers in its input errors with
  no code of its own.
+ The input hash is unchanged: the input is re-serialised from the parsed value, as before.
− One more runtime dependency of the core, small and maintained by the `serde` maintainer.
− Two rewrites depend on texts of `serde` and `rust_decimal` that are not a stable contract:
  "missing field `", "unknown field `" and "expected field with custom name" (an object where
  a decimal is expected). `crates/fairmark-core/tests/input_errors.rs` pins each one, so a
  change upstream fails the tests rather than the messages.
− Errors of a method evaluated by an analysis (`base_input`) carry pointers relative to
  `base_input`, as the analysis pointers (`variable`, `output`) already are.
