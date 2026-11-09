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

## Rerun after the triage

cargo-mutants 27.1.0, `--file <module> --re <functions>` restricted to the functions of the
missed mutants (every mutant of those functions, caught ones included), 2026-11-09, this
branch, Linux with 2 cores: 256 mutants in 61 minutes.

| Module | Mutants | Caught | Missed | Equivalent | Unviable | Timeout | Failure |
| --- | --- | --- | --- | --- | --- | --- | --- |
| dcf/engine | 111 | 103 | 0 | 1 | 7 | 0 | 0 |
| rates | 12 | 11 | 1 | 0 | 0 | 0 | 0 |
| statements/balance_sheet | 37 | 36 | 0 | 0 | 1 | 0 | 0 |
| tvm | 96 | 87 | 3 | 1 | 5 | 0 | 0 |
| Total | 256 | 237 | 4 | 2 | 13 | 0 | 0 |

The four missed mutants are the open rows below.

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

Still open: each is blocked by a defect found during the triage (below), which the maintainer
decides on first.

| Category | Location | Function | Mutation | Status |
| --- | --- | --- | --- | --- |
| C | `tvm.rs:1274` | `factor_with` | `replace < with <= in factor_with` | Open: blocked by defect 1 below |
| D | `rates.rs:137` | `inverse_pow` | `replace match guard !g.is_zero() with true in inverse_pow` | Open: blocked by defect 2 below |
| D | `tvm.rs:1274` | `factor_with` | `replace < with == in factor_with` | Open: blocked by defect 1 below |
| D | `tvm.rs:1274` | `factor_with` | `replace match guard underflow_to_zero && ratio < one with false in factor_with` | Open: blocked by defect 1 below |

### Defects found by the triage

1. **The documented underflow step of the annuity factor is never written**
   (`tvm.rs` `factor_with`, line 1274). `tv-limited-horizon.md` states that when
   ((1 + g)/(1 + r))^n falls below the decimal precision "it is taken as 0, its limit, and an
   explain step says so". The integer power of `rust_decimal` returns 0 for a base below 1
   instead of an overflow, so the branch that writes the step is never reached: the value is
   right (for example the factor 2 = 1 / 0.5 at 50% over 10,000 periods), the audit trail is not
   the documented one. The three open mutants of line 1274 only change that unreachable branch.
   Failing test, ignored until the maintainer decides:
   `mutants_tvm.rs::limited_horizon_reports_the_power_taken_as_zero`.
2. **`tvm::discount_factor` at a negative time returns 0** (`rates.rs` `inverse_pow`, line 137,
   public Rust API only). The function is documented as `1 / (1 + rate)^t` with no bound on t;
   at 50% and t = −163 the value is 1.5^163 ≈ 5.045 × 10^28, inside the decimal range, but the
   underflow fallback of `inverse_pow` assumes a non-negative exponent and returns 0. No method
   passes a negative time, so no method result is affected; for every non-negative exponent the
   open mutant is equivalent. Failing test, ignored until the maintainer decides (refuse a
   negative time or compute it):
   `mutants_tvm.rs::discount_factor_at_a_negative_time_is_the_compounding_factor`.

## Resolved

67 mutants killed by new behaviour tests in `crates/fairmark-core/tests/` (`mutants_tvm.rs`, `mutants_dcf_engine.rs`, `mutants_balance_sheet.rs`;
expected solver iterations and rates from `scripts/reference_solver.py`), 2 recorded as
equivalent in `equivalent.toml`, pending the maintainer's review. Each kill was confirmed by
rerunning cargo-mutants 27.1.0 on the functions concerned. These rows can be removed once the
pull request is merged.

