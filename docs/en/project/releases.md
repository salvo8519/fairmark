# Releases and versions

Fairmark has one version, `[workspace.package] version` in `Cargo.toml`, shared by the Rust
crates and the Python package. Versions follow Semantic Versioning.

## Policy

| Version | When |
| --- | --- |
| `0.1.0-alpha.N`, `-beta.N`, `-rc.N` | Whenever there is something worth trying; open work is reported, not required |
| `0.1.0` | Milestones M0 to M3 complete: the first public release (gate of M3) |
| `0.2.0`, `0.3.0` | M4 and M5 complete |
| `1.0.0` | M6 complete: stable API with a published deprecation policy |
| Patch `X.Y.Z+1` | Fixes between minor releases |

Each milestone in `roadmap/roadmap.yaml` names the release that ships it, and the
[roadmap](roadmap.md) shows it. Before 1.0.0 a minor version may change an input contract; every
such change is a new method version with its own schema file, so earlier contracts stay
documented.

## Method status

Every method has one of two statuses. The engine registry holds it (ADR 0012) and reports it
to every adapter: `fm.methods()` and `fairmark methods` return a `status` field, the MCP server
puts it in each tool's `_meta` (`fairmark/status`), and the description of every MCP and
function-calling tool of a preview method says "Status: preview". The methods index, the
methods reference and the README tables are generated from the same field.

| Status | Meaning |
| --- | --- |
| `available` | The method meets the promotion rule below. |
| `preview` | The method is implemented, tested against golden cases and documented, but has not met the promotion rule. Results are deterministic and traced in `explain`, but they have not been independently verified: agents tell their users when a result comes from a preview method. |

Both statuses follow the same versioning rules (ADR 0009): any numerical or breaking input
change is a new method version with its own schema. A new method is preview until it is
promoted.

### Promotion rule

A method may be available only when all of the following hold:

1. its spec (`specs/NNNN-*/spec.md`) is approved: the Status row starts with `Approved` or
   `Implementing`;
2. the spec has no unchecked item (`- [ ]`, with any bullet: `-`, `*`, `+` or a number;
   fenced code blocks are ignored): every open question is resolved and every acceptance
   criterion with a checkbox is ticked;
3. the spec names the method id;
4. the method has golden cases, and every one names its independent source (`source`);
5. the independent numerical audit is recorded as a ticked task in the spec's `tasks.md`
   whose text names the role literally, `numerical-auditor`;
6. the code review is recorded as a ticked task in the spec's `tasks.md` whose text names the
   role literally, `code-reviewer`.

### Promotion, checked

- `specs/method-evidence.yaml` lists, for each spec (a quoted four-digit key such as
  `"0002"`), the methods it covers and the ids (`T<n>`) of the `tasks.md` tasks that record
  the audit and the review. Each id must be defined exactly once. The checker rejects an
  unknown key or a malformed value, so a typo cannot pass as missing evidence.
- Promotion is one line in the `AVAILABLE` list of `crates/fairmark-core/src/registry.rs`;
  the maintainer approves it in the pull request.
- `python scripts/method_status.py check` (CI and `just check`) fails when an available method
  does not meet every condition. `python scripts/method_status.py report` prints the evidence
  of every method and the preview methods that are eligible for promotion.
- Methods that were available before this rule was introduced and do not yet meet it are
  listed under `legacy:` in the evidence file and reported on every run. The list may only
  shrink: it must stay within the set frozen in the checker (`FROZEN_AT_149`), and the check
  fails when one of them meets the rule (remove it from the list) or becomes preview. The list
  has been empty since the maintainer decision of 2026-11-08, and `release.py ready` fails
  while it is not.
- Results carry the status too: `meta.method_status` in every result, and
  `meta.evaluated_method_status` in the result of an analysis method, which also states
  "Evaluated method <id> is preview" in `explain.conventions` when the method it evaluates is
  preview.
- The evidence file, the checker and the specs' `tasks.md` files are owned by the maintainer
  (`.github/CODEOWNERS`).
- A defect found in an available method is fixed in a new method version; the maintainer may
  also move the method back to preview by removing its line.

## Readiness, checked

`python scripts/release.py ready <version>` decides whether a version can be released:

- the version is valid and newer than the last tag;
- every milestone whose release is not later than the version is complete in the roadmap (a
  pre-release lists the open tasks instead);
- there are release notes to publish in `changes/`;
- the roadmap, the translations, the schema snapshots and the version files are current;
- every available method meets the promotion rule and none is available on legacy grounds
  (see [Method status](#method-status));
- `main` is green, no open pull request carries `numerical-change`, and the roadmap agrees with
  GitHub.

## What users read between releases

`RELEASE_NOTES.md` and `CHANGELOG.md` are written in the repository at release time. The
documentation site renders both at every build, with the changes not yet released on top: the
release notes from the fragments in `changes/` (towncrier, draft), the changelog from the
commits since the last tag (git-cliff). Every user-facing pull request adds its fragment (CI
checks it), so the site is current after each merge.

## Steps

1. The maintainer decides to release and starts the release preparation, in one of two ways:
   - on GitHub, the **Prepare release** workflow (Actions, Run workflow, on `main`), with the
     version and, optionally, the issue of the release task;
   - locally, an agent runs the `release` skill.

   Both start with `release.py ready` and stop if it fails; the workflow writes the report in
   the job summary.
2. Both open a `chore(release)` pull request from the branch `release/vX.Y.Z`: version bump,
   release notes for analysts (`RELEASE_NOTES.md`, from `changes/`) and the technical
   changelog (`CHANGELOG.md`). The maintainer reviews the release notes in it: they are the
   text of the GitHub Release.
3. The maintainer merges it and creates the tag `vX.Y.Z`, or asks an agent to. The workflow
   never tags and never merges.
4. The Release workflow checks the version, builds the wheels for Linux, macOS and Windows,
   creates the GitHub Release with the release notes and publishes to PyPI and crates.io.

### Setting up the Prepare release workflow

- In the repository settings (Actions, General, Workflow permissions), allow GitHub Actions
  to create pull requests.
- A pull request opened with the default workflow token does not start the pull request
  workflows; its body says so, and closing and reopening it starts them. To have the checks
  start on their own, create a GitHub App with contents and pull requests write access,
  install it on the repository, and store its id in the variable `RELEASE_APP_ID` and its
  private key in the secret `RELEASE_APP_PRIVATE_KEY`. The workflow then uses a token limited
  to those two permissions.

The documentation site is republished at every merge on `main`; the **Docs** workflow can also
be run by hand on `main` to republish it without a commit.
