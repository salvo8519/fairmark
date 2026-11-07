<!--
Title: a Conventional Commit header, for example `feat(dcf): add mid-year discounting`.
It becomes the commit on main and the CHANGELOG line. See docs/en/project/conventions.md.
-->

## Summary
<!-- Two to four sentences: what changes and why, for a reader who has not seen the issue. -->

## Linked issue
Closes #

## Specification
<!-- SPEC-NNNN and the requirement ids covered, or "Not applicable" (tooling, docs). -->

## Changes
<!-- One line per change, grouped by area: core, adapters, documentation, CI. -->

## Numerical impact
<!-- "None", or: methods affected, old and new values, method_version bump, independent source. -->

## Contracts
<!-- Schema snapshots changed (breaking or not), error codes added, CLI or MCP changes. Or "None". -->

## Testing
<!-- Golden cases and tests added; commands run and their result. -->

## Risks and follow-ups
<!-- Known limitations and the issues opened for later. Or "None". -->

## Checklist
- [ ] Title, branch and linked issue follow the conventions
- [ ] `uv run just check` passes locally
- [ ] Tests for every acceptance criterion; golden cases carry their independent source
- [ ] No change to existing golden results, or label `numerical-change`, `method_version` bump and `Numerical-change:` trailer
- [ ] Documentation updated (methodology page, guides, generated references); Italian edition updated or label `translation-pending`
- [ ] Release note fragment in `changes/`, or label `no-release-note`
- [ ] No real client data; commits signed off (DCO)
