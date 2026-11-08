---
source: guides/mcp.md
source_hash: 38fa65d72d02
---
# Server MCP

`fairmark-mcp` espone ogni metodo come tool MCP. Per impostazione predefinita gira in locale
via stdio: non c'è un server da ospitare e nessun dato lascia il tuo computer. Con `--http`
espone il trasporto Streamable HTTP, per n8n e per gli altri client che si collegano via HTTP.

## Configura il client

```json
{
  "mcpServers": {
    "fairmark": { "command": "uvx", "args": ["--from", "fairmark[mcp]", "fairmark-mcp"] }
  }
}
```

In alternativa, installalo in un ambiente e usa `"command": "fairmark-mcp"`.

## Modalità HTTP (n8n e client remoti)

```bash
fairmark-mcp --http                                # http://127.0.0.1:8765/mcp
fairmark-mcp --http --host 0.0.0.0 --port 8765 --log-format json
```

In n8n: aggiungi un **MCP Client Tool** al nodo AI Agent, scegli il trasporto HTTP Streamable e
imposta l'URL `http://<host>:8765/mcp`. L'agente vede ogni metodo di Fairmark come tool.

Il server ascolta su `127.0.0.1` per impostazione predefinita. Esponilo in rete solo dietro un
tuo sistema di autenticazione (reverse proxy, VPN): la modalità HTTP non ha un controllo
d'accesso integrato.

## Log

I log vanno su stderr; `--log-format json` scrive un oggetto JSON per riga con il tool, l'esito
(`ok` o un codice di errore), la durata e l'hash dell'input. I log non contengono mai valori
finanziari.

## Che cosa riceve il client

- Un tool per metodo (`fairmark_dcf_fcff`, `fairmark_tvm_npv`, ...), con lo schema di input
  completo, marcato come di sola lettura e idempotente.
- Lo stato di ogni metodo nel campo `_meta` del tool, `{"fairmark/status": "available"}`
  oppure `"preview"`. La descrizione di un tool in anteprima riporta "(preview)" dopo il
  titolo e una frase che inizia con "Status: preview", così il modello lo vede prima della
  chiamata. La pagina sullo
  [stato dei metodi](https://salvo8519.github.io/fairmark/project/releases/#method-status)
  (in inglese) definisce i due stati.
- Istruzioni del server che chiedono al modello di usare i tool per ogni numero, di citare
  `explain.steps`, di dichiarare all'utente quando un risultato proviene da un metodo in
  anteprima e di presentare ogni risultato come un calcolo sotto la responsabilità
  dell'utilizzatore, mai come una consulenza (vedi le [avvertenze legali](../disclaimer.md)).
- Risultati come contenuto strutturato (la busta JSON completa) e come testo.
- Errori segnalati con `isError` e un codice stabile, così il modello può correggere la
  chiamata.
