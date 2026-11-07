# Function calling

Fairmark generates tool definitions for any LLM that supports function calling. They are
built from the engine's JSON Schemas, so they always match the implementation.

| Provider | Definitions | Format |
| --- | --- | --- |
| Anthropic Messages API | `fm.tools.anthropic()` | `name`, `description`, `input_schema` |
| OpenAI-compatible Chat Completions | `fm.tools.openai()` | `{"type": "function", "function": {...}}` |
| Anything else | `fm.tools.definitions()` | `name`, `description`, `parameters` (JSON Schema) |

## Executing calls

```python
result = fm.tools.dispatch(name, arguments)   # arguments: dict or JSON string
```

`dispatch` never raises on bad input. It returns `{"error": {"code", "message"}}`, which you
send back to the model as the tool result so it can fix its parameters.

## Tool names

Method `dcf.fcff` becomes tool `fairmark_dcf_fcff`. Use `fm.tools.method_id(name)` to map back.

## Schemas the model sees

Schemas are inlined (no `$ref`) for compatibility across providers. Decimal fields accept
numbers or strings; descriptions state units explicitly ("decimal fraction, 0.09 = 9%"),
which is what makes models extract rates correctly.
