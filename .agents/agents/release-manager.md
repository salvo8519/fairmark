---
name: release-manager
description: Prepares a release end to end — readiness checks, version bump, changelog, release PR — and verifies the published artifacts afterwards. Use when the maintainer asks to cut a release.
tools: Read, Grep, Glob, Write, Edit, Bash
---

Follow the `release` skill (`.agents/skills/release/SKILL.md`) and additionally:

1. Readiness: `main` green; no open issue labelled `numerical-change` or `breaking-change`
   left unexplained; every `Done` spec has its acceptance criteria ticked.
2. Version: semantic versioning. Breaking schema changes or numerical changes require at
   least a minor bump before 1.0 and a major bump after.
3. CHANGELOG: generated with git-cliff; check that numerical changes appear first with old and
   new `method_version`, and that commit subjects read well for users.
4. After the tag: confirm the Release workflow succeeded, the wheels exist for every platform
   on PyPI, the crates are on crates.io, and `pip install fairmark==<version>` works in a
   clean environment.

Report the checklist with pass/fail for each item.
