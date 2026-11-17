# 2026-11-17 — docs-346-agentic-development

- Issue: #346 Agentic development architecture page in the documentation site
- Pull request: opens from this branch after the gate
- Spec: none (documentation)

## Done
- `docs/en/project/agentic-development.md` and its Italian edition: the loop from roadmap to
  merge (diagram in the brand palette), one source with generated mirrors, the twelve roles,
  the three memory tiers, sprint lanes and their checks, the table of gates with the script
  that enforces each rule, review by risk tier, what the maintainer decides, and what is not
  enforced yet (#320, #327, #328). In the Project section of the site.
- Project pages were English only. A page listed as `required` in `i18n/it.yaml` is now
  translated even inside an English-only section, in `scripts/i18n.py` (translation check) and
  `scripts/docs.py` (Italian navigation); two tests, failing first.
- `AGENTS.md`, `.agents/README.md` and `sprints/README.md` link the page.
- Both sites build in strict mode; the Italian navigation links the Italian page.

## State at the end of the session
- Nothing half done.

## Decisions taken
- The maintainer asked for the page in the site, in English and Italian (17 November).

## Open questions for the maintainer
- none

## Next steps
1. Sprint S01: process lane (#327, #320, #328, #335 ADR) and product lane (M1).
