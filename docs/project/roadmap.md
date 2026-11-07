# Roadmap

The roadmap lives in [`roadmap/roadmap.yaml`](https://github.com/salvo8519/fairmark/blob/main/roadmap/roadmap.yaml)
and is mirrored on GitHub as milestones, epics and sub-issues. Tasks marked
`parallel` can be picked up independently; `good first issue` is a good place to start.

| Milestone | Goal | Gate |
| --- | --- | --- |
| M0 · Foundations | Repository, CI, wheels for 3 OS | A test package installs on Linux, macOS and Windows |
| M1 · Numerical core | Decimal TVM, solvers, explain, golden harness | NPV and IRR validated; property tests running |
| M2 · Complete DCF | Financial statements → FCFF, WACC/CAPM, terminal value options | DCF passes practitioners' golden cases |
| M3 · Agent layer and v0.1 | Python, CLI, MCP, function calling, skills, docs, n8n node | An external user values a company with an agent using only the docs |
| M4 · European methods | Asset-based, income-based, excess earnings, EVA, multiples | Golden cases green for every method |
| M5 · Opt-in statistics | Public RFC, privacy-first client and collector | CI proves no financial data in the payload |
| M6 · v1.0 and real estate | Real estate module, scenarios, WASM playground, hosted MCP | Stable API with deprecation policy |
