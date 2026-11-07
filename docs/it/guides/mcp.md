---
source: guides/mcp.md
source_hash: cf01b4c5999a
---
# Server MCP

`fairmark-mcp` espone ogni metodo come tool MCP. Gira in locale via stdio: non c'è un server
da ospitare e nessun dato lascia il tuo computer.

## Configura il client

```json
{
  "mcpServers": {
    "fairmark": { "command": "uvx", "args": ["--from", "fairmark[mcp]", "fairmark-mcp"] }
  }
}
```

In alternativa, installalo in un ambiente e usa `"command": "fairmark-mcp"`.

## Che cosa riceve il client

- Un tool per metodo (`fairmark_dcf_fcff`, `fairmark_tvm_npv`, ...), con lo schema di input
  completo, marcato come di sola lettura e idempotente.
- Istruzioni del server che chiedono al modello di usare i tool per ogni numero e di citare
  `explain.steps`.
- Risultati come contenuto strutturato (la busta JSON completa) e come testo.
- Errori segnalati con `isError` e un codice stabile, così il modello può correggere la
  chiamata.
