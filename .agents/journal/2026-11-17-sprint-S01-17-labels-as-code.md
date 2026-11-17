# 2026-11-17 — sprint/S01/17-labels-as-code

- Issue: #17 Labels, GitHub Project board, Discussions
- Pull request: labels as code; Discussions and label clean-up are maintainer settings
- Spec: none (repository configuration)

## Done
- Found that the issue templates apply `method-request` and `numerical`, which do not exist on
  GitHub, so GitHub silently dropped them; `bug`, `area:project` and the Dependabot labels
  existed but were not declared.
- Declared every label in use in `roadmap/roadmap.yaml`; test `test_labels.py` fails when a
  label named in templates, labeler or workflows is not declared.
- Maintainer guide: Discussions setting with its categories, and the labels rule.

## State at the end of the session
- Missing labels to create on GitHub after the merge (method-request, numerical, test-health).

## Decisions taken
- Triage labels kept and declared: question, duplicate, invalid, wontfix.

## Open questions for the maintainer
- Turn on Discussions (the docs already send readers there).
- Delete the unused default labels: accessibility, documentation, enhancement.
- The project board: the project uses milestones; drop the board from #17 and remove
  `project-sync.yml` and `scripts/setup_github_project.py`?

## Next steps
1. The answers above; then close #17.
