# ADR 0006 — Provider-agnostic tools

Status: accepted · 2026-11-07

## Context
Analysts and companies use different model providers: Anthropic, OpenAI, Google Gemini, and
open-weight models served locally (Ollama, vLLM, LM Studio) or through gateways (OpenRouter,
LiteLLM). Automation platforms such as n8n connect to tools over MCP. Tying Fairmark to one
provider would exclude privacy-sensitive users who run models on their own machines.

## Decision
The engine never calls a model. The Python adapter exposes the same tools in every provider
format from one source (`fm.tools.for_provider`), with a portable schema variant for providers
and models with limited JSON Schema support, and one provider-independent `dispatch`. The MCP
server offers stdio and Streamable HTTP. New providers are added as formats, never as engine
changes.

## Consequences
+ Works with closed and open-weight models alike, including fully local setups.
+ n8n integration is available today through MCP over HTTP, before a dedicated node.
− Each provider format is a contract to test; format changes by providers need maintenance
  in `tools.py` (covered by `python/tests/test_providers.py`).