| Category | Location | Function | Mutation | Status |
| --- | --- | --- | --- | --- |
| A | `dcf/engine.rs:76` | `Projection::label` | `replace Projection::label -> &'static str with ""` | Killed: `mutants_dcf_engine.rs::stage_errors_name_the_projection` |
| A | `dcf/engine.rs:76` | `Projection::label` | `replace Projection::label -> &'static str with "xyzzy"` | Killed: `mutants_dcf_engine.rs::stage_errors_name_the_projection` |
| A | `rates.rs:253` | `growth_formula` | `replace growth_formula -> String with "xyzzy".into()` | Killed: `mutants_tvm.rs::rates_convert_explain_writes_out_both_conversions` |
| A | `rates.rs:253` | `growth_formula` | `replace growth_formula -> String with String::new()` | Killed: `mutants_tvm.rs::rates_convert_explain_writes_out_both_conversions` |
| A | `rates.rs:261` | `inverse_formula` | `replace inverse_formula -> String with "xyzzy".into()` | Killed: `mutants_tvm.rs::rates_convert_explain_writes_out_both_conversions` |
| A | `rates.rs:261` | `inverse_formula` | `replace inverse_formula -> String with String::new()` | Killed: `mutants_tvm.rs::rates_convert_explain_writes_out_both_conversions` |
| A | `statements/balance_sheet.rs:1887` | `Outputs::note` | `delete match arm "current_non_current" in Outputs::note` | Killed: `mutants_balance_sheet.rs::lease_prepayment_without_a_source_is_noted_and_not_computed` |
| A | `tvm.rs:846` | `discount_formula` | `replace discount_formula -> String with "xyzzy".into()` | Killed: `mutants_tvm.rs::xnpv_explain_writes_out_each_discount_factor` |
| A | `tvm.rs:846` | `discount_formula` | `replace discount_formula -> String with String::new()` | Killed: `mutants_tvm.rs::xnpv_explain_writes_out_each_discount_factor` |
| A | `tvm.rs:1420` | `level_factor_steps` | `replace level_factor_steps -> Vec<Step> with vec![]` | Killed: `mutants_tvm.rs::future_value_reports_the_level_annuity_factor` |
| A | `tvm.rs:1424` | `level_factor_steps` | `delete match arm "Annuity factor" in level_factor_steps` | Killed: `mutants_tvm.rs::future_value_reports_the_level_annuity_factor` |
| A | `tvm.rs:1428` | `level_factor_steps` | `delete match arm "Annuity factor (r close to g, direct sum)" in level_factor_steps` | Killed: `mutants_tvm.rs::future_value_reports_the_level_annuity_factor` |
| A | `tvm.rs:1432` | `level_factor_steps` | `delete match arm "Annuity factor (r = g)" in level_factor_steps` | Equivalent (`equivalent.toml`) |
| B | `rates.rs:171` | `discount_factor_derivative` | `replace discount_factor_derivative -> Result<Decimal> with Ok(Default::default())` | Killed: `mutants_tvm.rs::xirr_documented_example_converges_by_newton_in_six_iterations` |
| B | `rates.rs:174` | `discount_factor_derivative` | `delete - in discount_factor_derivative` | Killed: `mutants_tvm.rs::xirr_monthly_compounding_converges_by_newton_in_six_iterations` |
| B | `rates.rs:175` | `discount_factor_derivative` | `delete - in discount_factor_derivative` | Killed: `mutants_tvm.rs::xirr_continuous_compounding_converges_by_newton_in_six_iterations` |
| B | `tvm.rs:61` | `npv_derivative` | `replace npv_derivative -> Result<Decimal> with Ok(Default::default())` | Killed: `mutants_tvm.rs::irr_documented_example_converges_by_newton_in_five_iterations` |
| B | `tvm.rs:64` | `npv_derivative` | `replace + with * in npv_derivative` | Killed: `mutants_tvm.rs::irr_documented_example_converges_by_newton_in_five_iterations` |
| B | `tvm.rs:64` | `npv_derivative` | `replace + with - in npv_derivative` | Killed: `mutants_tvm.rs::irr_documented_example_converges_by_newton_in_five_iterations` |
| B | `tvm.rs:65` | `npv_derivative` | `delete - in npv_derivative` | Killed: `mutants_tvm.rs::irr_documented_example_converges_by_newton_in_five_iterations` |
| B | `tvm.rs:290` | `require_sign_change` | `replace > with >= in require_sign_change` | Killed: `mutants_tvm.rs::zero_flows_are_neither_positive_nor_negative` |
| B | `tvm.rs:291` | `require_sign_change` | `replace < with <= in require_sign_change` | Killed: `mutants_tvm.rs::zero_flows_are_neither_positive_nor_negative` |
| B | `tvm.rs:424` | `solve_rate` | `replace > with >= in solve_rate` | Killed: `mutants_tvm.rs::irr_outside_the_grid_is_found_by_newton_from_the_guess` |
| B | `tvm.rs:464` | `solve_rate` | `delete match arm [] in solve_rate` | Killed: `mutants_tvm.rs::irr_outside_the_grid_is_found_by_newton_from_the_guess` |
| B | `tvm.rs:488` | `newton` | `replace newton -> Option<(Decimal, u32)> with None` | Killed: `mutants_tvm.rs::irr_documented_example_converges_by_newton_in_five_iterations` |
| B | `tvm.rs:496` | `newton` | `delete - in newton` | Killed: `mutants_tvm.rs::irr_documented_example_converges_by_newton_in_five_iterations` |
| B | `tvm.rs:496` | `newton` | `replace <= with > in newton` | Killed: `mutants_tvm.rs::irr_documented_example_converges_by_newton_in_five_iterations` |
| B | `tvm.rs:501` | `newton` | `replace < with <= in newton` | Killed: `mutants_tvm.rs::irr_newton_result_on_the_lower_end_of_the_interval_is_accepted` |
| B | `tvm.rs:501` | `newton` | `replace < with == in newton` | Killed: `mutants_tvm.rs::irr_newton_result_below_the_interval_falls_back_to_bisection` |
| B | `tvm.rs:501` | `newton` | `replace < with > in newton` | Killed: `mutants_tvm.rs::irr_documented_example_converges_by_newton_in_five_iterations` |
| B | `tvm.rs:501` | `newton` | `replace > with < in newton` | Killed: `mutants_tvm.rs::irr_documented_example_converges_by_newton_in_five_iterations` |
| B | `tvm.rs:501` | `newton` | `replace > with == in newton` | Killed: `mutants_tvm.rs::irr_newton_result_above_the_interval_falls_back_to_bisection` |
| B | `tvm.rs:501` | `newton` | `replace > with >= in newton` | Killed: `mutants_tvm.rs::irr_newton_result_on_the_upper_end_of_the_interval_is_accepted` |
| B | `tvm.rs:501` | `newton` | `replace match guard next < lo |  | next > hi with false in newton` | Killed: `mutants_tvm.rs::irr_newton_result_below_the_interval_falls_back_to_bisection` |
| B | `tvm.rs:501` | `newton` | `replace match guard next < lo |  | next > hi with true in newton` | Killed: `mutants_tvm.rs::irr_documented_example_converges_by_newton_in_five_iterations` |
| B | `tvm.rs:501` | `newton` | `replace |  | with && in newton` | Killed: `mutants_tvm.rs::irr_newton_result_below_the_interval_falls_back_to_bisection` |
| B | `tvm.rs:811` | `dated_npv_derivative` | `replace dated_npv_derivative -> Result<Decimal> with Ok(Default::default())` | Killed: `mutants_tvm.rs::xirr_documented_example_converges_by_newton_in_six_iterations` |
| C | `dcf/engine.rs:695` | `check_stages` | `replace > with >= in check_stages` | Killed: `mutants_dcf_engine.rs::three_stages_are_accepted` |
| C | `dcf/engine.rs:862` | `check_stages` | `replace > with >= in check_stages` | Killed: `mutants_dcf_engine.rs::horizon_of_ten_thousand_periods_is_accepted` |
| C | `dcf/engine.rs:1324` | `restate_growth` | `replace < with <= in restate_growth` | Killed: `mutants_dcf_engine.rs::real_decline_warning_only_below_inflation` |
| C | `statements/balance_sheet.rs:191` | `split_maturities` | `replace > with >= in split_maturities` | Killed: `mutants_balance_sheet.rs::split_beyond_one_year_equal_to_the_loans_is_not_refused` |
| C | `tvm.rs:1246` | `factor_with` | `replace < with <= in factor_with` | Killed: `mutants_tvm.rs::annuity_closed_form_applies_at_a_rate_gap_of_exactly_one_billionth` |
| C | `tvm.rs:1483` | `future_value` | `replace > with >= in future_value` | Killed: `mutants_tvm.rs::future_value_accepts_one_to_ten_thousand_periods` |
| D | `dcf/engine.rs:509` | `apply_defaults` | `replace == with != in apply_defaults` | Killed: `mutants_dcf_engine.rs::default_periods_reason_names_flows_or_lines` |
| D | `dcf/engine.rs:713` | `check_stages` | `replace && with |  | in check_stages` | Equivalent (`equivalent.toml`) |
| D | `dcf/engine.rs:733` | `check_stages` | `delete match arm (None, None) in check_stages` | Killed: `mutants_dcf_engine.rs::given_stage_without_flows_or_lines_names_both_fields` |
| D | `dcf/engine.rs:752` | `check_stages` | `replace match guard s.lines.is_some() with true in check_stages` | Killed: `mutants_dcf_engine.rs::empty_flows_or_lines_name_the_empty_field` |
| D | `dcf/engine.rs:756` | `check_stages` | `delete match arm Projection::Given in check_stages` | Killed: `mutants_dcf_engine.rs::empty_flows_or_lines_name_the_empty_field` |
| D | `dcf/engine.rs:1000` | `resolve_inflation` | `replace match guard !uses_terminal_rate with false in resolve_inflation` | Killed: `mutants_dcf_engine.rs::terminal_inflation_refused_without_a_terminal_rate` |
| D | `dcf/engine.rs:1022` | `resolve_inflation` | `replace match guard uses_terminal_rate with true in resolve_inflation` | Killed: `mutants_dcf_engine.rs::per_period_inflation_without_terminal_rate_needs_no_terminal_inflation` |
| D | `dcf/engine.rs:1125` | `resolve_terms` | `replace match guard i.single with true in resolve_terms` | Killed: `mutants_dcf_engine.rs::per_period_inflation_without_terminal_rate_needs_no_terminal_inflation` |
| D | `dcf/engine.rs:1397` | `project` | `replace match guard s.built_up.is_some() with true in project` | Killed: `mutants_dcf_engine.rs::given_flows_stage_convention` |
| D | `dcf/engine.rs:1448` | `project` | `replace && with |  | in project` | Killed: `mutants_dcf_engine.rs::negative_flow_warning_only_for_negative_flows` |
| D | `dcf/engine.rs:1497` | `build_up_stages` | `replace |  | with && in build_up_stages` | Killed: `mutants_dcf_engine.rs::financing_lines_refused_on_the_enterprise_basis_one_at_a_time` |
| D | `dcf/engine.rs:1670` | `run` | `replace + with * in run` | Killed: `mutants_dcf_engine.rs::periods_are_numbered_from_one_and_discount_factors_chain` |
| D | `dcf/engine.rs:1671` | `run` | `replace - with / in run` | Killed: `mutants_dcf_engine.rs::periods_are_numbered_from_one_and_discount_factors_chain` |
| D | `statements/balance_sheet.rs:529` | `statutory` | `delete ! in statutory` | Killed: `mutants_balance_sheet.rs::stated_treatments_are_assignments_of_analyst_origin` |
| D | `statements/balance_sheet.rs:530` | `statutory` | `delete ! in statutory` | Killed: `mutants_balance_sheet.rs::stated_treatments_are_assignments_of_analyst_origin` |
| D | `statements/balance_sheet.rs:572` | `statutory` | `delete ! in statutory` | Killed: `mutants_balance_sheet.rs::stated_treatments_are_assignments_of_analyst_origin` |
| D | `statements/balance_sheet.rs:869` | `statutory` | `replace && with |  | in statutory` | Killed: `mutants_balance_sheet.rs::lease_prepayment_without_a_source_is_noted_and_not_computed` |
| D | `statements/balance_sheet.rs:940` | `statutory` | `delete ! in statutory` | Killed: `mutants_balance_sheet.rs::lease_prepayment_without_a_source_is_noted_and_not_computed` |
| D | `statements/balance_sheet.rs:1358` | `statutory` | `replace |  | with && in statutory` | Killed: `mutants_balance_sheet.rs::gross_debt_and_cash_steps_are_written_once` |
| D | `tvm.rs:623` | `present_value` | `delete match arm (None, Some(curve)) in present_value` | Killed: `mutants_tvm.rs::present_value_period_rates_of_the_wrong_length_name_both_counts` |
| D | `tvm.rs:658` | `present_value` | `replace - with / in present_value` | Killed: `mutants_tvm.rs::present_value_explain_chains_the_discount_factors` |
| D | `tvm.rs:1274` | `factor_with` | `replace && with |  | in factor_with` | Killed: `mutants_tvm.rs::annuity_long_horizon_is_an_overflow` |
| D | `tvm.rs:1274` | `factor_with` | `replace < with > in factor_with` | Killed: `mutants_tvm.rs::limited_horizon_growth_above_rate_overflows_at_a_long_horizon` |
| D | `tvm.rs:1274` | `factor_with` | `replace match guard underflow_to_zero && ratio < one with true in factor_with` | Killed: `mutants_tvm.rs::limited_horizon_growth_above_rate_overflows_at_a_long_horizon` |
| D | `tvm.rs:1483` | `future_value` | `replace > with == in future_value` | Killed: `mutants_tvm.rs::future_value_accepts_one_to_ten_thousand_periods` |
| D | `tvm.rs:1483` | `future_value` | `replace |  | with && in future_value` | Killed: `mutants_tvm.rs::future_value_accepts_one_to_ten_thousand_periods` |
