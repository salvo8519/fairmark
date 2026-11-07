---
name: pr-triage
description: Review the queue of open pull requests on Fairmark — CI and mergeability, conventions, linked issue and spec, numerical and contract changes, documentation and release notes — post factual review comments on GitHub and give the maintainer a merge recommendation for each. Use when the maintainer asks to review pending pull requests or the merge queue.
---

Talk to the maintainer in their preferred language; everything posted to GitHub is in
English, factual, courteous and without icons. **Never merge, approve or close** a pull
request: those decisions belong to the maintainer. Never push to a contributor's branch.

## 1. Inventory
`uv run python scripts/project_state.py --json` gives open pull requests with checks and
reviews. For each pull request also read, with `gh api`:
`repos/{owner}/{repo}/pulls/<n>` (mergeable state, base, draft, author, body),
`.../pulls/<n>/files` (changed paths) and `.../pulls/<n>/commits`.

## 2. Deterministic checks (per pull request)
| Check | How |
| --- | --- |
| Conventions | `python scripts/conventions.py pr --title ... --branch ... --body ... --label ...` |
| CI | all check runs green; name the failing jobs |
| Up to date | `mergeable_state` is not `behind` or `dirty` (conflicts) |
| Linked issue | the issue exists, is open, is in the current milestone or sprint |
| Spec | method changes cite an `Approved` spec (`specs/NNNN-*/spec.md`) |
| Golden cases | existing expected values unchanged, or label `numerical-change` and a `Numerical-change:` trailer |
| Contracts | `schemas/` changes are additive, or label `breaking-change` |
| Documentation | `scripts/docs_sync.py impact --base origin/main` on the pull request head |
| Release note | a fragment in `changes/`, or label `no-release-note` |
| Size and scope | one concern; flag unrelated files and diffs above about 600 changed lines |
| Overlaps | other open pull requests touching the same files (merge-order risk) |

To run local checks on a pull request, fetch it into a worktree, never into the current branch:
`git fetch origin pull/<n>/head:pr-<n> && git worktree add ../fairmark-pr-<n> pr-<n>`.
Remove the worktree when done.

## 3. Judgement (only for pull requests that pass step 2 or are close)
Delegate to `code-reviewer` (diff against AGENTS.md, the constitution and the spec); add
`numerical-auditor` when numbers or golden cases change and `security-reviewer` when
dependencies or workflows change. Bot pull requests (Dependabot) need only `security-reviewer`
for major versions and the release notes of the dependency.

## 4. Comments
Show the maintainer every comment before posting it, then post one review per pull request:
`gh api -X POST repos/{owner}/{repo}/pulls/<n>/reviews -f event=COMMENT -f body=@review.md`
(inline comments: add `comments[]` with `path`, `line`, `body`). Structure the body as:
- a one-sentence summary of what the pull request does;
- **Required changes**: numbered, each with the file, the rule it breaks (link to
  `docs/en/project/conventions.md`, `AGENTS.md` or the spec) and the fix;
- **Suggestions**: optional improvements, clearly marked as optional;
- a thank-you line for external contributors.
Do not repeat what CI already reports in its own annotations; refer to the job instead.

## 5. Report to the maintainer
One table, ordered by recommended merge sequence:

| PR | Title | CI | Conventions | Risk | Recommendation |
| --- | --- | --- | --- | --- | --- |

Recommendations: **Merge** (all green, no open points), **Merge after rebase**,
**Changes requested** (comment posted), **Needs your decision** (methodology, breaking change,
numerical change), **Close** (superseded or out of scope; propose the closing comment).
For overlapping pull requests, say which to merge first and why.
