---
source: reference/errors.md
source_hash: ef289aa4908a
---
# Codici di errore

I codici sono stabili: una volta rilasciato, un codice non cambia mai significato. Ogni
adattatore (Python, CLI, MCP, function calling) riporta gli errori come
`{"code": "...", "message": "..."}`. I messaggi sono in inglese.

| Codice | Significato | Correzione tipica |
| --- | --- | --- |
| `FM-E001` | Input non valido: tipo errato, campo obbligatorio mancante, campo sconosciuto | Controlla l'input con `fairmark schema <metodo>` |
| `FM-E002` | Serie di flussi di cassa vuota | Fornisci almeno un flusso |
| `FM-E003` | Tasso ≤ −100% | I tassi sono frazioni decimali: 10% è `0.10` |
| `FM-E004` | Crescita di lungo periodo ≥ tasso di attualizzazione | Il modello di Gordon richiede g < WACC |
| `FM-E005` | L'algoritmo del TIR non converge | Controlla i flussi; prova un `guess` diverso |
| `FM-E006` | Nessun cambio di segno nei flussi | Il TIR richiede sia uscite sia entrate |
| `FM-E007` | Metodo o tool sconosciuto | Vedi `fairmark methods` |
| `FM-E008` | Overflow aritmetico | Input fuori da un intervallo realistico |
