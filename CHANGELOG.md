# Changelog

All notable changes to Fairmark. Generated from Conventional Commits; do not edit by hand.
Numerical changes (a different result for the same input) are always listed first.

## [0.1.0-alpha.1] - 2026-11-24

### Numerical changes

- **tvm**: Time value foundations with dates, rates and solvers (#96). Method versions: tvm.npv v1 -> v2, dcf.fcff v1 -> v2, tvm.irr v1 -> v2
- **dcf**: Shared engine with stages for dcf.fcff v3 (#178). Method versions: dcf.fcff v2 -> v3
- **dcf**: Limit the tax shield to EBIT under zero [SPEC-0001 R3] (#204). Method versions: dcf.free_cash_flows v1 -> v2

### Added

- **brand**: Logo, palette and a README worthy of the project
- **i18n**: Bilingual documentation (English source, Italian edition) and language skills
- **tools**: Provider-agnostic tools, MCP over HTTP, structured logging, release tooling
- **coc**: Cost of equity, beta relevering, peer betas and WACC (#100)
- **analysis**: Goal seek on any method with Brent's method (#109)
- **analysis**: Sensitivity tables, scenarios and tornado (#129)
- **ratios**: Five ratio groups with not-defined reasons (#140)
- **tv**: Terminal value methods and their comparison (#141)
- **loans**: Instalment, instalment count and amortisation schedules (#156)
- **ratios**: Covenant tests on contractual definitions (#170)
- **registry**: Method status, available or preview, for every adapter (#179)
- **dcf**: Free cash flow build-up and build-up lines in the DCF (#180)
- **dcf**: Terminal value techniques inside the DCF (#187)
- **dcf**: Nominal and real terms in the DCF [SPEC-0001 R16] (#194)
- **dcf**: Bridge to equity value [SPEC-0001 R23, R25, R26] (#196)
- **dcf**: Equity DCF dcf.fcfe on the shared engine [SPEC-0001] (#205)
- **dcf**: Value from a vector of flows with defaults [SPEC-0001 R31] (#221)
- **dates**: Timeline and value series builders [SPEC-0010] (#242)
- **statements**: Reorganise the statutory balance sheet [SPEC-0005] (#247)
- **tools**: Enum meanings, examples, compact profile and areas (#274)
- **ratios**: Compute every ratio from partial input [SPEC-0007 R7] (#276)
- **schemas**: Output schemas and schema-driven Python decoding (#275)
- **statements**: Reorganise the income statement [SPEC-0005] (#277)
- **errors**: Structured error report with every input error (#279)
- **agents**: Worktree lifecycle and generated-artifacts merge policy (#325)
- **agents**: Persistent sprint state and per-task checkpoints (#326)
- **agents**: Lanes declare their paths and overlaps fail validation (#341)
- **agents**: One generating lane per group of generated artifacts (#349)
- **agents**: Index and search the session journal (#358)
- **agents**: Sprint event ledger and burn-down (#365)
- **scripts**: Pull request queue and test quarantine (#367)
- **explain**: One envelope with diagnostics and coded warnings (#361)
- **agents**: Generate agent roles for Codex, Cursor and Copilot (#380)

### Fixed

- **release**: Validate versions as whole strings (#155)
- **tvm**: Review follow-ups and promotion to available [SPEC-0002] (#188)
- **agents**: Protect golden cases in every git worktree (#195)
- **errors**: Name input error fields by JSON pointer [SPEC-0001 R27] (#202)
- **tvm**: Guess domain for IRR and XIRR, one key per tv.compare entry (#203)
- **cli**: Exit quietly when the output pipe is closed (#234)
- **dcf**: Report the net financial position default and warn (#273)
- **agents**: Share sprint runtime state across every worktree (#338)
- **ci**: Stop background git maintenance in the workspace report tests (#340)
- **tools**: One type per keyword in every tool schema (#344)
- **errors**: Error report order independent of input key order (#371)
- **ci**: Roadmap sync reports only real differences (#398)
- **ci**: Mutation summary passes when no shard ran (#417)
- **mcp**: Stop with the install command when the mcp extra is missing (#424)

### Changed

- **core**: Method trait per theme and versioned schema history (#104)
- **py**: Remove explain_markdown, JSON is the only output (#311)

### Documentation

- **specs**: Draft cost of capital, multi-stage DCF and Italian methods (#98)
- **specs**: Approve SPEC-0008 analysis methods (#107)
- **specs**: Approve terminal value and ratio specs (#102)
- **project**: Public pages state only what exists (#122)
- **project**: Current release notes and changelog on the site (#128)
- **agents**: Session journal for 2026-11-07 (#133)
- **specs**: Loans and amortisation schedules (#139)
- **brand**: Readable active navigation item in the light theme (#145)
- **project**: Undated draft notes and method versions in the changelog (#146)
- **adr**: Release notes translated with each fragment (#162)
- **specs**: Approve the complete DCF spec (#168)
- **project**: Disclaimer, decisions belong to the user (#172)
- **i18n**: Add a README status legend and Italian method titles (#186)
- **specs**: Approve SPEC-0010 timeline and value series (#235)
- **specs**: Approve SPEC-0005 statements reorganisation (#237)
- **adr**: Propose contract guidelines for inputs, envelope, errors (#270)
- **adr**: Add default admission tests and the floor to ADR 0014 (#272)
- **specs**: Draft SPEC-0011 cash flow statement [SPEC-0011] (#278)
- **project**: Review and audit effort proportional to risk (#305)
- **i18n**: Glossary entries for numerical audit and code review (#317)
- **project**: Rule for external analysis failures before merge (#322)
- **agents**: Session journal for 2026-11-14 (#324)
- **adr**: Agent memory architecture, ADR 0015 and agent loop planning (#330)
- **adr**: Accept ADR 0014 on contract guidelines (#337)
- **project**: One page on how Fairmark is built by agents (#355)
- **adr**: Constitution principle IX, unambiguous and transparent (#360)
- **adr**: Propose per-method schema manifests (ADR 0017) (#382)
- **adr**: Accept ADR 0017, per-method schema manifests (#383)
- **adr**: Propose the sources policy (ADR 0018) (#384)
- **adr**: Correct the inventory of ADR 0018 (#385)
- **adr**: Accept the sources policy (ADR 0018) (#389)
- **specs**: State the sources rule in constitution principle VII (#390)
- **project**: Freeze the agentic development process after S01 (#392)
- **docs**: Add a README status line and how the project is engineered (#416)
- **docs**: Hand over the product status of 2026-11-22 (#420)

### Build and CI

- **deps**: Replace unmaintained serde_yaml with serde_norway in golden runner
- **deps**: Bump the actions group with 8 updates (#105)
- **project**: Pin actions, audit workflows and check YAML (#130)
- **project**: Roadmap sync tolerates pending pull requests (#138)
- **project**: Issues closed by an open pull request are pending (#153)
- **release**: Prepare the release pull request from a workflow (#150)
- **ci**: Install cargo-deny with checksum and retries (#182)
- **release**: Wheel matrix and trusted publishing to PyPI and crates.io (#238)
- **release**: Pin build and smoke images by digest (#255)
- **deps**: Drop filelock 4.1.0 from the Python 3.10 dev tools (#294)
- **ci**: Mutation testing of the core with cargo-mutants (#290)
- **deps**: Bump astral-sh/setup-uv from 7.6.0 to 10.3.3 (#241)
- **deps**: Bump actions/add-to-project from 1.0.2 to 2.0.1 (#269)
- **py**: Raise the supported Python floor to 3.12
- **deps**: Bump the actions group with 2 updates (#312)
- **scripts**: Check that every task id is defined once (#316)
- **mcp**: Drive the MCP server over stdio with the MCP Inspector (#342)
- **ci**: App key in a protected environment (#375)
- **ci**: Put the AI workflow key behind an approval per run (#378)
- **deps**: Bump the actions group with 3 updates (#413)
- **ci**: Mutation testing completes in CI or fails explicitly (#414)
- **deps**: Bump clap from 4.6.7 to 4.6.8 in the cargo-minor group (#412)
- **ci**: Failures on main open issues, agents fix lint before stopping (#400)
- **release**: Build the macOS Apple silicon wheel on macos-15 (#422)
- **release**: Let Prepare release read issues for the readiness check (#427)

### Maintenance

- Agentic, spec-driven development setup and extended CI
- **scripts**: Make GitHub roadmap bootstrap resumable with retries
- **agents**: Canonical .agents/ folder, new roles, and a no-icons house style
- **project**: Script to create the Kanban GitHub Project and auto-add workflow
- **automation**: Documentation sync checks, generated changelog, sprints and orchestration
- **project**: Add session protocol, naming rules and dependency checks (#94)
- **project**: One commit scope per engine theme (#112)
- **project**: Roadmap as code, synchronised with GitHub (#118)
- **project**: A pull request marks the task it closes as done (#120)
- **agents**: Propose roadmap changes from requests (#124)
- **project**: Release readiness check and version policy (#126)
- **core**: Golden runner compares expected values exactly (#151)
- **project**: Commit scope for the loans theme (#158)
- **project**: Generate the roadmap page at build time (#160)
- **core**: Property tests for time value, rates and loans (#169)
- **agents**: Approvals cover only the golden cases they name (#177)
- **project**: Keep the required checks in step with the ruleset (#184)
- **project**: Roadmap update of 2026-11-08 (#218)
- **project**: Roadmap tasks approved on 2026-11-08 (#220)
- **ci**: Enforce explicit utf-8 encoding with ruff PLW1514 (#233)
- **project**: Add issue 243 on the loans precision wording (#244)
- **project**: Add issue 245 on intercompany financial payables (#246)
- **scripts**: Add the statements commit scope [SPEC-0005] (#248)
- **agents**: Add a computability table to specs and audits (#250)
- **project**: Add issue 251 on non-recurring exchange losses (#252)
- **project**: Add the cash flow statement and widen the playground (#254)
- **project**: Add contract hardening tasks and the ESMA view (#268)
- **agents**: Fix every review finding in the same pull request (#281)
- **project**: Plan mutation testing and its v1.0 release gate (#284)
- **project**: Plan the release readiness reviews for v0.1 (#289)
- **project**: Move the real estate module to v2.0 (#291)
- **project**: Plan the evaluation pack for v1.0 (#301)
- **project**: Plan the numerical edge cases found by the #298 audit (#303)
- **core**: Kill the missed mutants of the baseline (#298)
- **scripts**: Read-only workspace report and smaller debug builds (#299)
- **project**: Plan tables for notebooks and the cookbook (#308)
- **project**: One output format, drop the table and markdown outputs (#309)
- **project**: Roadmap update of 2026-11-14 (#321)
- **project**: Plan MCP protocol checks and parallel sprint tooling (#336)
- **project**: Plan the process work of M0 and drop #27 (#348)
- **scripts**: A test for every process script (#350)
- **project**: Plan contract stability tasks (#354)
- **project**: Sprint S01, process and product lanes (#356)
- **project**: Plan the agent framework cookbook after v1.0 (#364)
- **project**: Give the process lane the files of #320, second part (#366)
- **project**: Plan mutation testing that completes or fails explicitly (#369)
- **project**: Plan #372, secrets in protected environments (#373)
- **project**: Add project-sync to the process lane (#374)
- **project**: Plan #376, AI workflow key behind an approval (#377)
- **project**: Add the agent mirror folders to the process lane (#379)
- **scripts**: Let every sprint lane add its navigation entries (#381)
- **project**: Declare every label the repository applies (#386)
- **project**: Remove the project board, milestones only (#393)
- **core**: Golden cases assert values, schemas assert shape (#394)
- **ci**: Remove the Claude workflows that never ran (#396)
- **project**: Plan the maintainability audit and two open items (#411)

