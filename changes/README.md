# Release notes fragments

Every pull request that changes what users can do or get adds one short file here. At release
time `towncrier` turns them into `RELEASE_NOTES.md` and the GitHub Release text.

Name: `<issue number>.<type>.md`, for example `55.feature.md`. Types:

| Type | Use for |
| --- | --- |
| `breaking` | Users must change something (inputs, names, behaviour) |
| `numerical` | The same input now gives a different result (always with `Numerical-change:`) |
| `feature` | Something new users can do |
| `improvement` | Something existing works better |
| `fix` | A defect is corrected |

Write for a financial analyst, not a developer: what changed for them and why it matters, in
one to three sentences, in English, without code jargon. The technical detail belongs in the
commit messages, which generate `CHANGELOG.md`.

Good: "Fairmark now works with Google Gemini and with open-weight models running on your own
computer through Ollama, so valuations can stay fully on premises."

Not useful: "Add gemini() adapter and portable schema to tools.py."

Changes with no effect for users (tests, CI, refactoring) need no fragment; label the PR
`no-release-note`.
