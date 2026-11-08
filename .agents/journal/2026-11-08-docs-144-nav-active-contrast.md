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
- First commit limited to the active navigation item. The maintainer then asked to fix the
  other uses of gold on white as well (hover and focus of links and controls, 2.4:1): the
  light theme accent is now Deep Navy (12.8:1), links underline on hover and focus, and the
  active item keeps a gold marker (acceptance criterion 3 of the issue).
- Issues #142 and #143 were opened in the same session and run on their own branches.

## Open questions for the maintainer
- None.

## Next steps
1. Review the pull request and the rendered site in the light theme after merge.
