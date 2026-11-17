# 2026-11-17 — test-345-script-tests

- Issue: #345 Every process script tested; a script without a test fails the suite
- Pull request: opens from this branch after the gate
- Spec: none (tests)

## Done
- `python/tests/scripts/` holds the tests of the process scripts: twelve existing test files
  moved there (sprint, project state, workspace report, roadmap, conventions, required checks,
  MCP protocol check, mutants, release, wheel smoke test, i18n, site release notes), with a
  shared `conftest.py` (`load_script`, a temporary git repository).
- New tests for every script that had none: check_style, check_yaml, check_task_ids,
  check_golden_changes, release_notes, sync_agents, docs_sync, pin_actions, bootstrap_github,
  setup_github_project, tool_examples; and, in `python/tests/test_reference_oracles.py`, the
  numerical references coc, npv_irr, solver, time, schedule, values and ratios, against closed
  forms, against each other and against the golden values they produced.
- `test_every_script_is_tested.py` fails when a script is named by no test. On its first run it
  found three scripts the manual survey had missed (reference_ratios, reference_values,
  tool_examples): a name search had matched `reference_ratios_partial` and
  `tool_examples.json`.
- Seven injected faults (one per script: golden deletion ignored, duplicate task ids allowed,
  hand-edited mirror accepted, annotated tag not peeled, docs waiver ignored, 30/360 day rule,
  release section heading) each fail their tests.
- Found on the way: `reference_coc.py`, `reference_time.py` and `reference_values.py` set the
  global decimal precision when imported. The tests load them in a local context; a test
  proves the caller's precision is unchanged. The scripts themselves are product references
  and are not changed here.

## State at the end of the session
- Nothing half done. Full suite 4221 passed.

## Decisions taken
- Tests of the numerical reference scripts stay with the numerical tests in `python/tests/`
  (product lane); `python/tests/scripts/` is the process lane.

## Open questions for the maintainer
- Whether the three reference scripts should stop changing the global precision on import
  (as `reference_ratios.py` already does with a local 50-digit context).

## Next steps
1. #346: the agentic development page in the documentation site (EN and IT); then sprint S01.
