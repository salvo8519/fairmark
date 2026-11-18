---
name: plan-sprint
description: Plan the next Fairmark sprint from open GitHub issues into sprints/SNN.yaml with parallel lanes. Use when the maintainer asks to plan or prepare a sprint.
---

<!-- Generated from .agents/skills/plan-sprint/SKILL.md by scripts/sync_agents.py. Edit the source, not this file. -->

1. Delegate to the `sprint-planner` agent with the current milestone and any priorities the
   maintainer stated.
2. Show the maintainer the proposed goal and lanes, with the `paths` of each lane and the
   generator groups each lane regenerates (`generates`); apply their changes. `uv run python scripts/sprint.py validate
   sprints/SNN.yaml` must pass: lanes with overlapping paths cannot run in parallel.
3. When approved, set `status: active` and commit `chore(sprint): plan SNN` (signed off).
