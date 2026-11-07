---
name: explain-changes
description: Explain commits, pull requests, branches or a diff in the reader's language — what changed, why, impact on results, documentation and contracts. Use when the maintainer or a contributor asks to review or understand changes, for example in Italian.
---

1. Identify the scope: a commit range (`git log A..B`), a PR (`gh pr view N`, `gh pr diff N`),
   a branch versus `main`, or the working tree.
2. Read the commits and the diff. Group changes by area: core methods, adapters, tests and
   golden cases, schemas, documentation, tooling and CI.
3. Write the explanation in the requested language (default: the language of the request;
   for the maintainer see `CLAUDE.local.md`):
   - one-paragraph summary;
   - per area: what changed and why, citing files;
   - impact: numerical changes (golden cases, `method_version`), schema changes (additive or
     breaking), documentation and translations to update, risks;
   - questions or decisions for the reviewer.
4. Quote commit subjects and code identifiers in their original English. No icons or emoji.
