# Session journal

Short, factual handover notes written at the end of every working session, so that the next
session (a person or an agent, on any machine) starts with full context instead of guessing.

The journal complements, and never replaces, the other sources of project memory:

| Source | Answers |
| --- | --- |
| Git history and `CHANGELOG.md` | What changed, technically |
| `changes/` fragments and `RELEASE_NOTES.md` | What changed, for users |
| Specs and ADRs | What was decided, and why |
| Issues and pull requests | What is planned, in progress, in review |
| This journal | Where a piece of work stopped, what is half done, what to do next |

## Rules

- One file per branch and day: `YYYY-MM-DD-<branch-slug>.md` (slashes in the branch name
  become dashes). Separate files avoid merge conflicts between parallel branches.
- Written by the `session-end` skill, read by the `session-start` skill.
- English, short sentences, no icons. Facts only: no client data, no financial figures from
  real engagements, no secrets.
- Committed with the work it describes. Entries are never edited after merge; a later session
  writes a new entry.
- Entries older than two releases may be pruned by the `release` skill.

## Search

`uv run just journal <query>` (or `uv run python scripts/journal.py search <query>`) finds the
entries about an issue (`327`: its own entries first, then the entries that mention it), an
area (`area:mcp`, from the epics of the roadmap), a day or month (`2026-11-16`, `2026-11`), or
any text. `journal.py index` lists every entry; `--json` gives agents the same data. The index
is built from the entries each time and never committed (ADR 0015, amended by #327).

## Template

Copy `_template.md`.
