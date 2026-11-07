## What and why

## Checklist
- [ ] Release note fragment in `changes/` written for users (or label `no-release-note`)
- [ ] Golden cases added or updated, with their source
- [ ] Methodology page in `docs/en/methods/` updated (if a method changed)
- [ ] No change to existing golden results — or `numerical-change` label, `method_version` bump and `Numerical-change:` commit trailer
- [ ] `cargo fmt`, `cargo clippy -D warnings`, `cargo test`, `pytest`, `ruff`, `mypy` pass
- [ ] Commits signed off (DCO)
