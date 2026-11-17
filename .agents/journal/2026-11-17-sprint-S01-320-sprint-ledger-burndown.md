# 2026-11-17 — sprint/S01/320-sprint-ledger-burndown

- Issue: #320 Sprint burn-down, loop metrics and pull request queue automation (first part)
- Pull request: opens from this branch after the gate (sprint S01, process lane)
- Spec: none (tooling)

## Done
- The sprint event ledger: every `scripts/sprint.py state` call appends one JSON line to
  `ledger.jsonl`, beside the shared state directory (time, issue, lane, step, pull request,
  blockers, agent steps), in one append per line so parallel lanes never interleave.
- `blocked_since` in the per-issue state: set when blockers appear, cleared with them.
- `scripts/sprint.py burndown [--json]`: per day, each issue in exactly one state (done, in
  review, in progress, open); cycle time from the first checkpoint to closing, with the median
  and the issues closed without a checkpoint listed as not measured; failed check runs per
  lane; agent steps; blocked issues. Pure computation, GitHub facts gathered separately.
- Three false readings found by running it on S01 and fixed: pull requests that only mention
  an issue counted as its pull request (11 issues "in review"); a merged planning pull request
  kept #243 in review; commits shared by two pull requests of one issue counted twice. A pull
  request now belongs to an issue only from a branch of that issue, review means an open pull
  request that day, and each commit counts once.
- 11 new tests; two injected faults fail them (one only after adding the case of #243).

## State at the end of the session
- First part of #320. Still to do: the pull request queue (scheduled update of stale pull
  requests) and per-test failure statistics with quarantine notes.

## Decisions taken
- No `agent_cost` field: `agent_steps` already measures the agent cost of an issue, in
  orchestrator steps; a second name for the same figure would break constitution IX.

## Open questions for the maintainer
- none

## Next steps
1. #320 second part: pull request queue automation and per-test failure statistics.
