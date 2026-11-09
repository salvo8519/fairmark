# Releases and versions

Fairmark has one version, `[workspace.package] version` in `Cargo.toml`, shared by the Rust
crates and the Python package. Versions follow Semantic Versioning.

## Policy

| Version | When |
| --- | --- |
| `0.1.0-alpha.N`, `-beta.N`, `-rc.N` | Whenever there is something worth trying; open work is reported, not required |
| `0.1.0` | Milestones M0 to M3 complete: the first public release (gate of M3) |
| `0.2.0`, `0.3.0` | M4 and M5 complete |
| `1.0.0` | M6 complete: corporate valuation with a stable API and a published deprecation policy |
| `2.0.0` | M7 complete: the real estate module, on the same contract |
| Patch `X.Y.Z+1` | Fixes between minor releases |

Each milestone in `roadmap/roadmap.yaml` names the release that ships it, and the
[roadmap](roadmap.md) shows it. Before 1.0.0 a minor version may change an input or output
contract; every such change is a new method version with its own schema files, so earlier
contracts stay documented.

## Method versions

Each method has its own version, a positive integer independent of the release version
(ADR 0009). It is bumped, and never reused, on:

- a **numerical change**: the same input gives a different value;
- a **breaking input change**: a field removed or renamed, a new required field, a changed
  type or constraint, a value removed from an enumeration;
- a **breaking output change** (ADR 0014 G8): an output field removed, renamed or retyped, a
  field that becomes nullable, or a field that was always present and may now be absent.

A new optional input field, a new output field and a value added to an enumeration, of an
input or of an output, are additive: they keep the method version; removing a value from an
enumeration is breaking. Clients therefore accept output enumeration values they do not know
(maintainer decision of 2026-11-09, recorded in ADR 0014 G8). Every version keeps two schema snapshots,
`schemas/<method>/v<N>.json` (input) and `schemas/<method>/v<N>.output.json` (output, the
`value` of the result), recorded with their hashes in `schemas/index.json`. Snapshots of
earlier versions are never rewritten; `scripts/schema_snapshots.py` enforces both rules in CI,
for inputs and outputs alike. Versions released before output snapshots existed have only the
input snapshot.

