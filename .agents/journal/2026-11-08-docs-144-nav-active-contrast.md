# 2026-11-08 — docs/144-nav-active-contrast

- Issue: #144 Documentation site meets WCAG AA contrast for the active navigation item
- Pull request: opened at the end of this session
- Spec: none (documentation styling)

## Done
- fdd2279 Light theme: the active navigation item uses Deep Navy on a navy tint (11.4:1) with a
  gold inset marker; the dark theme keeps gold on navy (6.8:1). Task #144 recorded in the
  roadmap as done.

## State at the end of the session
- `uv run just check` passes locally.

## Decisions taken
- Scope limited to the active navigation item, as the issue states. Gold on the light
  background is also used by the theme for hover and focus states of links (2.4:1); not
  changed here, reported to the maintainer.
- Issues #142 and #143 were opened in the same session and run on their own branches.

## Open questions for the maintainer
- Should hover and focus states in the light theme stop using gold as well (brand.md: gold on
  navy only)?

## Next steps
1. Review the pull request and the rendered site in the light theme after merge.
