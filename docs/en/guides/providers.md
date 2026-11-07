# LLM providers

Fairmark is provider-agnostic. The engine never calls a model: it exposes tools, and any model
that supports tool use can call them. One function returns the definitions in the format each
provider expects; one function executes the calls, whatever the provider.

```python
import fairmark as fm

tools = fm.tools.for_provider("ollama")       # definitions for the provider
result = fm.tools.dispatch(name, arguments)   # execution, for every provider
```

## Supported providers

| Provider | `for_provider(...)` | Format | Schema |
| --- | --- | --- | --- |
| Anthropic | `anthropic` | Messages API tools | full |
| OpenAI, Azure OpenAI | `openai`, `azure-openai` | Chat Completions tools | full |
| OpenAI Responses API | `openai-responses` | flat function tools | full |
| Google Gemini | `gemini` | function declarations (`parametersJsonSchema`) | portable |
| OpenRouter, LiteLLM, Mistral, Groq | `openrouter`, `litellm`, `mistral`, `groq` | OpenAI-compatible | full |
| Ollama, vLLM, LM Studio (open-weight models) | `ollama`, `vllm`, `lmstudio` | OpenAI-compatible | portable |
| LangChain, LangGraph | `fm.tools.langchain()` | `StructuredTool` | full |
| Any MCP client, n8n | `fairmark-mcp` | MCP (stdio or HTTP) | full |

The **portable** schema writes decimals as strings (`"0.095"`) and drops validation keywords
that some providers and open-weight models handle poorly. Nothing is lost: strings are also the
exact representation, and the engine validates every input anyway. Use
`fm.tools.openai(portable=True)` to request it explicitly.

## OpenAI-compatible APIs (Ollama, OpenRouter, LiteLLM, vLLM)

The same code works with every OpenAI-compatible endpoint: only `base_url`, the key and the
model change.

```python
import json
from openai import OpenAI
import fairmark as fm

client = OpenAI(base_url="http://localhost:11434/v1", api_key="ollama")   # Ollama
# client = OpenAI(base_url="https://openrouter.ai/api/v1", api_key=OPENROUTER_KEY)

messages = [{"role": "user", "content": "NPV at 7% of -5000, 1200, 1500, 1800, 2100?"}]
response = client.chat.completions.create(
    model=MODEL, messages=messages, tools=fm.tools.for_provider("ollama"),   # e.g. a Qwen or Llama tool-use model
)
for call in response.choices[0].message.tool_calls or []:
    result = fm.tools.dispatch(call.function.name, call.function.arguments)
    messages += [response.choices[0].message,
                 {"role": "tool", "tool_call_id": call.id, "content": json.dumps(result)}]
```

With open-weight models, choose a model trained for tool use and keep the system prompt short.
Fairmark's tool descriptions state units explicitly, which is what makes extraction reliable.

## Google Gemini

```python
from google import genai
from google.genai import types
import fairmark as fm

client = genai.Client()
config = types.GenerateContentConfig(
    tools=[types.Tool(function_declarations=fm.tools.gemini())],
)
response = client.models.generate_content(
    model=MODEL, contents="NPV at 7% of -5000, 1200, 1500, 1800, 2100?", config=config,
)
for call in response.function_calls or []:
    result = fm.tools.dispatch(call.name, call.args)
```

## Anthropic

See [Function calling](function-calling.md) and the
[agent tutorial](../getting-started/first-valuation-with-an-agent.md).

## LangChain and LangGraph

```python
import fairmark as fm   # pip install "fairmark[langchain]"

tools = fm.tools.langchain()     # list of StructuredTool, ready for bind_tools or a ToolNode
```

## n8n and other automation platforms

n8n's AI Agent connects to MCP servers over HTTP with the MCP Client Tool. Start Fairmark's
MCP server in HTTP mode and point the node at it: see [MCP server](mcp.md). A dedicated n8n
node is on the roadmap.
