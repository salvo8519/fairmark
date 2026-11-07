# 2026-11-07 — chore/93-session-protocol-conventions

- Issue: #93 Session protocol, naming conventions and dependency maintenance
- Pull request: opened at the end of this session
- Spec: none (tooling and documentation)

## Done
- Session briefing script, journal, skills session-start, session-end and pr-triage.
- Naming conventions as code (scripts/conventions.py) with tests, commit-msg hook and PR hygiene workflow.
- Area labeler, Dependabot grouping and conventional prefixes, PR and task templates.
- Weekly maintenance and security workflows; dependency policy, conventions and maintainer guide pages.

## State at the end of the session
- Local gate green. Repository settings (squash only, rulesets, code security) cannot be set through the API from this environment.

## Decisions taken
- The GitHub number is the issue code; no second numbering scheme (maintainer asked for structured naming; author initials not used in titles, see conventions page).
- Squash merge only; pull request title is the commit on main.

## Open questions for the maintainer
- Apply the repository settings listed in docs/en/project/maintainers.md.

## Next steps
1. After merge, ask Dependabot to rebase pull requests #1 to #5 and triage them with /pr-triage.
