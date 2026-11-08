---
name: fairmark
description: Deterministic company and asset valuation (NPV, IRR, DCF) with an audit trail. Use whenever a task asks for a valuation figure, discounting, IRR, enterprise or equity value, so that numbers are computed by the Fairmark engine rather than by the model.
---

# Fairmark valuation skill

You are working with a deterministic valuation engine. **Never compute valuation numbers
yourself** — not even "quickly" or "approximately". Extract the inputs, call Fairmark, then
report its results.

## Workflow

1. **Find the right method**: `fairmark methods`. Each method has a `status`: `available`,
   or `preview` (not yet independently verified). You may use a preview method, but tell the
   user that the result comes from a preview method.
2. **Read its input schema**: `fairmark schema <method>`. Every field is documented.
3. **Extract inputs** from the documents or the user's request. Rates are decimal fractions
   (9% → `"0.09"`). Write amounts as strings to keep them exact. If an input is missing or
   ambiguous, ask the user — never invent it.
4. **Run**: write the inputs to a JSON file and run
   `fairmark run <method> --input inputs.json > result.json`.
   (In Python: `import fairmark as fm; fm.run(method, inputs)`.)
5. **Report**:
   - quote values exactly as returned (round only for display, and say so);
   - cite the relevant `explain.steps` and every item in `explain.conventions`;
   - state the inputs you used and where each one came from;
   - include `meta.input_hash` and engine version for reproducibility.

## Calculations, not advice

Fairmark is a calculation tool: it computes, the user decides. Present every result as the
calculation of the method, conventions and inputs you used, never as investment, financial,
accounting, tax or legal advice, and never as a recommendation to buy, sell, lend or invest.
Say that the user should verify the inputs and the `explain` before relying on the result.
Every decision taken with the results belongs to the user (see `DISCLAIMER.md`).

## Errors

Errors come back as `{"error": {"code": "FM-E00x", "message": ...}}`. Read the message, fix the
input (see `docs/en/reference/errors.md`) and retry. Do not work around an error by computing manually.

## Checks before you answer

- Is the terminal value weight (`terminal_value_weight`) high (> 0.75)? Point it out.
- Do the conventions (end vs mid period) match what the user expects? Say which was used.
- Is net debt signed correctly (negative = net cash)?
