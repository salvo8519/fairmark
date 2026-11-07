# SPEC-NNNN — <title>

| | |
| --- | --- |
| Status | Draft |
| Milestone | M? |
| Owner | @handle |
| Methodology sources | <standard / textbook / practitioner model> |
| Related | ADRs, issues, other specs |

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

## Acceptance criteria
- **AC1** (R1) — Golden case `tests/golden/<method>/<case>.yaml` from <source> matches within 1e-9.
- **AC2** (R2) — Input … returns error `FM-E00x`.
- **AC3** (R1) — Property: …

## Interfaces
- Method id(s): `area.name`
- Input schema changes: additive / breaking (explain)
- Output fields: …

## Open questions
- [ ] …
