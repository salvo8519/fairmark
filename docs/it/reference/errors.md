---
source: reference/errors.md
source_hash: 14cf83dc1f77
---
# Codici di errore

I codici sono stabili: una volta rilasciato, un codice non cambia mai significato. Ogni
adattatore (Python, CLI, MCP, function calling) riporta gli errori come
`{"code": "...", "message": "..."}`. I messaggi sono in inglese.

| Codice | Significato | Correzione tipica |
| --- | --- | --- |
| `FM-E001` | Input non valido: tipo errato, campo obbligatorio mancante, campo sconosciuto | Controlla l'input con `fairmark schema <method>` |
| `FM-E002` | Serie di flussi di cassa vuota | Fornisci almeno un flusso di cassa |
| `FM-E003` | Tasso fuori intervallo: il tasso per periodo, o per periodo di capitalizzazione, è pari o inferiore a −100%; il messaggio indica il tasso con il suo JSON Pointer, per esempio `/wacc_per_period/3` nel DCF o `/finance_rate` in `tvm.mirr`; `rates.convert`, `tvm.xnpv` e `loans.*` indicano il valore e la regola per cui uno più il tasso per periodo di capitalizzazione ($1 + r/m$) deve essere positivo, senza indicare il campo | I tassi sono frazioni decimali: 10% è `0.10`; un tasso nominale con capitalizzazione è verificato per periodo di capitalizzazione ($r/m$) |
| `FM-E004` | Crescita ≥ tasso di attualizzazione (valore terminale, rendita perpetua); nel DCF e in `tvm.perpetuity` il messaggio indica la crescita e il tasso con i loro JSON Pointer; nel DCF precisa anche quando il tasso terminale è stato convertito con l'inflazione attesa | Il modello di Gordon richiede g < r |
| `FM-E005` | L'algoritmo del TIR o dell'XIRR non converge; il messaggio indica l'algoritmo | Il tasso può trovarsi fuori dall'intervallo dei decimali o lontano da `guess`: indica un `guess` vicino al tasso atteso |
| `FM-E006` | `cash_flows` non contiene alcun importo positivo o alcun importo negativo | TIR, XIRR e MIRR richiedono sia uscite sia entrate |
| `FM-E007` | Metodo o tool sconosciuto | Vedi `fairmark methods` |
| `FM-E008` | Overflow aritmetico | Input fuori da un intervallo realistico |
| `FM-E009` | Data non valida: malformata, inesistente (`2023-02-29`) o fuori dall'intervallo 1900-01-01 - 2199-12-31 | Scrivi le date in formato ISO 8601 `YYYY-MM-DD` entro l'intervallo supportato |
| `FM-E010` | Un flusso ha una data anteriore alla data di valutazione | Sposta la data di valutazione alla data del primo flusso o prima, oppure omettila per usare la data del primo flusso |
| `FM-E011` | Nessun tasso unico può essere isolato: più tassi annullano il valore attuale, è possibile una radice multipla, oppure i cambi di segno dei flussi indicano che esiste un altro tasso; il messaggio elenca gli intervalli | Il TIR è ambiguo: usa `tvm.npv` al costo del capitale, oppure `tvm.mirr` con tassi espliciti |
| `FM-E012` | Errore interno: un difetto di Fairmark, non dell'input | Segnalarlo con l'`input_hash`; l'input non serve mai |
| `FM-E013` | Ricerca obiettivo: l'output meno il valore obiettivo ha lo stesso segno a entrambi gli estremi dell'intervallo di ricerca | Amplia l'intervallo di ricerca in modo che il valore obiettivo sia compreso tra gli output ai due estremi |
| `FM-E014` | La ricerca obiettivo non ha raggiunto la tolleranza entro `max_iterations`; restituito anche quando l'output salta oltre il valore obiettivo (discontinuità) | Aumenta `max_iterations` o la tolleranza; verifica che l'output sia continuo nella variabile |
| `FM-E015` | Un puntatore di un'analisi (`variable`, `output` o il `pointer` di una modifica di scenario) non si risolve in un decimale | Usa un JSON Pointer RFC 6901 a un campo decimale esistente, ad esempio `/discount_rate` |
| `FM-E016` | L'analisi richiede più di 10.000 valutazioni del metodo interno, oppure più di 1.000.000 di valutazioni × valori in `base_input` | Suddividi la richiesta in tabelle più piccole o riduci l'input interno |
| `FM-E017` | A un metodo di analisi è stato chiesto di valutare un altro metodo di analisi | Esegui direttamente l'analisi interna; le analisi non si annidano |
| `FM-E018` | Un input del valore terminale è fuori dal suo dominio economico: rendimento del nuovo capitale investito o tasso di capitalizzazione pari a zero o negativi, multiplo o grandezza di riferimento pari a zero o negativi, orizzonte finito fuori dall'intervallo da 1 a 10.000 periodi, H negativo, oppure valore di realizzo, passività, costo di liquidazione o imposte sulle plusvalenze negativi | Controlla il segno e l'unità del campo indicato nel messaggio; le obbligazioni vanno in `liabilities`, non come attività negative |
| `FM-E019` | Grandezze enterprise ed equity mescolate: un confronto di valori terminali dichiarato su un approccio contiene un multiplo di uscita, un reddito normalizzato o una liquidazione dell'altro approccio, oppure la tecnica del valore terminale di un DCF è un multiplo di uscita, un reddito normalizzato o una liquidazione dell'altro approccio (P/E e P/BV sono equity side; EV/EBITDA, EV/EBIT, EV/Sales sono asset side; `dcf.fcff` è asset side) | Usa un solo approccio: un multiplo enterprise in un confronto asset side o in `dcf.fcff`, il P/E o il P/BV in un confronto equity side, e dichiara un reddito normalizzato o una liquidazione sull'approccio del confronto o del DCF |
| `FM-E020` | Balloon fuori intervallo: negativo, non inferiore al capitale, oppure indicato con un piano `bullet` | Indica un balloon di almeno 0 e inferiore al capitale; omettilo per `bullet` |
| `FM-E021` | La rata non rimborsa mai il finanziamento: è pari a 0 o negativa, oppure non supera la quota interessi del primo periodo, qualunque sia il segno del tasso | Aumenta `instalment` oltre 0 e oltre tasso periodico × capitale (per rate anticipate, tasso periodico × capitale / (1 + tasso periodico)) |
| `FM-E022` | Preammortamento non inferiore al numero delle rate, oppure indicato con un piano `bullet` | Indica meno periodi di preammortamento che rate; omettili per `bullet`, che prevede solo interessi fino all'ultima rata |
| `FM-E023` | Con l'arrotondamento, il debito residuo diventerebbe negativo prima dell'ultimo periodo, oppure la rettifica dell'ultima rata renderebbe negativa l'ultima quota capitale o l'ultima rata | Arrotonda a più decimali, oppure verifica che il finanziamento sia abbastanza grande per l'arrotondamento scelto |
| `FM-E024` | I flussi di cassa e i tassi di attualizzazione di un DCF sono dichiarati in termini diversi (`nominal` e `real`) e non è fornita l'inflazione attesa | Fornire l'inflazione attesa (`inflation`, oppure `inflation_per_period` con `terminal_inflation`), in modo che i tassi di attualizzazione siano convertiti nei termini dei flussi di cassa con la relazione di Fisher esatta; oppure esprimere i tassi di attualizzazione o i flussi di cassa negli stessi termini, in modo che `cash_flow_terms` coincida con `discount_rate_terms` |
| `FM-E025` | Un conteggio di un DCF non corrisponde agli stage: uno stage `given` con più o meno flussi dei suoi `periods`, tassi o inflazione attesa per periodo non uno per ogni periodo dell'orizzonte esplicito, un base flow (flusso dell'anno base) mancante per un primo stage generato, oppure fornito quando il primo stage è `given` | Fornire esattamente un flusso, un tasso o un'inflazione per periodo, come indica il messaggio; fornire `base_flow` solo quando il primo stage è `constant_growth` o `linear_decline` |
