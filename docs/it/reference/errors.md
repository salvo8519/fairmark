---
source: reference/errors.md
source_hash: d24e6709270f
---
# Codici di errore

I codici sono stabili: una volta rilasciato, un codice non cambia mai significato. Ogni
adattatore (Python, CLI, MCP, function calling) riporta un errore con lo stesso report,
descritto sotto: il codice, il messaggio e i campi che servono a un client per correggere
l'input senza leggere il messaggio. I messaggi sono in inglese. I messaggi indicano il campo
da correggere con il suo JSON Pointer (RFC 6901), per esempio `/stages/0/flows/1`, e la
correzione da apportare. Per un metodo valutato da un'analisi, il messaggio mantiene il
pointer interno a `base_input`, mentre il `pointer` del report parte dalla radice dell'input
dell'analisi, sotto `/base_input` (per esempio `/base_input/wacc`).

## Il report di errore

Una chiamata rifiutata restituisce `{"error": <report>}` (ADR 0014 G5):

```json
{"error": {
  "code": "FM-E001",
  "message": "invalid input: `principal` must be greater than 0 (got 0)",
  "pointer": "/principal",
  "pointers": [],
  "reason": "range",
  "expected": ["an amount greater than 0"],
  "fix": null,
  "errors": [
    {"code": "FM-E001", "message": "invalid input: `principal` must be greater than 0 (got 0)",
     "pointer": "/principal", "pointers": [], "reason": "range",
     "expected": ["an amount greater than 0"], "fix": null},
    {"code": "FM-E001",
     "message": "invalid input: `number_of_instalments` must be between 1 and 10000 (got 0)",
     "pointer": "/number_of_instalments", "pointers": [], "reason": "range",
     "expected": ["a whole number from 1 to 10000"], "fix": null}
  ]
}}
```

