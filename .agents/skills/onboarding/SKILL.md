---
name: onboarding
description: Onboard a new contributor, forker or evaluator in their own language — guided tour of Fairmark's purpose, architecture, workflow and first tasks, with on-the-fly translation of any document. Use when someone new asks to understand, explore or start contributing to the project, in any language.
---

Detect the reader's language from their messages (ask if unclear). Everything you say is in
that language; the files you read stay as they are.

1. **Purpose** — summarise `README.md` and `docs/en/project/vision.md`: what Fairmark does,
   for whom, what makes it different.
2. **Principles** — explain `specs/constitution.md` in plain words, with one example each.
3. **Architecture** — walk through `docs/en/concepts/architecture.md` and the repository map in
   `AGENTS.md`; open one method end to end (`crates/fairmark-core/src/dcf.rs`, its golden
   cases, its documentation page) to make it concrete.
4. **How work is done** — spec-driven development (`specs/README.md`), agents and skills
   (`.agents/README.md`), the quality gate (`uv run just check`), sprints (`sprints/README.md`).
5. **Set up** — `uv sync --all-extras --all-groups`, `uv run just check`, and make sure it is
   green before anything else.
6. **First contribution** — list open `good first issue` issues
   (`gh issue list --label "good first issue"`) and suggest two that fit the person's skills
   (developer, valuation professional, translator).
7. **Reading in their language** — on request, translate any document on the fly in the
   conversation. Do not commit translations: maintained languages are decided by the
   maintainers. If the person wants to maintain a new language, explain the process:
   `i18n/<lang>.yaml`, a glossary, `docs/<lang>/`, `README.<lang>.md`, then a PR discussed
   with the maintainers first.

Contributions themselves (code, commits, issues, PRs) are in English, as stated in
`CONTRIBUTING.md`. No icons or emoji.
