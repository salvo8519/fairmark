# ADR 0016 — Constitution principle IX: unambiguous and transparent

Status: accepted · 2026-11-17 (#359; approved by the maintainer on 2026-11-17)

## Context

Fairmark is used by analysts and by AI agents in loops. Both trust a result only when nothing in
it is left to interpretation and nothing happens out of sight. The existing principles cover
parts of this: no silent defaults (III), an audit trail (II), public methodology (VII), explicit
units (VIII). Recent work showed the same need in other places: a number rounded at reading
time, a warning written as free text, a field name that held a pointer without saying so, a
rule that two agents read in two ways. The maintainer asked that ambiguity be avoided in the
code and in the documentation alike, because the project's aim is to build trust.

## Decision

Add principle IX to `specs/constitution.md`:

- Every name, value, message and document has one meaning. A term means the same thing in the
  code, the schemas, the documentation and both language editions; a value is never interpreted
  by guesswork (units, sign, order, period).
- Whatever the engine does on the user's behalf is visible: a default, a rounding, a
  reinterpretation or a value not computed is reported in the result, with its reason. Nothing
  changes silently.
- Documentation states only what exists and what is verified; limits and known gaps are written
  down, not left out.
- When two readings of a rule are possible, the rule is rewritten until one remains.

## Consequences

- Reviews and agents check principle IX like every other principle; a finding against it is
  fixed in the same pull request.
- Ambiguity is treated as a defect: a name, a message or a sentence of documentation that admits
  two readings is changed, even when the code behaves correctly.
- The principle is enforced by checks where it can be (warning catalogue, pointer grammar,
  glossary, the documentation rule that pages describe only what exists) and by review where it
  cannot.
