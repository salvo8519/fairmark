---
name: translate
description: Bring every maintained translation of the documentation and README up to date with the English source. Use when an English page changed, when CI reports stale translations, or when the maintainer asks to sync the docs languages.
---

<!-- Generated from .agents/skills/translate/SKILL.md by scripts/sync_agents.py. Edit the source, not this file. -->

1. `uv run python scripts/i18n.py status` and show the maintainer what is stale or missing.
2. Delegate to the `translator` agent with the list from `uv run python scripts/i18n.py todo`.
3. Verify: `uv run python scripts/i18n.py check` and `uv run python scripts/docs.py build`.
4. Commit with `docs(i18n): update <lang> translation of <pages>` (signed off).

Adding a new maintained language is a decision for the maintainer (see `docs/en/project/` and
the onboarding skill): never add one on your own initiative.
