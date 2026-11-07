---
name: commit
description: Prepare and create a commit — a Conventional Commit message in English, shown to the maintainer with a translation in their own language for review before committing. Use whenever changes are ready to be committed.
---

<!-- Generated from .agents/skills/commit/SKILL.md by scripts/sync_agents.py. Edit the source, not this file. -->

The repository's history is in English (it is read by contributors worldwide and generates the
CHANGELOG). The maintainer reviews in their own language: read `CLAUDE.local.md` for the
preferred review language (default: English, no translation).

1. Inspect the change: `git status`, `git diff --staged` (stage related files only; one task
   per commit).
2. Write the message:
   - subject: `<type>(<scope>): <imperative summary>` in English, at most 72 characters,
     citing the spec when relevant (`[SPEC-0001 R2]`); types and scopes are those in
     `docs/en/project/conventions.md`;
   - body: what changed and why, wrapped at 72 characters;
   - trailer `Refs: #<issue>` (the number in the branch name);
   - trailers when applicable: `Numerical-change: <method> v<N> -> v<N+1>`,
     `Docs-impact: none` (only for genuinely docs-neutral changes).
3. Show the maintainer, in their review language:
   - the English message exactly as it will be committed;
   - a faithful translation of subject and body;
   - a short summary of the files changed.
4. If the change is visible to users and has no fragment yet, propose one in `changes/`
   (`changes/README.md`), in plain language for analysts, and show its translation too.
5. Validate the message: `python scripts/conventions.py commit-msg <message file>`
   (after adding the sign-off line), then commit only after the maintainer approves:
   `git commit -s -F <message file>`.
   Never use icons or emoji.
