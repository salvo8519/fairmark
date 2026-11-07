# 2026-11-07 — main (several branches, all merged)

- Issues: #103, #106, #108, #110, #111, #113, #114 to #117, #119, #121, #123, #125, #127, #132
- Pull requests merged: #94, #102, #104, #105, #107, #109, #112, #118, #120, #122, #124, #126,
  #128, #129, #130
- Specs: SPEC-0008 complete (R1 to R23); SPEC-0006 and SPEC-0007 approved

## Done
- Architecture: Method trait per theme, one registry list, versioned schema history (#104,
  ADR 0009).
- Analysis theme complete: goal seek with Brent's method on decimals (#109, ADR 0010), then
  sensitivity tables, scenarios and tornado (#129). Work bound: 10,000 evaluations and
  1,000,000 evaluations × values of base_input (FM-E016).
- Roadmap as code (#118, #120): every task names its issue and spec; CI refuses a pull request
  that closes an unplanned issue or does not mark its task done; scheduled drift check;
  `roadmap.py new | list | sync`; skill `roadmap-update` and a UserPromptSubmit reminder (#124).
- Public pages state only what exists (#122): plans live in the generated roadmap page;
  `docs_sync.py claims` in CI; README methods table generated from the registry.
- Releases: version policy (0.1.0 = M0 to M3) and `release.py ready` (#126); release notes and
  changelog pages current at every site build, numerical changes listed first (#128).
- Workflows: every action pinned to a commit; zizmor, actionlint, YAML validity and
  cargo-machete in CI; the @claude workflow limited to owner, members and collaborators;
  crates.io publishing after all checks; retired macOS runner replaced (#130).

## State at the end of the session
- Nothing half done. `main` green; roadmap and GitHub agree.
- Open pull request #131 (Dependabot): major versions of setup-uv (7 to 10),
  add-to-project (1 to 2) and action-gh-release (2 to 3). CI green, but the release workflow
  is not exercised by CI: read the three changelogs before merging (dependency policy: one
  major version at a time).

## Decisions taken
- Maintainer, 2026-11-07: SPEC-0008 recommendations (explicit bracket, per-cell errors,
  explicit tornado shocks, Monte Carlo in a later spec); SPEC-0006 and SPEC-0007 decisions;
  version policy; roadmap-update workflow; inflation as a declared basis (nominal or real)
  rather than independent flags (#110).
- Maintainer, 2026-11-08: SPEC-0008 amendments (AC6 without a spreadsheet table, R5 work bound,
  R19 overflowing deltas recorded).
- Naming rule, maintainer: identifiers and documentation use the terminology of international
  and European financial practice, never spreadsheet function names; Italian names go in the
  glossary.

## Open questions for the maintainer
- Confirm the proposed roadmap change: in M1, epic Time value of money, three tasks for loan
  functions (payment, number of periods, future value) and amortisation schedules (annuity,
  linear, bullet, interest-only period), with SPEC-0009.
- Pull request #131: merge after the changelog review, or split per action.

## Next steps
1. If the maintainer confirms the loan functions: `roadmap.py new` for the three tasks, then
   the `spec-writer` for SPEC-0009 with terms verified on recognised sources.
2. Otherwise continue M2: #114 (ratios and covenants, SPEC-0007) and #115 (terminal value
   building blocks, SPEC-0006) can run in parallel (different theme modules).
