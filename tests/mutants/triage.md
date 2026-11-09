# Mutation testing triage list (issue #282)

Missed mutants found by the first baseline. Each row ends as **Killed** (a new test, named in
the pull request that adds it) or **Equivalent** (an entry in `equivalent.toml` reviewed by a
maintainer); rules in `docs/en/project/mutation-testing.md`. Nothing here is excluded: the
weekly run keeps reporting these mutants until they are resolved. Remove a row in the pull
request that resolves it.

## Baseline

cargo-mutants 27.1.0 with the configuration of this branch, `just mutants <module>`, 2026-11-09,
source of the core at commit 10a1ca4 (`origin/main`),
Linux with 2 cores: 456 mutants in 83 minutes, about 11 seconds per mutant. The other modules
of the core have not run yet: the first weekly run of the *Mutation testing* workflow gives
the baseline of the whole core (2,037 mutants).

| Module | Mutants | Caught | Missed | Equivalent | Unviable | Timeout | Failure |
| --- | --- | --- | --- | --- | --- | --- | --- |
| dcf/engine | 153 | 122 | 18 | 0 | 13 | 0 | 0 |
| rates | 34 | 21 | 8 | 0 | 5 | 0 | 0 |
| statements/balance_sheet | 82 | 52 | 8 | 0 | 22 | 0 | 0 |
| tvm | 187 | 124 | 39 | 0 | 24 | 0 | 0 |
| Total | 456 | 319 | 73 | 0 | 64 | 0 | 0 |

Caught over viable mutants (caught + missed): 319 / 392 = 81.4 per cent.

## Provisional categories

The category is a first reading of the mutation, to order the work; it is not a verdict and
does not make a mutant equivalent. Each row is confirmed or reclassified when it is triaged.

- **A. Explain text not asserted.** A formula string, label or explain step can be emptied or
  replaced without a failing test. The audit trail is part of the result (constitution,
  principle II), so the expected outcome is a test on the explain content.
- **B. Solver internals.** Newton steps and derivatives: the bracketed fallback still reaches
  the same root, so only the path changes. The solver reports its method and iterations
  (constitution, principle I); a test on those reported values would catch most of them.
- **C. Boundary comparisons.** `<` against `<=`, `>` against `>=`: either a missing test at the
  boundary value or an equivalent mutant (both branches give the same result there).
- **D. Branches and validation.** Guards, match arms and boolean operators in input checks,
  defaults and statement building: most likely missing tests for a combination of inputs.

## Missed mutants