The method version is reported everywhere a client can read it: `meta.method_version` of every
result (`Result.method_version` in Python), the `version` field of `fm.methods()` and
`fairmark methods`, and `fairmark/method_version` in the `_meta` of each MCP tool, beside
`fairmark/status`. The envelope around `value` (`explain`, `meta`) follows its own version, not
the method version (ADR 0014 G4). A client-side version pin, which would refuse to run a method
whose version differs from the one the client expects, is not decided yet (ADR 0014, "Not
decided here").

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

Both statuses follow the same versioning rules ([method versions](#method-versions)): any
numerical change, or breaking input or output change, is a new method version with its own
schemas. A new method is preview until it is
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
3. The maintainer merges it and creates the tag `vX.Y.Z` on the merge commit, with the
   maintainer's own credentials. Agents and workflows never create, move or delete a release
   tag, and never merge the release pull request.
4. The tag starts the **Release** workflow (see [The Release workflow](#the-release-workflow)).
   It builds and tests every package, then waits for the maintainer's approval of the `pypi`
   environment, attests the provenance of the files and publishes to PyPI, waits for the
   approval of the `crates-io` environment, publishes to crates.io and creates the GitHub
   Release with the release notes. Before approving each environment, the maintainer checks
   on the run page that the commit of the run is the merge commit of the release pull request
   on `main`.
5. The maintainer checks the result: the workflow is green, PyPI lists one wheel per platform
   and the sdist, crates.io lists `fairmark-core` and `fairmark-cli`, and
   `pip install fairmark==<version>` works in a new environment. The provenance of a
   downloaded file is checked with:

   ```bash
   gh attestation verify <file> --repo salvo8519/fairmark \
     --signer-workflow salvo8519/fairmark/.github/workflows/release.yml \
     --source-ref refs/tags/vX.Y.Z \
     --deny-self-hosted-runners
   ```

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

## The Release workflow

`.github/workflows/release.yml` runs on a tag `v*` (the release) and by hand with Run workflow
(the dry run). The tag is `v` followed by the version in `Cargo.toml`; the first job fails when
they differ (`scripts/version.py check`) or when the tagged commit is not on `main`. That last
check only guards against mistakes, such as a tag on the wrong commit: it is not a security
control. The controls are the protected tags and the maintainer's check of the run's commit
before approving each environment.

### What is built

The Python package is an `abi3` wheel: one wheel per platform serves Python 3.10 and every
later version. Each wheel is built on a runner of its own architecture.

| Platform | Wheel tag | Runner |
| --- | --- | --- |
| Linux x86_64, glibc | `manylinux_2_17_x86_64` (manylinux2014) | `ubuntu-latest` |
| Linux arm64, glibc | `manylinux_2_17_aarch64` (manylinux2014) | `ubuntu-24.04-arm` |
| Linux x86_64, musl (Alpine) | `musllinux_1_2_x86_64` | `ubuntu-latest` |
| Linux arm64, musl (Alpine) | `musllinux_1_2_aarch64` | `ubuntu-24.04-arm` |
| macOS Intel | `macosx_*_x86_64` | `macos-15-intel` |
| macOS Apple silicon | `macosx_*_arm64` | `macos-14` |
| Windows x86_64 | `win_amd64` | `windows-latest` |

The source distribution (sdist) serves every other platform; installing it needs a Rust
toolchain.

The Linux wheels are built in containers, and the musllinux wheels are tested in containers.
Each image is pinned by digest, not by tag, so that a tag moved upstream cannot change what
builds or tests a release. The pinned reference is the `FROM` line of one file per image in
`.github/release/images/`; the workflow reads it with `.github/release/pinned-image.sh`, which
fails when the line is not pinned by digest. Dependabot updates the digests weekly
([dependency policy](dependencies.md#container-images-are-pinned-to-digests)).

| Image | Used for | Pinned |
| --- | --- | --- |
| `quay.io/pypa/manylinux2014_x86_64:latest` | Build, Linux x86_64 glibc | Manifest of `linux/amd64` |
| `quay.io/pypa/manylinux2014_aarch64:latest` | Build, Linux arm64 glibc | Manifest of `linux/arm64` |
| `ghcr.io/rust-cross/rust-musl-cross:x86_64-musl` | Build, Linux x86_64 musl | Multi-platform index |
| `ghcr.io/rust-cross/rust-musl-cross:aarch64-musl` | Build, Linux arm64 musl | Multi-platform index, includes `linux/arm64` |
| `python:3.10-alpine`, `python:3.13-alpine` | Smoke test of the musllinux wheels | Multi-platform index |

On crates.io the workflow publishes `fairmark-core` (the engine) and `fairmark-cli` (the
`fairmark` binary), in that order, with `cargo publish --workspace`. `fairmark-py` has
`publish = false`: it is the Python extension and reaches users only inside the wheels.

### What is checked before anything is published

- **Smoke test of every wheel.** Each wheel is uploaded as soon as it is built, before any
  test code runs. Then, on its own runner, it is installed in a new environment with Python
  3.10 and Python 3.13, the oldest and newest versions that CI tests, and
  `scripts/smoke_wheel.py` runs every golden case through it. The script also checks that the
  package is imported from the environment, not from the checkout, and that its version is the
  one in `Cargo.toml`. musllinux wheels are tested on Alpine Linux in a container that sees the
  files read-only. PyYAML, the only other package installed, comes from
  `.github/release/smoke-requirements.txt` with `--require-hashes`.
- **The sdist** is built from source in a new environment and passes the same smoke test. Its
  build backend, maturin, comes from `.github/release/build-requirements.txt` with
  `--require-hashes`.
- **maturin version**: the packages are built with the maturin locked in `uv.lock`; the first
  job fails when the workflow or the requirements file names another version.
- **Completeness**: exactly one wheel per platform above and one sdist, all of this version,
  and `twine check --strict` on their metadata. This job writes `SHA256SUMS` and uploads the
  files with it as one artifact, `release-dist`; every publishing job downloads that artifact
  and runs `sha256sum -c SHA256SUMS` first.
- **Crates**: `cargo publish --dry-run --workspace` packages both crates and builds them as
  crates.io would.

### What is published, in order

1. **Build provenance and PyPI**, in one job in the `pypi` environment: a signed SLSA
   provenance attestation of every wheel and of the sdist, stored in the repository (artifact
   attestations), then the upload with trusted publishing. PyPI accepts a short-lived token
   issued for this workflow and the `pypi` environment; no PyPI token is stored in the
   repository. The upload also carries a PEP 740 attestation of every file.
2. **crates.io**, with trusted publishing in the same way, in the `crates-io` environment.
   crates.io cannot withdraw a version: this step runs only after everything above. The
   crates are packaged and verified with `cargo publish --dry-run` without any token; the
   upload then runs with `--no-verify`, so no build runs while a token is available.
3. **The GitHub Release**, created with `gh release create --verify-tag`, with the release
   notes, every file and `SHA256SUMS`; a tag with a hyphen (`-alpha.N`, `-beta.N`, `-rc.N`)
   makes a pre-release.

Every job that can request an OIDC token runs in an environment that requires the
maintainer's approval. Permissions, job by job (the workflow default is `contents: read`):

| Job | Permissions |
| --- | --- |
| Publish to PyPI | `id-token: write` (Sigstore signature and PyPI token), `attestations: write`, `artifact-metadata: write` (attestation and its record of the attested files) |
| Publish to crates.io | `id-token: write` (crates.io token) |
| GitHub Release | `contents: write` |
| Every other job | `contents: read` |

Builds use no cache.

### Dry run

Actions, **Release**, Run workflow, on `main` or on a branch: the same builds, smoke tests,
completeness check and `cargo publish --dry-run`, and nothing else. The publishing jobs run
only when the event is a tag push, so a manual run skips them even when it is started from a
tag; it uses no environment and asks for no approval. Run it before the first release and after
any change to the build.

### If a publishing job fails

Re-run the failed jobs from the run page. PyPI refuses a file that it already has and
crates.io refuses a version that it already has: when a job failed after part of its upload,
check what was published before re-running. A version is never published twice; a defect in a
published version is fixed by the next version.

On the first release, if `fairmark-core` reached crates.io and `fairmark-cli` did not, a re-run
fails because the bootstrap token is no longer accepted once `fairmark-core` exists. The
maintainer then publishes `fairmark-cli` from the tag with `cargo publish -p fairmark-cli` and
the same scoped token, and continues with step 5 of the setup below.

### One-time setup for publishing

These steps are done once, by the maintainer, before the first release. The workflow needs no
other secret.

1. **GitHub environments.** In the repository settings, Environments, create `pypi` and
   `crates-io`. For each:
   - Required reviewers: the maintainer (each release then waits for an approval);
   - "Allow administrators to bypass configured protection rules": off;
   - Deployment branches and tags: Selected branches and tags, with the tag rule `v*`.
2. **Protected tags (required).** In the repository settings, Rules, Rulesets, add a tag
   ruleset, Active, targeting the tags `v*`, with the rules "Restrict creations", "Restrict
   updates" and "Restrict deletions". In the bypass list add only the Repository admin role;
   no GitHub App, team or other role. The environments trust a tag `v*`: this ruleset decides
   who can create one.
3. **PyPI.** With two-factor authentication on the PyPI account, open Your account,
   Publishing, and add a pending publisher of type GitHub:
   - PyPI project name: `fairmark`;
   - Owner: `salvo8519`; Repository name: `fairmark`;
   - Workflow name: `release.yml`; Environment name: `pypi`.

   A pending publisher does not reserve the name: if someone else registers `fairmark` first,
   it is invalidated. The first release creates the project and turns the pending publisher
   into a normal one.
4. **crates.io, first release.** The crates.io account needs a verified email address to
   publish (Account Settings). crates.io accepts trusted publishing only for a crate that
   already exists, so the first upload uses an API token:
   - on crates.io, Account Settings, API Tokens, create a token with the scopes `publish-new`
     and `publish-update`, limited to the crates `fairmark-core` and `fairmark-cli`, with the
     shortest expiry that covers the release (for example 7 days);
   - store it as the secret `CARGO_REGISTRY_TOKEN` of the `crates-io` environment
     (Environments, `crates-io`, Environment secrets), never as a repository secret.

   The job accepts the secret only while `fairmark-core` does not exist on crates.io: it asks
   the crates.io API first. Once the crate exists, a remaining secret makes the job fail with
   "delete CARGO_REGISTRY_TOKEN and use trusted publishing"; without the secret, the job uses
   trusted publishing.
5. **crates.io, after the first release.** For each crate, `fairmark-core` and
   `fairmark-cli`: Settings, Trusted Publishing, Add, GitHub, with owner `salvo8519`,
   repository `fairmark`, workflow filename `release.yml` and environment `crates-io`. Then
   delete the `CARGO_REGISTRY_TOKEN` environment secret and revoke the token on crates.io.
6. **Dry run.** Run the Release workflow by hand on `main` and check that it is green.
