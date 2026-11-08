# ADR 0012 — Method status lives in the registry, promotion evidence is checked in CI

Status: proposed · 2026-11-08 (issue #149)

## Context
The methods index marked some methods "preview" in a hand-edited column, and
`scripts/gen_reference.py` read that column to render the README tables. No page defined the
statuses or when a method may become available, and the engine did not report the status:
an agent calling `fm.methods()`, the MCP server or a function-calling tool could not tell a
preview method from an available one. Fairmark is used mainly by AI agents, so the status
must reach them through the same channel as the id and the version, and promotion must be
decided by a deterministic check, not by an agent's reading of the documentation.

Three places could hold the status:

1. a constant on the `Method` trait, next to each method's code (ADR 0009, decision 2);
2. a list in `registry.rs`, beside the list of method types;
3. a data file outside the core, checked against the registry.

A data file keeps the core free of release concerns, but every adapter would have to read it
(the core performs no I/O, constitution V), so there would be two catalogues to keep in step.
A trait constant touches every method type and the trait itself, and gives a new method no
safe default: its author would choose the status.

## Decision
1. **The registry reports the status.** `MethodInfo` gains `status`, `available` or
   `preview` (`registry::Status`). Every adapter reads it from the catalogue it already uses:
   `fm.methods()`, `fairmark methods`, the MCP tool list, the function-calling definitions
   and the generated documentation (methods reference, methods index, README tables).
2. **Promotion is one explicit list.** `registry.rs` holds `AVAILABLE`, the ids of the
   methods promoted to available. A method not in the list is preview: a new method is
   preview until someone promotes it in a reviewed one-line change. A compile-time
   assertion rejects an id in `AVAILABLE` that is not registered. The list of method types
   (ADR 0009, decision 3) is unchanged.
3. **Evidence is data, checked in CI.** The promotion rule is defined in
   `docs/en/project/releases.md`. The evidence for each spec (which methods it covers,
   which `tasks.md` tasks record the independent numerical audit and the code review) is in
   `specs/method-evidence.yaml`. `scripts/method_status.py check` fails when an available
   method lacks evidence: spec approved with no unchecked item, spec naming the method,
   golden cases each with a `source`, audit and review tasks ticked.
4. **Methods available before the rule are listed, not hidden.** The checker keeps a fixed
   set of methods that were available when the rule was introduced and do not yet meet it.
   They are reported on every run; the set may only shrink (the check fails when one of them
   meets the rule or becomes preview). Promoting a method on any other grounds needs a change
   to the checker, which is visible in review.
5. **Status is not a version.** Both statuses follow ADR 0009: every numerical or breaking
   change is a new method version with its own schema. The status states how much
   independent verification a method has had, so that an agent can tell its user.

## Consequences
+ One source of truth: every adapter and every generated page reports the same status.
+ A new method cannot be available by accident; promotion is a reviewed one-line change that
  CI checks against recorded evidence.
+ Parallel work on new methods does not touch the status list.
− Status is not next to the method's code, unlike the other metadata of ADR 0009; the
  compile-time assertion and the generated reference compensate.
− Evidence refers to task ids in `tasks.md`; renumbering a task requires updating the
  evidence file (the check fails until it is).