| Category | Location | Function | Mutation | Status |
| --- | --- | --- | --- | --- |
| A | `dcf/engine.rs:76` | `Projection::label` | `replace Projection::label -> &'static str with ""` | Open |
| A | `dcf/engine.rs:76` | `Projection::label` | `replace Projection::label -> &'static str with "xyzzy"` | Open |
| A | `rates.rs:253` | `growth_formula` | `replace growth_formula -> String with "xyzzy".into()` | Open |
| A | `rates.rs:253` | `growth_formula` | `replace growth_formula -> String with String::new()` | Open |
| A | `rates.rs:261` | `inverse_formula` | `replace inverse_formula -> String with "xyzzy".into()` | Open |
| A | `rates.rs:261` | `inverse_formula` | `replace inverse_formula -> String with String::new()` | Open |
| A | `statements/balance_sheet.rs:1887` | `Outputs::note` | `delete match arm "current_non_current" in Outputs::note` | Open |
| A | `tvm.rs:846` | `discount_formula` | `replace discount_formula -> String with "xyzzy".into()` | Open |
| A | `tvm.rs:846` | `discount_formula` | `replace discount_formula -> String with String::new()` | Open |
| A | `tvm.rs:1420` | `level_factor_steps` | `replace level_factor_steps -> Vec<Step> with vec![]` | Open |
| A | `tvm.rs:1424` | `level_factor_steps` | `delete match arm "Annuity factor" in level_factor_steps` | Open |
| A | `tvm.rs:1428` | `level_factor_steps` | `delete match arm "Annuity factor (r close to g, direct sum)" in level_factor_steps` | Open |
| A | `tvm.rs:1432` | `level_factor_steps` | `delete match arm "Annuity factor (r = g)" in level_factor_steps` | Open |
| B | `rates.rs:171` | `discount_factor_derivative` | `replace discount_factor_derivative -> Result<Decimal> with Ok(Default::default())` | Open |
| B | `rates.rs:174` | `discount_factor_derivative` | `delete - in discount_factor_derivative` | Open |
| B | `rates.rs:175` | `discount_factor_derivative` | `delete - in discount_factor_derivative` | Open |
| B | `tvm.rs:61` | `npv_derivative` | `replace npv_derivative -> Result<Decimal> with Ok(Default::default())` | Open |
| B | `tvm.rs:64` | `npv_derivative` | `replace + with * in npv_derivative` | Open |
| B | `tvm.rs:64` | `npv_derivative` | `replace + with - in npv_derivative` | Open |
| B | `tvm.rs:65` | `npv_derivative` | `delete - in npv_derivative` | Open |
| B | `tvm.rs:290` | `require_sign_change` | `replace > with >= in require_sign_change` | Open |
| B | `tvm.rs:291` | `require_sign_change` | `replace < with <= in require_sign_change` | Open |
| B | `tvm.rs:424` | `solve_rate` | `replace > with >= in solve_rate` | Open |
| B | `tvm.rs:464` | `solve_rate` | `delete match arm [] in solve_rate` | Open |
| B | `tvm.rs:488` | `newton` | `replace newton -> Option<(Decimal, u32)> with None` | Open |
| B | `tvm.rs:496` | `newton` | `delete - in newton` | Open |
| B | `tvm.rs:496` | `newton` | `replace <= with > in newton` | Open |
| B | `tvm.rs:501` | `newton` | `replace < with <= in newton` | Open |
| B | `tvm.rs:501` | `newton` | `replace < with == in newton` | Open |
| B | `tvm.rs:501` | `newton` | `replace < with > in newton` | Open |
| B | `tvm.rs:501` | `newton` | `replace > with < in newton` | Open |
| B | `tvm.rs:501` | `newton` | `replace > with == in newton` | Open |
| B | `tvm.rs:501` | `newton` | `replace > with >= in newton` | Open |
| B | `tvm.rs:501` | `newton` | `replace match guard next < lo || next > hi with false in newton` | Open |
| B | `tvm.rs:501` | `newton` | `replace match guard next < lo || next > hi with true in newton` | Open |
| B | `tvm.rs:501` | `newton` | `replace || with && in newton` | Open |
| B | `tvm.rs:811` | `dated_npv_derivative` | `replace dated_npv_derivative -> Result<Decimal> with Ok(Default::default())` | Open |
| C | `dcf/engine.rs:695` | `check_stages` | `replace > with >= in check_stages` | Open |
| C | `dcf/engine.rs:862` | `check_stages` | `replace > with >= in check_stages` | Open |
| C | `dcf/engine.rs:1324` | `restate_growth` | `replace < with <= in restate_growth` | Open |
| C | `statements/balance_sheet.rs:191` | `split_maturities` | `replace > with >= in split_maturities` | Open |
| C | `tvm.rs:1246` | `factor_with` | `replace < with <= in factor_with` | Open |
| C | `tvm.rs:1274` | `factor_with` | `replace < with <= in factor_with` | Open |
| C | `tvm.rs:1483` | `future_value` | `replace > with >= in future_value` | Open |
| D | `dcf/engine.rs:509` | `apply_defaults` | `replace == with != in apply_defaults` | Open |
| D | `dcf/engine.rs:713` | `check_stages` | `replace && with || in check_stages` | Open |
| D | `dcf/engine.rs:733` | `check_stages` | `delete match arm (None, None) in check_stages` | Open |
| D | `dcf/engine.rs:752` | `check_stages` | `replace match guard s.lines.is_some() with true in check_stages` | Open |
| D | `dcf/engine.rs:756` | `check_stages` | `delete match arm Projection::Given in check_stages` | Open |
| D | `dcf/engine.rs:1000` | `resolve_inflation` | `replace match guard !uses_terminal_rate with false in resolve_inflation` | Open |
| D | `dcf/engine.rs:1022` | `resolve_inflation` | `replace match guard uses_terminal_rate with true in resolve_inflation` | Open |
| D | `dcf/engine.rs:1125` | `resolve_terms` | `replace match guard i.single with true in resolve_terms` | Open |
| D | `dcf/engine.rs:1397` | `project` | `replace match guard s.built_up.is_some() with true in project` | Open |
| D | `dcf/engine.rs:1448` | `project` | `replace && with || in project` | Open |
| D | `dcf/engine.rs:1497` | `build_up_stages` | `replace || with && in build_up_stages` | Open |
| D | `dcf/engine.rs:1670` | `run` | `replace + with * in run` | Open |
| D | `dcf/engine.rs:1671` | `run` | `replace - with / in run` | Open |
| D | `rates.rs:137` | `inverse_pow` | `replace match guard !g.is_zero() with true in inverse_pow` | Open |
| D | `statements/balance_sheet.rs:529` | `statutory` | `delete ! in statutory` | Open |
| D | `statements/balance_sheet.rs:530` | `statutory` | `delete ! in statutory` | Open |
| D | `statements/balance_sheet.rs:572` | `statutory` | `delete ! in statutory` | Open |
| D | `statements/balance_sheet.rs:869` | `statutory` | `replace && with || in statutory` | Open |
| D | `statements/balance_sheet.rs:940` | `statutory` | `delete ! in statutory` | Open |
| D | `statements/balance_sheet.rs:1358` | `statutory` | `replace || with && in statutory` | Open |
| D | `tvm.rs:623` | `present_value` | `delete match arm (None, Some(curve)) in present_value` | Open |
| D | `tvm.rs:658` | `present_value` | `replace - with / in present_value` | Open |
| D | `tvm.rs:1274` | `factor_with` | `replace && with || in factor_with` | Open |
| D | `tvm.rs:1274` | `factor_with` | `replace < with == in factor_with` | Open |
| D | `tvm.rs:1274` | `factor_with` | `replace < with > in factor_with` | Open |
| D | `tvm.rs:1274` | `factor_with` | `replace match guard underflow_to_zero && ratio < one with false in factor_with` | Open |
| D | `tvm.rs:1274` | `factor_with` | `replace match guard underflow_to_zero && ratio < one with true in factor_with` | Open |
| D | `tvm.rs:1483` | `future_value` | `replace > with == in future_value` | Open |
| D | `tvm.rs:1483` | `future_value` | `replace || with && in future_value` | Open |
