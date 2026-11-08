# Claude Code, Codex and other coding agents

Coding agents work best with a CLI and a skill that tells them how to use it.

## Install the skill

Copy `skills/fairmark/` from the repository into your agent's skills directory (for Claude
Code: `.agents/skills/fairmark/` in your project, or `~/.agents/skills/fairmark/`).

The skill instructs the agent to:

1. discover methods with `fairmark methods` (each with its status, `available` or `preview`)
   and inputs with `fairmark schema <method>`, and say so when a result comes from a preview
   method;
2. write the extracted inputs to a JSON file and run `fairmark run`;
3. report values exactly as returned, citing `explain.steps` and the conventions;
4. never compute valuation figures itself.

## Without the skill

The CLI is self-describing, so an agent can usually figure it out from `fairmark --help`.
The skill makes the behaviour consistent and adds the reporting rules.
