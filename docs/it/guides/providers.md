---
source: guides/providers.md
source_hash: 5011f739a184
---
# Provider LLM

Fairmark è indipendente dal provider. Il motore non chiama mai un modello: espone dei tool, e
qualsiasi modello che supporti l'uso dei tool può chiamarli. Una funzione restituisce le
definizioni nel formato atteso da ciascun provider; una funzione esegue le chiamate, qualunque
sia il provider.

```python
import fairmark as fm

tools = fm.tools.for_provider("ollama")       # definizioni per il provider
result = fm.tools.dispatch(name, arguments)   # esecuzione, per ogni provider
```

## Provider supportati

| Provider | `for_provider(...)` | Formato | Schema |
| --- | --- | --- | --- |
| Anthropic | `anthropic` | tool della Messages API | completo |
| OpenAI, Azure OpenAI | `openai`, `azure-openai` | tool di Chat Completions | completo |
| OpenAI Responses API | `openai-responses` | tool di funzione "piatti" | completo |
| Google Gemini | `gemini` | function declaration (`parametersJsonSchema`) | portabile |
| OpenRouter, LiteLLM, Mistral, Groq | `openrouter`, `litellm`, `mistral`, `groq` | compatibile OpenAI | completo |
| Ollama, vLLM, LM Studio (modelli open-weight) | `ollama`, `vllm`, `lmstudio` | compatibile OpenAI | portabile |
| LangChain, LangGraph | `fm.tools.langchain()` | `StructuredTool` | completo |
| Qualsiasi client MCP, n8n | `fairmark-mcp` | MCP (stdio o HTTP) | completo |

Lo schema **portabile** scrive i decimali come stringhe (`"0.095"`) e rimuove le parole chiave
di validazione che alcuni provider e modelli open-weight gestiscono male. Non si perde nulla: le
stringhe sono anche la rappresentazione esatta, e il motore valida comunque ogni input. Per
richiederlo in modo esplicito usa `fm.tools.openai(portable=True)`.

## API compatibili con OpenAI (Ollama, OpenRouter, LiteLLM, vLLM)

Lo stesso codice funziona con ogni endpoint compatibile con OpenAI: cambiano solo
`base_url`, la chiave e il modello.

```python
import json
from openai import OpenAI
import fairmark as fm

client = OpenAI(base_url="http://localhost:11434/v1", api_key="ollama")   # Ollama
# client = OpenAI(base_url="https://openrouter.ai/api/v1", api_key=OPENROUTER_KEY)

messages = [{"role": "user", "content": "VAN al 7% di -5000, 1200, 1500, 1800, 2100?"}]
response = client.chat.completions.create(
    model=MODEL, messages=messages, tools=fm.tools.for_provider("ollama"),   # ad es. un modello Qwen o Llama con tool use
)
for call in response.choices[0].message.tool_calls or []:
    result = fm.tools.dispatch(call.function.name, call.function.arguments)
    messages += [response.choices[0].message,
                 {"role": "tool", "tool_call_id": call.id, "content": json.dumps(result)}]
```

Con i modelli open-weight scegli un modello addestrato all'uso dei tool e mantieni breve il
prompt di sistema. Le descrizioni dei tool di Fairmark dichiarano le unità in modo esplicito,
ed è questo che rende affidabile l'estrazione degli input.

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
    model=MODEL, contents="VAN al 7% di -5000, 1200, 1500, 1800, 2100?", config=config,
)
for call in response.function_calls or []:
    result = fm.tools.dispatch(call.name, call.args)
```

## Anthropic

Vedi [Function calling](function-calling.md) e il
[tutorial con un agente](../getting-started/first-valuation-with-an-agent.md).

## LangChain e LangGraph

```python
import fairmark as fm   # pip install "fairmark[langchain]"

tools = fm.tools.langchain()     # lista di StructuredTool, pronta per bind_tools o un ToolNode
```

## n8n e altre piattaforme di automazione

L'AI Agent di n8n si collega ai server MCP via HTTP con l'MCP Client Tool. Avvia il server MCP
di Fairmark in modalità HTTP e collega il nodo: vedi [Server MCP](mcp.md). Un nodo n8n dedicato
è in roadmap.
