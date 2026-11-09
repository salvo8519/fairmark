# Function calling

Fairmark generates tool definitions for any LLM that supports function calling. They are
built from the engine's JSON Schemas, so they always match the implementation.

| Provider | Definitions | Format |
| --- | --- | --- |
| Anthropic Messages API | `fm.tools.anthropic()` | `name`, `description`, `input_schema` |
| OpenAI and OpenAI-compatible Chat Completions | `fm.tools.openai(portable=False)` | `{"type": "function", "function": {...}}` |
| OpenAI Responses API | `fm.tools.openai_responses()` | `{"type": "function", "name": ..., "parameters": ...}` |
| Google Gemini | `fm.tools.gemini()` | `name`, `description`, `parametersJsonSchema` |
| By provider name | `fm.tools.for_provider("ollama")` | the right format for that provider |
| Anything else | `fm.tools.definitions(portable=False)` | `name`, `description`, `parameters` (JSON Schema) |

Every function above also takes two keyword options, both off by default:
`compact=True` (smaller schemas, see [Compact profile](#compact-profile)) and
`areas=[...]` (only some tools, see [Tool subsets by area](#tool-subsets-by-area)).

Supported providers and complete examples: [LLM providers](providers.md).

## Executing calls

```python
result = fm.tools.dispatch(name, arguments)   # arguments: dict or JSON string
```

`dispatch` never raises on bad input. It returns `{"error": {"code", "message"}}`, which you
send back to the model as the tool result so it can fix its parameters.

## Preview methods

Every method has a status, `available` or `preview`, reported by `fm.methods()`
(see [method status](../project/releases.md#method-status)). The definition of a preview
method has "(preview)" after its title and the sentence `fm.tools.PREVIEW_NOTE` ("Status:
preview. ...") in its description, so the model knows before it calls the tool and can tell
the user. The definitions carry no extra key, so every provider accepts them.

```python
[m["id"] for m in fm.methods() if m["status"] == "preview"]
```

## Tool names

Method `dcf.fcff` becomes tool `fairmark_dcf_fcff`. Use `fm.tools.method_id(name)` to map back.

## Schemas the model sees

Schemas are inlined (no `$ref`) for compatibility across providers. Decimal fields accept
numbers or strings; descriptions state units explicitly ("decimal fraction, 0.09 = 9%"),
which is what makes models extract rates correctly. The portable variant (`portable=True`,
used for Gemini and local open-weight runtimes) declares decimals as strings, turns optional
values (`X or null`) into plain `X` (an optional field is simply not required), and removes
`pattern` and `additionalProperties`. Dates are ISO 8601 strings (`"2026-06-30"`) in both
variants.

## Enum values and their meaning

Many fields take one of a fixed set of values, each with a meaning: the statutory codes of
`statements.reorganise` (`A1`, `B6`, `B7`, ...), `period_0` and `period_1` in
`series.project`, the amortisation types of `loans.amortisation_schedule`. The engine
documents each value separately; the tool definition turns the list into a plain `enum` and
adds the meaning of every value to the description of the field, so nothing is lost:

```text
`french` (constant instalment), `fixed_amortisation_schedule` (constant principal portion)
or `bullet` (principal repaid with the last instalment). Values: `french` (Constant
instalment (French amortisation, "ammortamento alla francese")); `fixed_amortisation_schedule`
(Constant principal portion ("ammortamento all'italiana")); `bullet` (Interest only; the full
principal is repaid with the last instalment).
```

## Examples

Every definition carries input examples, taken from the golden cases of the method:

- the minimal example: the smallest input that produces a result;
- where useful, a maximal example: an input that shows most of the optional fields.

In the default definitions they are in the JSON Schema `examples` keyword of the parameters
(`[minimal, maximal]`). In the portable variant, where some providers reject keywords they do
not know, they are written in the description instead ("Minimal input example: {...}").
`fm.tools.examples("dcf.fcff")` returns them as data.

Which golden case provides each example is set in one table, `TABLE` in
`scripts/tool_examples.py`; `python scripts/tool_examples.py` copies the inputs into
`python/fairmark/tool_examples.json` (generated, never edited by hand, part of
`just generate`). The test suite checks that every example validates against the method's
schema and runs through the engine without error.

## Compact profile

`compact=True` produces smaller definitions for clients with a limited context. It accepts
exactly the same inputs and keeps the meaning of every enum value; it removes only repetition:

- a description of 40 characters or more, or a list of enum values, already given earlier in
  the same tool is given once;
- `"default": null` is removed (absent and null mean the same);
- the decimal rule repeated on every amount (the `pattern`, or the portable hint) is stated
  once in the tool description, where that is shorter;
- non-standard `format` values (`uint32`) are removed; `minimum` still applies;
- only the minimal example is kept.

The saving is largest for the largest methods (about a quarter for `statements.reorganise`,
a sixth for `dcf.fcff`); small methods change little. Most of a large schema is documentation
that is not repeated, and the compact profile keeps it.

## Tool subsets by area

A client that needs only some methods can ask for the tools of some areas. The area is the
part of the method id before the dot:

```python
fm.tools.areas()                                   # ["analysis", "coc", "dates", "dcf", ...]
fm.tools.definitions(areas=["dcf", "tvm"])         # only dcf.* and tvm.* tools
fm.tools.for_provider("ollama", compact=True, areas=["ratios"])
```

An unknown area raises `ValueError` with the list of known areas. `dispatch` still executes
every tool: a subset reduces what the model sees, not what Fairmark can run.

