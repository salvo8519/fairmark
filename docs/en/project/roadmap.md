# Roadmap

The roadmap lives in [`roadmap/roadmap.yaml`](https://github.com/salvo8519/fairmark/blob/main/roadmap/roadmap.yaml)
and is mirrored on GitHub as milestones, epics and sub-issues. Tasks marked
`parallel` can be picked up independently; `good first issue` is a good place to start.
The Kanban board lives in the GitHub Project "Fairmark roadmap", created and filled by
`scripts/setup_github_project.py`.

Tasks are linked to their issues and, where one exists, to their specification. The state of
each task is copied from GitHub by `scripts/roadmap.py sync`, and a scheduled job reports any
difference with GitHub. The tables below are generated from `roadmap/roadmap.yaml` each time
the site is built.

--8<-- ".generated/ROADMAP.md"
