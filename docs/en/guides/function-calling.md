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
