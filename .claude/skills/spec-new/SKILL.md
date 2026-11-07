---
name: spec-new
description: Start a new feature spec for Fairmark from the templates (spec-driven development). Use when the user asks for a new method, behaviour change or any feature that changes what Fairmark computes or exposes.
---

1. Pick the next number: list `specs/` and take the highest `NNNN` + 1.
2. Create `specs/NNNN-<kebab-name>/` and copy `specs/_templates/spec.md` into it.
3. Delegate drafting to the `spec-writer` subagent with the user's request and any
   material they provided (models, examples, sources).
4. Show the user: the requirements list, the conventions table and the open questions.
   Ask them to answer the open questions and to approve.
5. Only when the user explicitly approves: set `Status: Approved`, then ask `spec-writer`
   to produce `plan.md` and `tasks.md`.
6. Add or update the epic in `roadmap/roadmap.yaml` if the spec introduces new work.
