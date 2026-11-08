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

`serde` does not track the path of the value being read. These ways to obtain it were
considered:

1. **`serde_path_to_error`**, a crate that wraps any `serde` deserialiser and records the
   path (object keys, array indexes, enumeration variants) of the value that failed.
   Chosen.
2. **Our own wrapper** with the same design: a deserialiser, visitor, sequence, map and
   enumeration access that each push and pop one segment. Rejected: about a thousand lines
   that would repeat the crate, with no gain in control.
3. **Vendoring the crate** (copying its source into the repository). Rejected while the crate
   is maintained: we would carry its maintenance and lose its fixes and advisories. It is the
   fallback, see below.
4. **JSON Schema validation before parsing**, with the `jsonschema` crate against the schema
   the registry already generates. Rejected: a large dependency tree, features that resolve
   remote `$ref` (network access the core must not have), and a second validator that
   duplicates the registry and could disagree with `serde` on what is accepted.
5. **Recovering the path from the line and column**, by scanning the input text up to the
   position of the error. Rejected: it depends on where `serde_json` stops reading for each
   kind of error, which is not part of its contract, and it fails for values read in two
   passes.

## Decision
1. **The registry reads every input through `serde_path_to_error`.** It is the only place
   that deserialises method inputs (`registry::parse`), so every method, every adapter and
   every analysis that evaluates another method get the pointer with no change to the
   methods.
2. **Messages.** A field that cannot be read is reported as ``invalid input at `<pointer>`:
   <detail>`` (`FM-E001`) or ``invalid date at `<pointer>`: <detail>`` (`FM-E009`). The pointer
   follows RFC 6901 and names the field itself: for a missing, repeated or unknown field, the
   field appended to the object that holds it. The detail keeps what `serde` says the input
   should be ("expected one of `given`, `constant_growth`, `linear_decline`") and adds the
   fix: "add it" for a missing field, "keep one" for a repeated field, "remove or rename it"
   for an unknown field. Rust type names are not shown ("expected an object"), and every
   error about a decimal uses one wording. A malformed JSON text (a syntax error, a truncated
   input, trailing characters) has no field and keeps the line and column.
3. **Two variants, existing codes.** `FairmarkError::InvalidInputAt` and `InvalidDateAt`
   carry the pointer and report `FM-E001` and `FM-E009`, as `DiscountRateOutOfRange` reports
   `FM-E003` beside `RateOutOfRange`. A variant that only adds the location of an existing
   condition keeps that condition's code: one condition, one code (SPEC-0001 Decision 19).
4. **Values read in two passes keep their pointer.** A type that collects its input before
   reading it (`tv::compare::TechniqueEntry`, which must see every key of an entry first)
   reads the collected value through `serde_path_to_error` too and hands the relative
   pointer to the registry (`registry::nested_error`), which appends it to the path. What the
   type accepts does not change: `tv.compare` version 1 still accepts a key repeated within
   an entry, with its last value.
5. **Unknown technique ids** are named at their key in both places that accept techniques:
   `/terminal/tv.gordo` in `dcf.fcff` and `/techniques/0/tv.gordo` in `tv.compare`. The detail
   differs because the inputs differ: `dcf.fcff` declares each technique as an optional field
   ("unknown field"), `tv.compare` as an enumeration ("unknown variant"); both list the valid
   ids.

### The dependency
The core follows the dependency policy for the core (`docs/en/project/dependencies.md`):

- maintained by David Tolnay, the maintainer of `serde`, which the core already depends on;
- licence `MIT OR Apache-2.0`, on the allow-list of `deny.toml` and of the *Security*
  workflow; `cargo deny` passes;
- no `unsafe` code and no build script; no I/O, network, file-system or runtime dependency;
- small and fixed in scope: about 1,700 lines for deserialisation (`de.rs`, `path.rs`,
  `wrap.rs`); minimum Rust 1.61, below the workspace minimum of 1.88; about 360 million
  downloads;
- it depends only on `serde_core` and `itoa`, both already in the build, so `Cargo.lock`
  gains one crate (0.1.20).

Its last release (0.1.20, September 2025) is fourteen months old. This is acceptable because
the crate does one thing that `serde`'s data model fixes, and it moves when `serde` moves: its
releases 0.1.18 to 0.1.20 followed the `serde_core` split. The weekly maintenance run and
`cargo deny check advisories` keep watching it, and an upgrade of it is always reviewed.

**Fallback.** If the crate is abandoned in a way that blocks a `serde` upgrade, or is
compromised, the three files the core uses (`src/de.rs`, `src/path.rs`, `src/wrap.rs`,
`MIT OR Apache-2.0`) are vendored into `fairmark-core` with their licence notice, through a
new ADR.

## Consequences
+ Agents correct an input from the message alone: the pointer, the values expected and the
  fix, without reading the schema.
+ One place, no change to the methods; a new method gets pointers in its input errors with
  no code of its own.
+ The input hash is unchanged: the input is re-serialised from the parsed value, as before.
− One more runtime dependency of the core, small and maintained by the `serde` maintainer.
− Several rewrites depend on texts of `serde` and `rust_decimal` that are not a stable
  contract: "missing field `", "duplicate field `", "unknown field `", "expected struct ",
  "a Decimal type representing a fixed-point number" and "expected field with custom name"
  (an object where a decimal is expected). `crates/fairmark-core/tests/input_errors.rs` pins
  each one, so a change upstream fails the tests rather than the messages.
− Errors of a method evaluated by an analysis (`base_input`) carry pointers relative to
  `base_input`, as the analysis pointers (`variable`, `output`) already are.
