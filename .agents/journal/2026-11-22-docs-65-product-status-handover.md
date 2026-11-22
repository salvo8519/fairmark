# 2026-11-22 — docs/65-product-status-handover

- Issue: #65 Release v0.1.0 and announcement (status handover for the product line of work)
- Pull request: this one (documentation only)
- Spec: none

Handover of the product session (core, contracts, release). The process session (CI, tooling,
agents) keeps its own entries.

## Done since 2026-11-09
- Contracts: #279 structured errors; #361 one envelope with diagnostics and coded warnings
  (FM-W001 to FM-W017, SPEC-0012); #371 canonical order of error reports; #394 golden cases
  keyed by identity and closed-mode schema conformance (SPEC-0013). ADR 0014 accepted;
  constitution principle IX (unambiguous and transparent, ADR 0016).
- Quality: #298 missed mutants killed; #414 mutation testing within a measured budget, weekly
  rotation in 15 slices, explicit "mutation coverage incomplete"; #305 review effort
  proportional to risk.
- Product: #296 Python 3.12 floor; #311 the JSON envelope is the only output; #416 README status
  line and engineering notes.
- No pull request is open for the product line at the end of this session.

## State at the end of the session
- Package not yet published: the PyPI, Python and crates.io badges in the README stay as a
  sign of work in progress (maintainer decision, 2026-11-22).
- A Dependabot run for the Docker image of the release workflow failed on 2026-11-21; its log
  is not readable through the API.

## Decisions taken
- The maintainer keeps the full scope of v0.1 and v1.0; previews (`0.1.0a1`, `a2`, ...) are
  published as soon as the registry accounts exist, without reducing scope (2026-11-18).
- Contract work before the first release: #266, #263, #352, #353; after v1.0 the
  compatibility policy of #353 applies.
- Areas: this session works on the product (core, contracts, release); the other session on
  process (CI, tooling, agents). Before fixing a CI defect, check whether the other session
  is already on it (#417 and #419 fixed the same defect in parallel on 2026-11-22).

## Open questions for the maintainer
- Create the PyPI and crates.io accounts (trusted publishing).
- Settings > Actions > General: require approval for workflows from outside contributors.
- Paste the Dependabot log of the Docker ecosystem (Insights > Dependency graph > Dependabot).

## Next steps
1. With the accounts: build, install from scratch and publish `0.1.0a1`; first calculation
   from a notebook and from the MCP server.
2. Contracts: #266 null and one number grammar, then #263 statements v2, #352 one catalogue of
   codes exposed by every channel, #353 ADR on compatibility after v1.0.
3. #302 numerical edge cases, merged first with the overlapping M1 issues #175, #189, #217.
4. Verification of the contracts: #285 conformity of every method to ADR 0014, #286
   cross-adapter consistency.
5. Release and adoption: #65, #332, #229, #230, #59, #231, #307, #219.
6. Maintainability issues #402 to #410 from the audit of #411.
