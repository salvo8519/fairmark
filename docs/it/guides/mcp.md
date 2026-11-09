---
source: guides/mcp.md
source_hash: 593f5f5dfddd
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

## Meno tool o tool più piccoli

Per impostazione predefinita il server elenca ogni tool con lo schema completo. Due opzioni
riducono ciò che il modello vede, per i client con un contesto limitato o un compito ristretto:

```bash
fairmark-mcp --areas dcf,tvm          # solo i tool di queste aree
fairmark-mcp --compact                # schemi compatti
fairmark-mcp --http --areas statements,ratios --compact
```

Nella configurazione del client si aggiungono ad `args`, per esempio
`["--from", "fairmark[mcp]", "fairmark-mcp", "--areas", "dcf,tvm"]`. Le aree sono la parte
dell'identificativo del metodo prima del punto (`analysis`, `coc`, `dates`, `dcf`, `loans`,
`rates`, `ratios`, `series`, `statements`, `tv`, `tvm`); un'area sconosciuta arresta il server
con l'elenco di quelle note. Il profilo compatto è quello del
[function calling](function-calling.md#profilo-compatto): sono accettati gli stessi input e
ogni valore enumerato conserva il suo significato. Un tool fuori dalle aree scelte non è
elencato, ma una sua chiamata viene comunque eseguita.

## Log

I log vanno su stderr; `--log-format json` scrive un oggetto JSON per riga con il tool, l'esito
(`ok` o un codice di errore), la durata e l'hash dell'input. I log non contengono mai valori
finanziari.

## Che cosa riceve il client

- Un tool per metodo (`fairmark_dcf_fcff`, `fairmark_tvm_npv`, ...), con lo schema di input
  completo, marcato come di sola lettura e idempotente.
- Il significato di ogni valore enumerato nella descrizione del campo, ed esempi di input presi
  dai golden case nella parola chiave `examples` dello schema (prima il minimo, poi il massimo
  dove utile; vedi [function calling](function-calling.md#esempi)).
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
