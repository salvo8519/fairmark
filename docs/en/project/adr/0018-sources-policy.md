# ADR 0018 — Sources policy

Status: accepted · 2026-11-18 (#208; records the maintainer's decision of 2026-11-08 under
constitution principle VII; accepted by the maintainer on 2026-11-18)

## Context

Constitution principle VII asks that every method has a documentation page with formula,
conventions, example and sources, and that practice varying across jurisdictions is modelled
as explicit profiles. Principle VI asks that golden cases come from sources independent of the
implementation. Neither says where each kind of source is written, how much of it a public page
carries, which vocabulary applies where Italian practice has no settled term, or what a page
may claim about regulation.

The repository today:

- Golden cases record their source in a `source` field (every file under `tests/golden/`).
- Specs cite sources where they decide a convention (the `Source` column of the conventions
  table of the spec template); the template has no section that gathers them.
- The Italian glossary (`i18n/glossary.yaml`) already applies the maintainer's rule of
  2026-11-08 on terms: the term of international practice where no verified Italian term
  exists.
- Public method pages carry two kinds of references. 44 of the 45 pages have a "Verified
  against" section, as the `docs-writer` role asks: the published examples the golden cases
  reproduce, with their figures, and the implementations that agree (median 5 lines, longest
  40, `dcf-fcff.md`). Three pages also have a "Sources" section of 7 to 18 lines
  (`dates-schedule.md`, `series-project.md`, `statements-reorganise.md`), including items still
  marked "to verify".
- The `spec-writer` role asks for no bibliographies in specs: one line naming the formulation,
  the published worked example and the cross-check implementations.
- Independent oracles used in tests (`pyxirr`, `numpy-financial`, QuantLib, python-dateutil)
  are cited in some places and not others.

On 2026-11-08 the maintainer decided the policy below (#208). This ADR records it and states
how it is applied and checked.

## Decision

1. **Full sources live with the evidence, not on public pages.** The complete list of sources
   of a method, with editions, sections and what each supports, is written in its spec (a
   `Sources` section) and, case by case, in the `source` field of its golden cases. Terms and
   their sources live in `i18n/glossary.yaml`. These are the records an auditor reads.
2. **A public page carries at most one line of references.** The `Sources` section of a method
   page is one line: the principal references by name (for example "CFA Institute curriculum;
   IFRS 13; Damodaran, Investment Valuation, 3rd ed.") and a link to the spec for the full
   list. A page never shows an item marked "to verify": an unverified source stays in the spec
   until it is verified or removed. The "Verified against" section is not a list of references
   and is not limited: it is the evidence of principle VI that a reader checks, each line naming
   a published example and the figures the golden cases reproduce (maintainer decision of
   2026-11-18).
3. **Vocabulary follows international practice where Italian practice is not settled.** The
   reference frameworks are CFA Institute, EBA, IFRS and IVS. Where no verified Italian term
   exists, or Italian sources conflict with each other or with those frameworks, the English
   term of international practice is kept, with an Italian explanation in parentheses where it
   helps (the rule of `i18n/glossary.yaml`). A term is never translated on the basis of an
   unverified source.
4. **Documents claim alignment with practice, never compliance with the law.** Pages, specs,
   release notes and messages say "aligned with practice" (or name the framework followed, for
   example "follows IVS 105"). They never say "compliant with the law", "conforme alla legge",
   "certified" or equivalent: Fairmark computes, the analyst signs. A rule of a specific
   jurisdiction is modelled as an explicit profile (principle VII), described by what it does,
   not as a legal guarantee.
5. **Mature libraries are cross-checks, not sources of truth.** Libraries such as `pyxirr`,
   `numpy-financial`, QuantLib and python-dateutil serve as independent oracles in tests
   (principle VI). A spec names the oracle it uses and its exact version where results depend
   on it; an oracle's behaviour never replaces a published source for a convention.
6. **Constitution VII is clarified, not changed in substance.** Its first line becomes: "Every
   method has a documentation page with formula, conventions, example and a one-line reference
   to its sources; the full sources are in its spec and golden cases." One reading remains
   (principle IX).

## Consequences

- An auditor finds every source in one place per method (the spec), with the evidence per case
  in the golden files; a reader of the site sees a short, verified reference line.
- Three "Sources" sections shrink to one line and move their detail to their specs; their
  Italian editions follow. Items marked "to verify" leave the public pages. "Verified
  against" sections stay as they are.
- The spec template gains a `Sources` section; the `spec-writer` role changes from "no
  bibliographies" to "every source used, with edition and section, in the `Sources` section";
  the `docs-writer` role applies the one-line rule.
- `scripts/docs_sync.py claims` gains two checks on public pages: a `Sources` section of at most
  one line (not `Verified against`), and none of the compliance phrases of decision 4 (English
  and Italian).

## Implementation

| Task | Lane | Issue |
| --- | --- | --- |
| Constitution VII wording (decision 6) | governance, with this ADR | #208 |
| `docs-writer` and `spec-writer` roles | process | #208 |
| `Sources` section in the spec template; the three method pages and their Italian editions | product | #387 |
| The two checks in `docs_sync.py claims`, merged after #387 | process | #388 |

## Decisions of the maintainer

- 2026-11-18: ADR accepted; `Verified against` stays on the pages, as evidence.
