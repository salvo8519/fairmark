---
source: concepts/architecture.md
source_hash: 13b119240b64
---
# Architettura

## Principi

- **Un solo motore, adattatori sottili.** Tutta la logica finanziaria vive in
  `fairmark-core` (Rust). Gli adattatori traducono, non calcolano mai.
- **Il registro è l'unica fonte di verità.** `registry.rs` elenca ogni metodo con la sua
  versione e il suo tipo di input. Gli JSON Schema sono generati dai tipi Rust (`schemars`) e
  ogni adattatore è generato da essi: helper Python, CLI, tool MCP, definizioni per il
  function calling, documentazione.
- **JSON ai confini.** I dati attraversano il confine Rust/Python come stringhe JSON con i
  decimali come stringhe. L'interfaccia FFI resta limitata a tre funzioni e ogni adattatore
  si testa con gli stessi casi.
- **Nessun I/O nel core.** Il core è una funzione pura dall'input al risultato. Telemetria,
  file e rete vivono solo negli adattatori.

## Struttura

```
crates/
  fairmark-core/    metodi, matematica decimale, explain, registro, errori (nessun I/O)
  fairmark-cli/     binario `fairmark`: methods | schema | run | verify
  fairmark-py/      ponte PyO3 (3 funzioni), compilato da maturin come fairmark._engine
python/fairmark/    API Python, tool per il function calling, server MCP
skills/fairmark/    Agent Skill per Claude Code, Codex e agenti compatibili
tests/golden/       casi di riferimento condivisi (Rust + Python)
docs/               documentazione in inglese (en) e italiano (it)
roadmap/            roadmap.yaml → milestone, epic e task su GitHub
```

## Aggiungere un metodo

1. Implementarlo in `crates/fairmark-core/src/<area>.rs` con una struttura di input (serde +
   `JsonSchema`, `deny_unknown_fields`, commento su ogni campo) e una funzione che restituisce
   `Result<(Output, Explain)>`.
2. Registrarlo in `registry.rs` (`METHODS`, `input_schema`, `run`).
3. Aggiungere casi di riferimento in `tests/golden/<metodo>/` da una fonte indipendente.
4. Aggiungere un helper Python sottile in `python/fairmark/<area>.py`.
5. Scrivere la pagina metodologica in `docs/en/methods/` e la sua traduzione in
   `docs/it/methods/`.

Tool per il function calling, tool MCP e supporto CLI arrivano senza lavoro aggiuntivo.

## Prestazioni

Il motore è codice nativo senza astrazioni costose; un DCF completo con explain richiede
microsecondi. L'aritmetica decimale è più lenta della virgola mobile, per scelta: nella
valutazione l'esattezza conta più dei nanosecondi, e il motore resta di ordini di grandezza più
veloce di qualsiasi chiamata a un LLM. Le API batch (griglie di sensitività, Monte Carlo con
seme esplicito) gireranno in Rust in una sola chiamata per evitare costi per chiamata.