| Campo | Contenuto |
| --- | --- |
| `code` | Il codice stabile della tabella sotto. |
| `message` | Che cosa non va e, quando è noto, come correggerlo, in inglese. Lo stesso testo di prima che il report avesse gli altri campi. |
| `pointer` | JSON Pointer del campo da correggere, sempre dalla radice dell'input inviato (dentro un'analisi, sotto `/base_input`); `""` (il pointer vuoto) per l'input nel suo insieme, per esempio un input che non è un oggetto. È `null` solo per un testo che non è JSON (`reason` `syntax`, il messaggio indica riga e colonna), per un errore sollevato durante il calcolo che nessun singolo campo causa (per esempio `FM-E005`, `FM-E008`, `FM-E012`), per `FM-E027`, che indica i suoi input in `missing_any_of`, e per `FM-E007` quando l'id del metodo non fa parte dell'input (l'argomento del metodo della CLI, di `fairmark.run` o di una chiamata di tool; dentro un'analisi è `/method`). |
| `pointers` | Tutti i campi di una relazione: i due lati di una quadratura `FM-E026`, i due campi di un conflitto, un valore e il campo da cui dipende il suo dominio (un `/rate` nominale e il suo `/compounding` o `/from`, una tecnica di un'altra base e `/basis`, `metric_value` e `metric` di un multiplo di uscita), un campo mancante e la chiave scritta male che lo rappresenta; vuoto quando l'errore riguarda un solo campo. |
| `reason` | Il sottotipo dell'errore (valori sotto), che distingue le condizioni che condividono un codice, soprattutto `FM-E001`; `null` quando il codice ha un solo significato. |
| `expected` | Una lista di stringhe. Quando i valori ammessi formano un elenco chiuso, i valori letterali: i valori di un'enumerazione (`reason` `enum`, e gli id dei metodi di `FM-E007`), i campi che un oggetto accetta (`unknown_field`, compreso il tag di un oggetto etichettato internamente), le chiavi di una scelta scritta come oggetto con una sola chiave (`length`, `unknown_field`). Altrimenti brevi descrizioni della forma ammessa in parole semplici, per esempio `"an amount greater than 0"` o `"a whole number of 0 or more"`, mai il nome di un tipo Rust. Vuota quando non sono noti. Quando anche il messaggio cita i valori ammessi, l'elenco completo è `expected`, non il messaggio. |
| `fix` | Una frase che risolve l'errore; `null` quando il messaggio è l'unica indicazione. La formulazione è un'indicazione e può cambiare tra un rilascio e l'altro; fai affidamento su `code`, `pointer` e `reason`. |
| `missing_any_of` | Solo `FM-E027`: insiemi alternativi di pointer dell'input, ciascuno dei quali basta a portare l'input sopra il minimo del metodo, per esempio `[["/closing/cash"], ["/closing/current_assets"]]`. |
| `errors` | Tutti gli errori trovati, almeno uno (`minItems` 1); il primo è il report di primo livello stesso. Le voci non hanno un proprio `errors`. |

**Tutti gli errori dell'input, in una chiamata.** Ogni metodo valida tutto l'input prima di
calcolare qualsiasi cosa: la validazione prosegue dopo un rifiuto e riporta tutti gli errori che
trova, nell'ordine in cui li controlla, quindi il primo errore è quello che darebbe un report
singolo. Un controllo che dipende da un input rifiutato viene saltato (per esempio il conteggio
dei tassi quando uno stadio del DCF è rifiutato, il confronto di una crescita con un tasso a sua
volta rifiutato, o il resto di una rettifica i cui importi non sono uno per esercizio); un
controllo che non ne dipende viene comunque fatto (un `wacc` rifiutato non nasconde una crescita
terminale rifiutata, un `principal` rifiutato non nasconde un tasso fuori intervallo).

- **Lettura dell'input.** Viene riportato ogni campo che non si riesce a leggere: il primo
  come lo riporta `serde` (ADR 0013), poi ogni altro valore di tipo errato, campo obbligatorio
  mancante e valore sconosciuto di un'enumerazione, ciascuno con il messaggio, il pointer, la
  reason, `expected` e `fix` che avrebbe se fosse l'unico errore. Il testo viene anche letto
  come JSON e percorso secondo lo schema del metodo, e vengono riportate tutte le chiavi
  sconosciute, tutte le chiavi indicate due volte nello stesso oggetto e tutti gli oggetti
  scritti come array posizionale, ciascuno con il suo pointer. Una chiave sconosciuta riporta in
  `expected` i campi che il suo oggetto accetta e una `fix` che indica il campo ammesso più
  vicino quando la chiave sembra un errore di battitura, senza distinguere maiuscole e
  minuscole (per esempio "rename `wac` to `wacc`"). `reason` `syntax` vale solo per un testo
  che non è JSON; un valore di tipo diverso per una scelta (`true` per `timing`, o un oggetto
  per `formula`) è `enum` sul suo campo, con i valori ammessi. Un campo ha la stessa reason e
  lo stesso `expected` ovunque si trovi: dentro una tecnica di `tv.compare` come nella tecnica
  indicata direttamente.
- **Errori di lettura e controlli del metodo.** Quando l'input non si riesce a leggere, i
  controlli del metodo vengono eseguiti comunque sui campi letti, e i loro errori sono
  riportati dopo quelli di lettura: un `"+0.1"` in un tasso non nasconde un'aliquota di -1.
  Ogni valore rifiutato è sostituito da uno che lo schema accetta solo per poter leggere
  l'input, e quel valore non decide mai un errore riportato. Ogni errore dipende da campi
  noti: il suo `pointer`, i suoi `pointers` e i campi che il suo controllo ha letto in altro
  modo (per esempio la proiezione di uno stadio, che decide i campi che lo stadio accetta).
  Viene riportato solo quando nessuno di essi è un campo rifiutato, un campo al suo interno o
  un campo che lo contiene: con `compounding` rifiutato, la regola di `rate`, che ne dipende,
  non viene controllata; con `stages/0/periods` rifiutato, un `cost_of_equity` di -5 viene
  comunque riportato. Un controllo non si ferma mai a un rifiuto prima di un controllo che non
  ne dipende.
- **Una chiave scritta male rappresenta il suo campo.** Una chiave con un suggerimento di
  rinomina (`wac` per `wacc`) vale come il campo che indica: nessun controllo che dipende da
  quel campo viene riportato e, quando il campo è obbligatorio, la sua voce `missing` ha
  `pointers` `[campo, chiave]` (per esempio `["/wacc", "/wac"]`) e la stessa fix della chiave,
  "rename `wac` to `wacc`".
- **Una voce per condizione.** Un oggetto scritto come array è un solo errore `type`
  sull'oggetto, senza errori per i suoi elementi. Una condizione incontrata più volte (una
  crescita costante controllata per ogni periodo) è riportata una volta. Quando il tag di un
  oggetto è rifiutato (il `kind` di una rettifica di `statements.reorganise`, mancante o
  sconosciuto), l'errore del tag è l'unica voce di quell'oggetto, con le sue chiavi
  sconosciute: gli altri campi dipendono dal tipo e non vengono indovinati. Una chiave
  sconosciuta di un oggetto così riporta in `expected` i campi di tutti i tipi.
