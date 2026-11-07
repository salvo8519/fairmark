---
name: adr-new
description: Record an architecture decision (ADR) for Fairmark. Use when a change affects module boundaries, adapters, schemas, dependencies, conventions or any constitution principle.
---

1. Delegate to the `architect` agent with the problem statement and constraints.
2. The ADR goes to `docs/project/adr/NNNN-<kebab-title>.md` with the next free number,
   status `proposed`, sections Context / Decision / Consequences.
3. Add it to the "Decisions (ADR)" section of `nav` in `zensical.toml`.
4. Ask the maintainer to accept it; only then set status `accepted` and start implementation.
