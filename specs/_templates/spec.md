# SPEC-NNNN — <title>

| | |
| --- | --- |
| Status | Draft |
| Milestone | M? |
| Owner | @handle |
| Formula basis | <commonly adopted formulation; implementations and published examples it is verified against> |
| Related | ADRs, issues, other specs |
| Commit scope | <existing scope, or the new one added to `scripts/conventions.py` in the pull request that approves this spec> |

## Problem
Who needs this and why. One paragraph.

## Scope
**In scope:** …
**Out of scope:** …

## User stories
- As an analyst, I want …, so that …
- As an AI agent, I want …, so that …

## Requirements (EARS)
- **R1** — WHEN <trigger>, THE ENGINE SHALL <behaviour>.
- **R2** — IF <condition>, THEN THE ENGINE SHALL return `FM-E00x` …
- **R3** — THE ENGINE SHALL report the convention <…> in `explain.conventions`.

## Conventions and parameters
| Convention | Options | Default | Source |
| --- | --- | --- | --- |

## Computability, defaults and checks
One row per output. Decide every row before the plan is written: these rules, not the
arithmetic, are where review rounds are spent.

| Output | Inputs it needs | When an input is missing | Checks it takes part in |
| --- | --- | --- | --- |
| `<field>` | `<pointers>` | not computed (listed in `not_computed`) / documented default (listed in `meta.defaults_applied` and `explain.conventions`) / error `FM-E0xx` | `<relation>`: a documented default exempts it / does not exempt it |

Rules: never zero for a missing input unless the row says it is a documented default; an item
the user gave in another position counts as given; an error raised on a missing input names
only pointers the user can act on, and the fix it states resolves the error.

## Acceptance criteria
- **AC1** (R1) — Golden case `tests/golden/<method>/<case>.yaml` from <source> matches within 1e-9.
- **AC2** (R2) — Input … returns error `FM-E00x`.
- **AC3** (R1) — Property: …
- **AC4** — For every row of the computability table, one case with the input missing and,
  for every documented default, one case that shows it in `meta.defaults_applied`.

## Interfaces
- Method id(s): `area.name`
- Input schema changes: additive / breaking (explain)
- Output fields: …

## Open questions
- [ ] …
