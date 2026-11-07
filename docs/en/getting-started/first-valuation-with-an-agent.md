# Your first valuation with an agent

In this tutorial an LLM reads a short business-plan summary, extracts the inputs, calls
Fairmark and writes a valuation note. The model never computes a number itself.

## 1. Install

```bash
pip install fairmark anthropic
export ANTHROPIC_API_KEY=...
```

## 2. Give the model Fairmark's tools

`fm.tools.anthropic()` returns tool definitions generated from the engine's schemas.
Each tool description tells the model to use the tool instead of doing arithmetic.

```python
import json

import anthropic
import fairmark as fm

client = anthropic.Anthropic()
messages = [{
    "role": "user",
    "content": (
        "Our business plan projects free cash flows to the firm of 1.2m, 1.35m, 1.5m, 1.6m "
        "and 1.7m euros. WACC is 9.5%, long-term growth 2%, cash flows arrive evenly during "
        "the year, net debt is 3.1m. Value the equity and explain the calculation."
    ),
}]
```

## 3. Run the loop

```python
while True:
    response = client.messages.create(
        model="claude-opus-5-5", max_tokens=4096,
        tools=fm.tools.anthropic(), messages=messages,
    )
    messages.append({"role": "assistant", "content": response.content})
    calls = [b for b in response.content if b.type == "tool_use"]
    if not calls:
        break
    messages.append({"role": "user", "content": [
        {"type": "tool_result", "tool_use_id": c.id,
         "content": json.dumps(fm.tools.dispatch(c.name, c.input))}
        for c in calls
    ]})

print(next(b.text for b in response.content if b.type == "text"))
```

## What happened

1. The model mapped "9.5%" to `"0.095"` and "evenly during the year" to `mid_period`.
2. `fm.tools.dispatch` ran `dcf.fcff` in the engine and returned the value, the steps and
   the input hash.
3. The model wrote its note from that result.

If the model sends an invalid input, `dispatch` returns an error such as
`{"error": {"code": "FM-E004", ...}}` instead of raising, so the model can correct itself.

!!! tip "Review the inputs, not the arithmetic"
    With Fairmark the arithmetic is guaranteed. What deserves your review is the input the
    model extracted: it is always echoed in the result and fingerprinted in `meta.input_hash`.
