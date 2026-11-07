#!/usr/bin/env bash
# UserPromptSubmit hook: every request is read with the roadmap in mind. The agent decides
# whether the request changes the plan (only a model can read intent); the reminder makes sure
# the question is asked every time, and the roadmap-update skill makes the answer uniform.
cat <<'TEXT'
Roadmap check: if this request adds, changes, moves or drops planned work, propose the
roadmap change with the roadmap-update skill before doing the work (or say in one line that
it has no roadmap impact). Never create issues or change milestones outside
scripts/roadmap.py.
TEXT
