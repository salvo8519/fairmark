# Usage statistics — RFC (draft, not implemented)

Status: **proposal under public discussion.** No telemetry exists in Fairmark today.

## Principles

1. **Opt-in.** Nothing is sent unless the user explicitly enables it. Non-interactive
   environments (CI, servers, agents) default to off.
2. **Never financial data.** No input or output values, company names, VAT numbers, file names,
   paths, prompts or any personal identifier. A CI test fails if the payload schema gains any
   field outside the allow-list below.
3. **Adapters only.** The Rust core has no network dependencies, enforced by `cargo deny`.
4. **Transparent.** `fairmark telemetry show` prints the exact payload; the collector is open
   source; aggregated statistics are published on a public dashboard.
5. **Easy off.** `FAIRMARK_TELEMETRY=0` and the standard `DO_NOT_TRACK=1` are always honoured.

## Allow-list (proposed)

| Field | Example |
| --- | --- |
| Method id | `dcf.fcff` |
| Engine and adapter version | `0.1.0`, `python` / `cli` / `mcp` |
| OS, architecture, Python version | `linux`, `x86_64`, `3.12` |
| Outcome | `ok` or an error code such as `FM-E004` |
| Duration bucket | `<1ms`, `1–10ms`, ... |
| Random installation id | resettable UUID, not linked to any person |

IP addresses are dropped at ingestion and never stored.

## Open questions

- Legal basis and notice under GDPR for the installation id (to be reviewed by counsel).
- Retention period of raw events before aggregation.

Comment in GitHub Discussions before any implementation starts.
