# Specs — spec-driven development

Every feature that changes what Fairmark computes or exposes starts as a spec. Code follows
the spec, tests follow the acceptance criteria, and reviewers check the code against both.

```
specs/
  constitution.md          non-negotiable principles (read first)
  _templates/              spec.md · plan.md · tasks.md
  NNNN-short-name/
    spec.md                WHAT and WHY — requirements and acceptance criteria
    plan.md                HOW — design, types, schema changes, conventions, risks
    tasks.md               ordered, test-first checklist; [P] = parallelisable
```

## Lifecycle

| Status | Meaning | Who moves it |
| --- | --- | --- |
| `Draft` | Being written; open questions allowed | anyone |
| `Approved` | Requirements frozen; implementation may start | maintainer |
| `Implementing` | Tasks in progress; PRs reference the spec id | implementer |
| `Done` | All acceptance criteria verified by tests; docs published | maintainer |
| `Superseded` | Replaced by a newer spec (link it) | maintainer |

## Rules

1. **No method code without an `Approved` spec.** Refactors, docs and tooling don't need one.
2. **Requirements use EARS phrasing** ("WHEN …, THE ENGINE SHALL …") and are numbered
   (`R1`, `R2`, …) so tests and PRs can cite them.
3. **Every requirement has at least one acceptance criterion**, and every acceptance criterion
   becomes a test: golden case, property test or unit test. Tests are written before the code.
4. **Methodology comes from a named source** (standard, textbook, practitioner model).
   Conventions that practitioners disagree on become explicit parameters with documented
   defaults.
5. **PR titles and descriptions cite the spec** (`feat(dcf): FCFF build-up [SPEC-0001 R3]`).
6. Changing an `Approved` spec requires a PR that edits the spec first.
7. **Names come from financial practice.** Method, field and value names use the terminology
   of international and European financial practice (regulation, central banks, supervisors,
   professional curricula), each verified on a recognised source cited in the spec's
   "Terminology and sources" section with its Italian equivalent. Spreadsheet function names
   are not used as names, and terms are never invented.

## Agent workflow

`/spec-new` → write `spec.md` → maintainer approves → `plan.md` + `tasks.md` →
`/implement-spec` (test-first, one task per commit) → `/pre-pr` (checks + review subagents) → PR.

## Methodology: how formulas are chosen and defended

Fairmark implements the formulas that practitioners commonly adopt, written cleanly in our own
words. A formula is defended by evidence, not by citations: each method is verified against a
published worked example (golden case) and against widely used implementations (spreadsheet
functions, established open-source libraries) that compute it the same way. Specs state the
formula basis in one line; methodology pages end with a short "Verified against" section.
Standards are referenced only where they define a convention the market uses (for example
ISDA day counts). Code from other projects is never copied; results are compared.
