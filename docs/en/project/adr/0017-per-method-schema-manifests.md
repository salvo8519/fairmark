# ADR 0017 — Per-method schema manifests and a reference assembled at build time

Status: proposed · 2026-11-17 (#335; supersedes the merge-order part of #319 for the `registry`
group)

## Context

Every method change regenerates files that every other method change regenerates too. Today
`uv run just generate` writes, for the `registry` group of generated artifacts:

| File | Written from | Shape |
| --- | --- | --- |
| `schemas/<method>/v<N>.json`, `v<N>.output.json` | the engine | one folder per method |
| `schemas/index.json` | the snapshots | one file for every method and the envelope |
| `docs/en/reference/methods.md`, `docs/it/reference/methods.md` | the registry | one page for every method |
| status block of `docs/<lang>/methods/index.md` | the registry | one table for every method |
| methods block of `README.md`, `README.it.md` | the registry | one table for every method |
| `python/fairmark/tool_examples.json` | golden cases named in `TABLE` of `scripts/tool_examples.py` | one file for every method |

Two pull requests that add two different methods both rewrite the five shared files, and git
reports a conflict when the lines they change are near each other (methods are sorted, so two
methods with close names always collide; `TABLE` gains a line in the same place). #319 settled
this with a merge order (`sprints/README.md`, Generated artifacts and the merge order): one
generating lane per sprint, generated pull requests merged one at a time, rebase and
regenerate after each merge. The policy works, but it serialises the work that benefits most
from parallel lanes (new methods), and it holds only as long as agents follow written rules.

The schema history is a contract (ADR 0009, ADR 0014 G8): the files and hashes of an earlier
version never change, a current version never goes back, a method is never removed silently.
`scripts/schema_snapshots.py --diff-against` enforces it against the base branch, reading
`schemas/index.json` on both sides. Readers of that file today: `scripts/schema_snapshots.py`
(write, check, release stamping, diff), `scripts/envelope_inventory.py`, and the tests
`test_schema_versions.py` and `test_envelope_inventory.py`. No adapter reads it at runtime.
Approved specs name the file as the place where a version is recorded.

## Decision

1. **Each method keeps its own manifest; the history is committed per method.**
   `schemas/<method>/index.json` records the method's current version and, for every version,
   the files, their SHA-256 and the release that introduced it, with exactly the records
   `schemas/index.json` holds today (`format: 1`). The envelope keeps its own at
   `schemas/envelope/index.json`. A pull request that adds or changes a method changes files
   in that method's folder only. The manifests are the source of truth of the history and are
   always committed: the history cannot be regenerated from the engine.