- **Grammatica dei numeri.** Un decimale è un numero JSON o una stringa con la grammatica dei
  numeri dell'ADR 0014 G7, `^-?\d+(\.\d+)?([eE][+-]?\d+)?$`: punto decimale, cifre da entrambi
  i lati, un esponente facoltativo con segno facoltativo (`"1E-7"`, `"1e+3"`), nessun altro
  segno salvo un meno iniziale, nessun separatore delle migliaia e nessun simbolo di valuta.
  Una stringa fuori dalla grammatica (`"+0.1"`, `".1"`, `"1."`, `"1_000"`, `"1,000"`) è `type`
  sul suo campo, in tutti i metodi. Il motore accetta già un esponente con segno; il pattern
  pubblicato negli schemi di input riporta ancora `[eE]\d+` e acquisisce il segno con la issue
  #266.
- **Il JSON ambiguo viene rifiutato.** Una chiave indicata due volte nello stesso oggetto
  (`duplicate`, sulla chiave) e un oggetto scritto come array dei suoi valori (`type`,
  sull'array; `""` per un input che è un array) sono rifiutati in tutti i metodi: verrebbero
  letti con l'ultimo valore o per posizione.
- **Scelte scritte come oggetto con una sola chiave** (`terminal` del DCF, le tecniche di
  `tv.compare`, `growth` e `timeline` di `series.project`): una chiave che non è tra le scelte è
  `unknown_field` sulla chiave, con le scelte e la più vicina, anche accanto a una chiave
  valida; quando è l'unica chiave (`{"tv.godon": {...}}`), è l'unica voce, senza `length` per
  la scelta che lascia vuota. Nessuna chiave, o una seconda chiave che è tra le scelte, è
  `length` (sull'oggetto, o sulla seconda chiave), con le scelte in `expected`.
- **Analisi.** I campi dell'analisi e il suo `base_input` sono controllati in un solo
  passaggio: `base_input` è controllato dal metodo che indica, con la sola validazione di quel
  metodo (ogni controllo del suo input, nessun calcolo), e i suoi errori di input sono
  riportati sotto `/base_input`, anche quando l'input dell'analisi stesso non si riesce a
  leggere. Il controllo di `base_input` non aggiunge valutazioni: un'analisi valuta il metodo
  interno solo per i suoi risultati. `analysis.goal_seek` controlla `base_input` con la
  variabile all'estremo inferiore dell'intervallo, il primo valore che valuta, anche quando
  l'input dell'analisi stesso non si riesce a leggere, così il valore che la variabile
  sostituisce non viene mai rifiutato. Una variabile o una modifica che punta a un campo che
  accetta un numero intero è `FM-E015` sul proprio campo. Una variabile o una modifica che punta
  a un valore che `base_input` rifiuta in lettura (un valore di tipo errato, un campo mancante,
  un campo rappresentato da una chiave scritta male), o a un campo al suo interno, non viene
  controllata: il rifiuto è riportato sotto `/base_input`, senza un `FM-E015`. Un metodo
  sconosciuto (`FM-E007`) è
  l'unico errore che lo riguarda: le variabili non sono controllate rispetto a esso. La
  dimensione della richiesta (`FM-E016`, un errore di input) è controllata per prima: una
  richiesta oltre i limiti non valuta nulla. Gli input che un'analisi costruisce per le sue
  valutazioni da un `base_input` già controllato sono letti solo da `serde`, senza percorrerli
  di nuovo secondo lo schema.
- **Errori del calcolo.** Un errore del calcolo (un overflow, un solver che non converge, un
  difetto) non compare mai accanto agli errori dell'input: quando l'input ha errori, vengono
  riportati quelli; solo un input senza errori arriva al calcolo, il cui primo errore lo
  interrompe ed è l'unica voce.
- **Condizioni dell'input trovate dal calcolo.** Alcune condizioni dell'input si possono
  trovare solo calcolando: non fanno parte della validazione (né del controllo di `base_input`
  in un'analisi) e sono riportate da sole, quando l'input non ha altri errori: una quadratura
  dei totali di `statements.reorganise` (`FM-E026`); un arrotondamento troppo grossolano per un
  piano di ammortamento (`FM-E023`, su `/rounding/decimal_places`); per
  `loans.number_of_instalments`, una rata che non rimborsa mai il finanziamento (`FM-E021`) e
  un numero di rate calcolato oltre il limite (`FM-E001`); una data generata da
  `dates.schedule` dopo il 2199-12-31 (`FM-E009`); un input di un metodo `ratios.*` sotto il suo
  minimo (`FM-E027`).

Lo schema del report fa parte di ogni schema del risultato (`fairmark schema <method>
--envelope`, `fairmark.result_schema(method)`): `$defs/FairmarkErrorResponse`, indicato alla
radice da `x-fairmark-error`. Le celle di una tabella di analisi mantengono il proprio report
compatto, `{"code", "message"}`, che fa parte dell'output dei metodi di analisi.

### Sottotipi

| `reason` | Significato | Codici |
| --- | --- | --- |
| `syntax` | Il testo non è JSON; `pointer` è `null` | `FM-E001` |
| `missing` | Manca un campo obbligatorio, o richiesto dagli altri campi indicati; per `FM-E027`, nessun output ha tutti gli input che gli servono | `FM-E001`, `FM-E025`, `FM-E027` |
| `unknown_field` | Un campo che l'input non definisce; `expected` elenca i campi ammessi | `FM-E001` |
| `duplicate` | Un campo indicato due volte nello stesso oggetto, o un pointer ripetuto di un'analisi | `FM-E001` |
| `type` | Un valore del tipo sbagliato, `null` per un campo che non lo accetta, un input che non è un oggetto, un oggetto scritto come array posizionale, o una data che non è una data di calendario | `FM-E001`, `FM-E009` |
| `enum` | Un valore fuori da un elenco chiuso, o un valore di tipo diverso per una scelta; `expected` elenca i valori; un metodo sconosciuto (`expected` elenca gli id dei metodi) | `FM-E001`, `FM-E007` |
| `conflict` | Campi che si escludono a vicenda sono indicati insieme, o si contraddicono (termini, basi, date fuori ordine) | `FM-E001`, `FM-E017`, `FM-E019`, `FM-E024` |
| `requires` | Un campo è indicato senza il campo che gli serve, o non è usato dalla scelta fatta | `FM-E001`, `FM-E020`, `FM-E022`, `FM-E025` |
| `sign` | Un importo con il segno sbagliato per il suo campo | `FM-E001`, `FM-E006` |
| `blank` | Un testo che deve dire qualcosa è vuoto o fatto di soli spazi | `FM-E001` |
| `range` | Un numero fuori dal dominio del suo campo | `FM-E001`, `FM-E003`, `FM-E004`, `FM-E009`, `FM-E010`, `FM-E018`, da `FM-E020` a `FM-E023` |
| `length` | Una lista con un numero sbagliato di voci (vuota, troppo lunga, non una voce per periodo o per esercizio), o una scelta scritta come oggetto senza chiavi o con due | `FM-E001`, `FM-E002`, `FM-E025` |
| `reconcile` | I due lati di una relazione che devono coincidere differiscono | `FM-E026` |

I valori si aggiungono, non si rinominano né si rimuovono: un client accetta un valore che non
conosce. I codici sollevati durante il calcolo (`FM-E005`, `FM-E008`, da `FM-E011` a
`FM-E014`) hanno `reason` `null`, come `FM-E015` e `FM-E016`, che hanno un solo significato
ciascuno; `FM-E015` indica comunque il campo che contiene il pointer dell'analisi
(`/variable`, `/output`, `/outputs/k`, il `pointer` di una modifica).

## Codici

| Codice | Significato | Correzione tipica |
| --- | --- | --- |
| `FM-E001` | Input non valido: tipo errato, campo obbligatorio mancante, campo sconosciuto, campo indicato due volte, valore sconosciuto di una scelta; il messaggio indica il campo con il suo JSON Pointer e riporta i valori ammessi, per esempio ``invalid input at `/terminal/tv.gordon/first_flow`: missing required field `first_flow`; add it`` o ``invalid input at `/stages/0/projection`: unknown variant `constant`, expected one of `given`, `constant_growth`, `linear_decline` ``; solo un testo JSON malformato (un errore di sintassi, un input troncato) indica invece la riga e la colonna. Inoltre: in `dates.schedule` e nella `timeline` di `series.project`, entrambi `frequency` e `period_months` (in `dates.schedule` anche nessuno dei due; il messaggio indica entrambi i pointer), un `period_months` pari a 0, oppure un `number_of_periods` fuori dall'intervallo da 1 a 10000; in `series.project`, un tasso di crescita pari a −1 o inferiore, come indicato o dopo la conversione (indicato con il suo pointer, per esempio `/growth/per_period/growth_rates/1`), una lista `per_period` senza un tasso per ogni passo di crescita, un `growth_basis` mancante o non applicabile, e un `timing` mancante con una `timeline` di tipo `schedule` o indicato con una `timeline` di tipo `periods` | Correggi il campo indicato dal pointer: aggiungi un campo mancante, elimina o rinomina un campo sconosciuto, usa uno dei valori elencati; indica esattamente uno tra `frequency` e `period_months`; `fairmark schema <method>` fornisce l'input completo |
| `FM-E002` | Serie di flussi di cassa vuota | Fornisci almeno un flusso di cassa |
| `FM-E003` | Tasso fuori intervallo: il tasso per periodo, o per periodo di capitalizzazione, è pari o inferiore a −100%; il messaggio indica il tasso con il suo JSON Pointer, per esempio `/wacc_per_period/3` o `/inflation` nel DCF o `/finance_rate` in `tvm.mirr`; nel DCF un tasso convertito con l'inflazione attesa che raggiunge −100% (possibile solo al limite dell'intervallo dei decimali) mantiene il pointer del tasso fornito e aggiunge l'inflazione con cui è stato convertito; per un tasso nominale con $m$ periodi di capitalizzazione all'anno (`rates.convert`, `tvm.xnpv`, `loans.*`) il messaggio indica il tasso, per esempio `/rate`, e la regola applicabile: il tasso per periodo di capitalizzazione $r/m$ deve essere maggiore di −1, quindi il tasso deve essere maggiore di $-m$; `tvm.irr` e `tvm.xirr` applicano la stessa regola a `guess`, prima di ogni iterazione, e indicano `/guess` | I tassi sono frazioni decimali: 10% è `0.10`; un tasso nominale con capitalizzazione è verificato per periodo di capitalizzazione ($r/m$) |
| `FM-E004` | Crescita ≥ tasso di attualizzazione (valore terminale, rendita perpetua); nel DCF e in `tvm.perpetuity` il messaggio indica la crescita e il tasso con i loro JSON Pointer; nel DCF, quando il tasso terminale è stato convertito con l'inflazione attesa, indica anche il campo dell'inflazione | Il modello di Gordon richiede g < r; per un tasso terminale convertito, verificare anche l'inflazione attesa |
| `FM-E005` | L'algoritmo del TIR o dell'XIRR non converge; il messaggio indica l'algoritmo | Il tasso può trovarsi fuori dall'intervallo dei decimali o lontano da `guess`: indica un `guess` vicino al tasso atteso |
| `FM-E006` | `cash_flows` non contiene alcun importo positivo o alcun importo negativo | TIR, XIRR e MIRR richiedono sia uscite sia entrate |
| `FM-E007` | Metodo o tool sconosciuto | Vedi `fairmark methods` |
| `FM-E008` | Overflow aritmetico | Input fuori da un intervallo realistico |
| `FM-E009` | Data non valida: malformata, inesistente (`2023-02-29`) o fuori dall'intervallo 1900-01-01 - 2199-12-31; una data letta dall'input è indicata con il suo JSON Pointer, per esempio `/cash_flows/1/date`; uno schedule (`dates.schedule`, oppure la `timeline` di tipo `schedule` di `series.project` in `/timeline/schedule/number_of_periods`) la cui data di fine generata cade dopo il 2199-12-31 è indicato in `/number_of_periods`, con la prima data fuori intervallo e l'ultimo periodo che rientra | Scrivi le date in formato ISO 8601 `YYYY-MM-DD` entro l'intervallo supportato; per uno schedule, indica meno periodi o una lunghezza del periodo più breve |
| `FM-E010` | Un flusso ha una data anteriore alla data di valutazione | Sposta la data di valutazione alla data del primo flusso o prima, oppure omettila per usare la data del primo flusso |
| `FM-E011` | Nessun tasso unico può essere isolato: più tassi annullano il valore attuale, è possibile una radice multipla, oppure i cambi di segno dei flussi indicano che esiste un altro tasso; il messaggio elenca gli intervalli | Il TIR è ambiguo: usa `tvm.npv` al costo del capitale, oppure `tvm.mirr` con tassi espliciti |
| `FM-E012` | Errore interno: un difetto di Fairmark, non dell'input | Segnalarlo con l'`input_hash`; l'input non serve mai |
| `FM-E013` | Ricerca obiettivo: l'output meno il valore obiettivo ha lo stesso segno a entrambi gli estremi dell'intervallo di ricerca | Amplia l'intervallo di ricerca in modo che il valore obiettivo sia compreso tra gli output ai due estremi |
| `FM-E014` | La ricerca obiettivo non ha raggiunto la tolleranza entro `max_iterations`; restituito anche quando l'output salta oltre il valore obiettivo (discontinuità) | Aumenta `max_iterations` o la tolleranza; verifica che l'output sia continuo nella variabile |
| `FM-E015` | Un puntatore di un'analisi (`variable`, `output` o il `pointer` di una modifica di scenario) non si risolve in un decimale | Usa un JSON Pointer RFC 6901 a un campo decimale esistente, ad esempio `/discount_rate` |
| `FM-E016` | L'analisi richiede più di 10.000 valutazioni del metodo interno, oppure più di 1.000.000 di valutazioni × valori in `base_input` | Suddividi la richiesta in tabelle più piccole o riduci l'input interno |
| `FM-E017` | A un metodo di analisi è stato chiesto di valutare un altro metodo di analisi | Esegui direttamente l'analisi interna; le analisi non si annidano |
| `FM-E018` | Un input del valore terminale è fuori dal suo dominio economico: rendimento del nuovo capitale investito o tasso di capitalizzazione pari a zero o negativi, multiplo o grandezza di riferimento pari a zero o negativi, orizzonte finito fuori dall'intervallo da 1 a 10.000 periodi, H negativo, oppure valore di realizzo, passività, costo di liquidazione o imposte sulle plusvalenze negativi | Controlla il segno e l'unità del campo indicato nel messaggio; le obbligazioni vanno in `liabilities`, non come attività negative |
| `FM-E019` | Grandezze enterprise ed equity mescolate: un confronto di valori terminali dichiarato su un approccio contiene un multiplo di uscita, un reddito normalizzato o una liquidazione dell'altro approccio, oppure la tecnica del valore terminale di un DCF è un multiplo di uscita, un reddito normalizzato o una liquidazione dell'altro approccio (P/E e P/BV sono equity side; EV/EBITDA, EV/EBIT, EV/Sales sono asset side; `dcf.fcff` è asset side, `dcf.fcfe` è equity side); `tv.value_driver` in `dcf.fcfe` (i suoi parametri, NOPAT e RONIC, sono grandezze a livello d'impresa, SPEC-0001 Decision 23); una posizione finanziaria netta fornita a `dcf.fcfe`, i cui FCFE includono già il debito | Usa un solo approccio: un multiplo enterprise in un confronto asset side o in `dcf.fcff`, il P/E o il P/BV in un confronto equity side o in `dcf.fcfe`, e dichiara un reddito normalizzato o una liquidazione sull'approccio del confronto o del DCF; con `dcf.fcfe`, elimina la posizione finanziaria netta e indica in `non_operating_assets` la liquidità che i FCFE non riflettono, oppure valuta l'enterprise value con `dcf.fcff` |
| `FM-E020` | Balloon fuori intervallo: negativo, non inferiore al capitale, oppure indicato con un piano `bullet` | Indica un balloon di almeno 0 e inferiore al capitale; omettilo per `bullet` |
| `FM-E021` | La rata non rimborsa mai il finanziamento: è pari a 0 o negativa, oppure non supera la quota interessi del primo periodo, qualunque sia il segno del tasso | Aumenta `instalment` oltre 0 e oltre tasso periodico × capitale (per rate anticipate, tasso periodico × capitale / (1 + tasso periodico)) |
| `FM-E022` | Preammortamento non inferiore al numero delle rate, oppure indicato con un piano `bullet` | Indica meno periodi di preammortamento che rate; omettili per `bullet`, che prevede solo interessi fino all'ultima rata |
| `FM-E023` | Con l'arrotondamento, il debito residuo diventerebbe negativo prima dell'ultimo periodo, oppure la rettifica dell'ultima rata renderebbe negativa l'ultima quota capitale o l'ultima rata | Arrotonda a più decimali, oppure verifica che il finanziamento sia abbastanza grande per l'arrotondamento scelto |
| `FM-E024` | I flussi di cassa e i tassi di attualizzazione di un DCF sono dichiarati in termini diversi (`nominal` e `real`) e non è fornita l'inflazione attesa | Fornire l'inflazione attesa (`inflation`, oppure `inflation_per_period` con `terminal_inflation`), in modo che i tassi di attualizzazione siano convertiti nei termini dei flussi di cassa con la relazione di Fisher esatta; oppure esprimere i tassi di attualizzazione o i flussi di cassa negli stessi termini, in modo che `cash_flow_terms` coincida con `discount_rate_terms`. Quando uno dei due non è fornito, assume il valore predefinito `nominal`; il messaggio lo indica insieme al valore da fornire |
| `FM-E025` | Un conteggio di un DCF non corrisponde agli stage: uno stage `given` con più o meno flussi dei suoi `periods`, tassi o inflazione attesa per periodo non uno per ogni periodo dell'orizzonte esplicito, un base flow (flusso dell'anno base) mancante per un primo stage generato, oppure fornito quando il primo stage è `given` | Fornire esattamente un flusso, un tasso o un'inflazione per periodo, come indica il messaggio; fornire `base_flow` solo quando il primo stage è `constant_growth` o `linear_decline` |
| `FM-E026` | Il bilancio non quadra (`statements.reorganise`): un totale fornito differisce dalla somma delle sue componenti, un valore netto dal lordo meno il fondo, il totale attivo dal totale passivo e patrimonio netto, l'utile della voce A.IX del patrimonio netto dalla voce 21 del conto economico, una parte "di cui" (compresa B10 c, o in sua assenza B10 a e b, entro B10 a-c) o un importo esigibile oltre l'esercizio successivo supera il suo aggregato, la ripartizione di un override non somma all'aggregato dell'anno, gli importi esigibili oltre l'esercizio successivo delle parti di una ripartizione dei debiti verso soci per finanziamenti superano una parte o differiscono da quello dei debiti verso soci, un totale della forma riclassificata differisce dal valore derivato secondo l'Appendice B, il totale degli impieghi (`total_funds_invested`) differisce dalle fonti di un esercizio in forma riclassificata, oppure di uno in forma civilistica una volta contate le voci separate considerate pari a zero (la voce A, e le voci D ed E esposte all'interno di C.II o di D; il messaggio le indica, mentre con tutte le voci fornite la differenza è `FM-E012`), una rettifica (o l'onere finanziario del trattamento di fine rapporto) supera quanto resta nella sua voce di origine, oppure la parte corrente di una passività per leasing supera la passività; il messaggio indica entrambi i lati con i JSON pointer delle voci lette, i due importi e la differenza. Il confronto è esatto, senza tolleranza, e usa solo voci fornite (una parte esposta all'interno di C.II o di D conta come fornita): una relazione con un lato mancante, o che si basa sulla voce A o su una parte "di cui" considerate pari a zero, non è verificata ed è riportata come `not_checked` | Correggere l'importo indicato dal messaggio, fornire la voce considerata pari a zero che non è zero, oppure omettere un lato della relazione per riceverla come non verificata; per i bilanci depositati in unità di euro, fornire la riserva da arrotondamento nelle altre riserve (A.VI) in modo che i totali quadrino |
| `FM-E027` | Input sotto la soglia minima di un metodo (ADR 0014 G10.1; `ratios.*`, SPEC-0007 R12): nessun output ha tutti gli input che gli servono (per `ratios.covenants`, nessun covenant può essere verificato o deciso dalla sua regola), quindi il risultato sarebbe fatto solo di null; il messaggio indica il metodo ed elenca gli insiemi alternativi di input, ciascuno dei quali basta, per esempio ``give at least one of these sets of inputs: [`/closing/cash`] or [`/closing/current_assets`]`` | Fornire tutti gli input di uno degli insiemi elencati; le sole scelte di metodo non calcolano nulla |
