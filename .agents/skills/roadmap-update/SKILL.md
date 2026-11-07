---
name: roadmap-update
description: Propose and apply the roadmap change that a request implies — new task, new epic, a task moved to another milestone, a change of scope, or work dropped — through scripts/roadmap.py, after the maintainer confirms. Use whenever a request adds, changes or drops planned work, or when the maintainer asks to update the roadmap.
---

Talk to the maintainer in their preferred language; everything written to the repository or to
GitHub is in English, without icons. The roadmap is `roadmap/roadmap.yaml`; never edit
milestones or issues on GitHub by hand.

1. **Classify the request.** One of:
   - **Already planned**: an existing task covers it. Name it (`uv run python
     scripts/roadmap.py list --open`), and say whether its scope must change.
   - **New task** in an existing epic, or **new epic** when no epic fits.
   - **Moved**: a task changes milestone (priority).
   - **Changed scope**: the title or the spec of a task changes.
   - **Dropped**: planned work will not be done.
   - **No impact**: a question, or work inside the current issue's scope. Say so in one line
     and stop.
2. **Propose**, in one short block, before doing the work: milestone, epic, title (the
   outcome, sentence case), labels (`area:*`, `method:*`, `parallel`, `needs-methodology`),
   spec (existing id, or "spec needed" for method work), dependencies on other tasks, and the
   reason for the milestone you chose. Methodology choices that are not settled get
   `needs-methodology`. Wait for confirmation, unless the maintainer has already stated the
   change explicitly.
3. **Apply** what was confirmed:
   - new task: `uv run python scripts/roadmap.py new --milestone M<n> --epic "<epic>" --title
     "<title>" --labels <labels> [--spec NNNN] [--body FILE]`;
   - moved or changed scope: edit the task in `roadmap/roadmap.yaml`, run
     `uv run python scripts/roadmap.py render`, then
     `uv run python scripts/roadmap.py sync --apply-milestones` (and update the issue title on
     GitHub if it changed);
   - dropped: close the issue as not planned with a comment that says why
     (`gh api -X PATCH repos/{owner}/{repo}/issues/<n> -f state=closed -f
     state_reason=not_planned`), then `uv run python scripts/roadmap.py sync --write`: the task
     stays in the file with status `dropped`, so the history remains visible.
4. **Record** the change: in the current pull request when it belongs to its issue, otherwise in
   a `chore(project)` pull request. `uv run python scripts/roadmap.py check` must pass.
5. **Report** to the maintainer: what changed in the plan, which issue, and what it means for
   the order of work.