2. **The global view is assembled, never committed.** `schemas/index.json` is removed from the
   repository. One function, `schema_history()` in `scripts/schema_snapshots.py`, assembles it
   from the manifests in the same format as today, and every reader (the scripts, the tests,
   the release and documentation builds) reads it through that function and never opens a
   manifest or the old file directly (#335 acceptance criterion 4). The documentation build
   publishes the assembled file at `schemas/index.json` of the site, and the release attaches
   it to the GitHub Release, so a consumer that reads the published history sees the same
   format. The name "schema history" means this assembled view in every document; approved
   specs that name `schemas/index.json` refer to it.

   Rejected: keeping the assembled file committed and rewriting it only by a check on `main`.
   A write to `main` from a workflow needs its own pull request through the ruleset "Protect
   main" (a bot pull request after each merge, with its own CI run) or a bypass of the ruleset,
   which ADR 0015 and #372 keep closed. Either way the file still changes in every method pull
   request that wants CI to pass.

3. **Immutability is checked per method, with the same rules.** `--diff-against` compares the
   manifests of the base branch with those of the branch, method by method, with the existing
   rules unchanged: an earlier version's records and files are identical, `introduced_in`
   changes only from `unreleased` through `--stamp-release`, the current version never goes
   back, a method folder never disappears. A method present in the base history and missing
   from the branch is a removal and fails, whether its manifest or its folder is missing.
   `--stamp-release` rewrites only the manifests that hold `unreleased` versions; in one
   release pull request that is expected, because no lane runs then.

4. **The migration changes no snapshot and no hash.** One pull request writes every manifest
   from the current `schemas/index.json` and removes it. A test proves that the assembled view
   equals the old file's content (parsed JSON, key for key) and that no `v<N>.json` or
   `v<N>.output.json` changed. For one release, `--diff-against` also reads a base branch that
   still has the old single file, so the pull request of the migration is checked by the same
   rules; that reader is then removed.

5. **The methods reference and the status tables are assembled at build time.**
   `docs/<lang>/reference/methods.md` and the status block of `docs/<lang>/methods/index.md`
   are no longer committed: `scripts/docs.py build` runs `scripts/gen_reference.py` before
   building, as it already runs other generators, and the published site is unchanged.
   `scripts/docs_sync.py`, `scripts/i18n.py` and the strict build read the generated pages from
   the build directory. The index row of each method on `docs/<lang>/methods/index.md` stays a
   hand-written line, added by the pull request of the method.

6. **The README lists the method themes, not every method.** The README is read on GitHub and
   on PyPI, where no build runs, so its methods block cannot be assembled at read time. It
   becomes one hand-written line per theme of ADR 0009 (today `dates`, `rates`, `tvm`,
   `loans`, `coc`, `dcf`, `tv`, `ratios`, `statements`, `series`, `analysis`) that links to the
   methods reference for the list and the status of each method; a theme line changes when a
   theme is added, rarely. `scripts/docs_sync.py claims`
   keeps checking that the README names only themes that exist.

7. **Tool examples live beside the golden cases of each method.** The choice of the minimal
   and maximal example moves from `TABLE` in `scripts/tool_examples.py` to a small file in the
   method's golden folder (`tests/golden/<method>/tool_examples.yaml`), and the package ships
   one file per method (`python/fairmark/tool_examples/<method>.json`), read through one
   function in `python/fairmark/tools.py`. Two methods never touch the same file.

8. **The one-generating-lane rule stays only where files are still shared.** After 1–7 the
   `registry` group has no file that two method pull requests both change, so
   `sprints/README.md` drops the merge order for it and `scripts/sprint.py` no longer assigns
   the group to a single lane. The rule stays for the `agents` group (the mirrors of
   `.agents/`, which change rarely). Hand-edited shared lists (the registry list in
   `registry.rs`, `specs/method-evidence.yaml`, the navigation in `zensical.toml`, the index
   rows) remain: they change by one line per method, and a conflict there is a two-line merge
   by hand after a rebase, not a regeneration.

## Consequences

- Two branches that each add a different method merge without conflict in `schemas/`, in the
  reference or in the tool examples. A test reproduces it with two synthetic methods in a
  temporary repository (#335 acceptance criterion 3).
- Lanes that add methods can run in parallel within a sprint; the planner no longer limits a
  sprint to one lane that generates `registry`.
- A reader that opened `schemas/index.json` in a checkout must call `schema_history()`
  instead; an external consumer reads the file published with the documentation or the
  release, in the same format. The change of location is written in the changelog of the
  migration and in `docs/en/project/releases.md`.
- The committed tree no longer shows the methods reference on GitHub; the site does. Pages of
  the repository that linked to the generated file link to the site instead.
- More files (one manifest and one example file per method), each small and owned by one
  method; the history check reads all manifests, which costs milliseconds.
- Specs and plans written before this ADR keep naming `schemas/index.json`; they are records of
  their time and are not rewritten.

## Implementation (#335, after approval)

1. `schema_snapshots.py`: manifests, `schema_history()`, per-method diff, migration with its
   test (acceptance criteria 2 and 4).
2. Readers moved to `schema_history()`; documentation build and release publish the
   assembled file.
3. Reference and status tables built at documentation time; README themes; tool examples per
   method.
4. The two-branch merge test (acceptance criterion 3); `sprints/README.md` and `sprint.py`
   drop the merge order for `registry` (acceptance criterion 5).

Tier: substantial (schema history contract): one `code-reviewer` pass on the implementation.
The maintainer accepts this ADR before the implementation is planned (sprint S02).

## Open questions for the maintainer

- Decision 6 removes the per-method table from the README. The alternative keeps it and
  accepts a README conflict when two methods are added in parallel, resolved by regenerating
  after a rebase: the only place left where the merge order would apply.
