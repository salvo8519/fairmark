"""LLM extracts the parameters, Fairmark computes, the LLM writes the report.

pip install fairmark anthropic && export ANTHROPIC_API_KEY=...
"""

import json

import anthropic

import fairmark as fm

client = anthropic.Anthropic()
MODEL = "claude-opus-5-5"

messages = [
    {
        "role": "user",
        "content": (
            "Our business plan projects free cash flows to the firm of 1.2m, 1.35m, 1.5m, 1.6m "
            "and 1.7m euros. WACC is 9.5%, long-term growth 2%, cash flows arrive evenly during "
            "the year, net debt is 3.1m. Value the equity and explain the calculation."
        ),
    }
]

while True:
    response = client.messages.create(
        model=MODEL, max_tokens=4096, tools=fm.tools.anthropic(), messages=messages
    )
    messages.append({"role": "assistant", "content": response.content})
    tool_uses = [b for b in response.content if b.type == "tool_use"]
    if not tool_uses:
        break
    messages.append(
        {
            "role": "user",
            "content": [
                {
                    "type": "tool_result",
                    "tool_use_id": b.id,
                    "content": json.dumps(fm.tools.dispatch(b.name, b.input)),
                }
                for b in tool_uses
            ],
        }
    )

print(next(b.text for b in response.content if b.type == "text"))
