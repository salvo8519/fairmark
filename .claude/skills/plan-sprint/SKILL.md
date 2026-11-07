---
name: plan-sprint
description: Plan the next Fairmark sprint from open GitHub issues into sprints/SNN.yaml with parallel lanes. Use when the maintainer asks to plan or prepare a sprint.
---

<!-- Generated from .agents/skills/plan-sprint/SKILL.md by scripts/sync_agents.py. Edit the source, not this file. -->

1. Delegate to the `sprint-planner` agent with the current milestone and any priorities the
   maintainer stated.
2. Show the maintainer the proposed goal and lanes; apply their changes.
3. When approved, set `status: active`, commit `chore(sprint): plan SNN` (signed off) and
   remind the maintainer to set the Iteration field of these issues in the GitHub Project.
