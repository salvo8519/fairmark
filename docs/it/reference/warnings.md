---
source: reference/warnings.md
source_hash: 99ddc3ecc244
---
# Codici delle avvertenze

Un'avvertenza segnala al lettore di un risultato qualcosa che potrebbe non notare: un valore
che il motore ha usato come fornito anche se sembra errato, un valore predefinito che ha
determinato un importo, una convenzione che sposta un valore. Un'avvertenza non interrompe mai
il calcolo e non cambia mai un numero: il risultato è completo ed esatto. Le condizioni che
rendono impossibile un risultato sono errori, con i loro codici (vedi
[codici di errore](errors.md)).

Ogni avvertenza è una voce di `diagnostics.warnings` nella busta (envelope) del risultato:

```json
{"code": "FM-W012", "pointer": "/net_financial_position",
 "message": "no net financial position given (`/net_financial_position`): zero is subtracted, ..."}
```

- **`code`** è stabile: `FM-W` e tre cifre. Un codice non è mai riutilizzato né rinumerato; una
  nuova condizione prende il primo codice libero, ed è una modifica additiva dell'envelope. Un
  client accetta i codici che non conosce.
- **`pointer`** è un JSON pointer RFC 6901 al campo cui si riferisce l'avvertenza, dalla radice
  indicata nella tabella: l'**input** inviato dal client (in un'analisi, sotto `/base_input`) o
  l'**envelope** del risultato (allora inizia con `/value`). È `null` solo dove la tabella lo
  indica.
- **`message`** è una frase in inglese. È un'indicazione, non un contratto: il testo può cambiare
  tra una release e l'altra; `code` e `pointer` no.

Ogni avvertenza è riportata una sola volta anche in `explain.conventions`, come
`warning FM-Wnnn: <message>`, nello stesso ordine di `diagnostics.warnings`. Nessuna
convenzione inizia con `warning` senza la sua voce. Leggere `diagnostics.warnings` invece di
analizzare il testo delle convenzioni.

## Ordine delle avvertenze

`diagnostics.warnings` elenca le avvertenze nell'ordine in cui il motore le scrive, lo stesso
della loro eco in `explain.conventions`. Vengono prima le avvertenze del metodo, nell'ordine del
calcolo; in un'analisi sono la sua `FM-W016`, poi le avvertenze della valutazione di
`base_input` (o, per il goal seek, della valutazione risolta). Vengono per ultime le avvertenze
sull'input così come è stato letto: ogni `FM-W001` (un decimale arrotondato alla lettura) in
ordine di pointer, poi ogni `FM-W017` (un valore arrivato come float binario) in ordine di
pointer, con gli indici degli array confrontati come numeri. L'ordine non dipende dall'ordine
delle chiavi dell'input.

## Catalogo

| Codice | Condizione | Metodi | Radice del pointer | Schema del messaggio | Aggiunta in |
| --- | --- | --- | --- | --- | --- |
| `FM-W001` | Un decimale dell'input ha più cifre di quante un decimale ne contenga ed è arrotondato alla lettura (ADR 0014 G7) | tutti i metodi | input: il campo | `` `{pointer}` has more digits than a decimal holds: given {text}, used {used}, rounded half away from zero ... `` | 0.1.0 |
| `FM-W002` | Il valore base è negativo, quindi una crescita positiva rende i valori più negativi | `series.project` | input: `/base_value` | `the base value {v} is negative, ...` | 0.1.0 |
| `FM-W003` | Una crescita è applicata a un flusso negativo, una volta per periodo | `dcf.fcff`, `dcf.fcfe` | input: `/stages/{i}`, lo stadio generato | `period {k}: growth {g} applied to a negative flow ({prev}); ...` | 0.1.0 |
| `FM-W004` | La crescita perpetua nominale è inferiore all'inflazione attesa della fase terminale | `dcf.fcff`, `dcf.fcfe` | input: la crescita della tecnica, `/terminal/{id}/growth` o `/terminal/tv.h_model/long_term_growth` | `the perpetual growth {g} (nominal) is below the expected inflation of the terminal phase {pi}: ...` | 0.1.0 |
| `FM-W005` | Nessun valore terminale è incluso, per scelta o per valore predefinito | `dcf.fcff`, `dcf.fcfe` | input: `/terminal` | `no terminal value is included: the value covers the {n} explicit flows only; ...` | 0.1.0 |
| `FM-W006` | Un valore terminale fornito è negativo | `dcf.fcff`, `dcf.fcfe` | input: `/terminal/given/value` | `the given terminal value {v} is negative (for example a decommissioning cost); ...` | 0.1.0 |
| `FM-W007` | Il valore terminale è negativo, quindi il suo peso non è significativo | `dcf.fcff`, `dcf.fcfe` | envelope: `/value/terminal_value_weight`; in un'analisi diversa dal goal seek, `null` (il risultato interno non è restituito) | `the terminal value is negative, so the terminal value weight is not meaningful ...` | 0.1.0 |
| `FM-W008` | La crescita dell'ultimo periodo esplicito è diversa dalla crescita perpetua | `dcf.fcff`, `dcf.fcfe` | input: la crescita della tecnica, come per `FM-W004` | `the growth of the last explicit period ({a}) differs from the perpetual growth of the terminal value ({g})` | 0.1.0 |
| `FM-W009` | Un prezzo a una data (multiplo di uscita, liquidazione, valore residuo, valore fornito) è attualizzato a N − 0,5 | `dcf.fcff`, `dcf.fcfe` | input: `/terminal_discount_time` | `{what}; discounting it at N − 0.5 brings it forward half a period; ...` | 0.1.0 |
| `FM-W010` | Con la tempistica a metà periodo un valore terminale a crescita perpetua è attualizzato a N | `dcf.fcff`, `dcf.fcfe` | input: `/terminal_discount_time` | `under mid-period timing a perpetuity-growth terminal value is commonly discounted at N − 0.5, ...` | 0.1.0 |
| `FM-W011` | L'equity value è negativo | `dcf.fcff`, `dcf.fcfe` | envelope: `/value/equity_value`; in un'analisi diversa dal goal seek, `null` | `the equity value is negative ({v}): {reason}; it is reported as computed, not floored at zero` | 0.1.0 |
| `FM-W012` | Posizione finanziaria netta non fornita: si sottrae zero | `dcf.fcff` | input: `/net_financial_position` | `no net financial position given (...): zero is subtracted, so the equity value equals the enterprise value...` | 0.1.0 |
| `FM-W013` | Altre voci di una posizione finanziaria netta non fornite: la determina il valore predefinito `[]` | `ratios.capital_structure`, `ratios.debt_service` | input: `/{closing,opening}/net_financial_position/other_items` | `no other items of the net financial position given (...): the net financial position is gross debt less cash; ...` | 0.1.0 |
| `FM-W014` | Saldo della riserva non fornito: il LLCR non considera alcun conto di riserva | `ratios.debt_service` | input: `/debt_schedule/reserve_balance` | `no reserve balance given (...): nothing is added to the present value of CFADS, ...` | 0.1.0 |
| `FM-W015` | Un B10 c o un B10 d diverso da zero è collocato sotto l'EBITDA dalla classificazione predefinita `operating` | `statements.reorganise` | input: `/treatments/write_downs_of_fixed_assets` o `/treatments/write_downs_of_receivables` | `year {name}: {item} ({v}) is in {line}, below EBITDA, by the default ...` | 0.1.0 |
| `FM-W016` | Il metodo valutato da un'analisi è in anteprima (`preview`, ADR 0012) | `analysis.*` | input: `/method` | `Evaluated method {id} is preview: its results have not been independently verified ...` | 0.1.0 |
| `FM-W017` | Un valore dell'input è arrivato all'adapter come numero in virgola mobile binaria: una libreria client ha letto un numero JSON come float, quindi le cifre oltre circa 17 possono essere perse | tutti i metodi, solo dove una libreria esterna converte i numeri JSON in float: function calling con un dizionario, il server MCP, LangChain; mai per un float dato all'SDK Python (vedi sotto) | input: il campo | `` `{pointer}` arrived as a binary floating-point number ({value}): a JSON number may have lost digits before Fairmark read it; ... `` | 0.1.0 |

## I float e il limite di FM-W017

Un `float` Python dato all'SDK (`fairmark.run`, `fairmark.run_json`, gli helper) è inviato con la
sua rappresentazione decimale più breve, il letterale scritto dall'analista: `0.095` resta
`0.095`, senza avvertenza. Un float che è il risultato di un calcolo in Python (per esempio
`0.1 + 0.2`, che vale `0.30000000000000004`) porta artefatti binari che Fairmark non può
rilevare: per i valori calcolati passare `Decimal` o stringhe. `FM-W017` è emessa solo dove una
libreria esterna converte i numeri JSON in float prima che Fairmark li veda: function calling
con un dizionario (`fairmark.tools.dispatch`), il server MCP e LangChain, che passano al motore
i pointer di quei valori.

## Avvertenze in un'analisi

Un'analisi (`analysis.*`) riporta le avvertenze della valutazione di `base_input` una sola volta,
qualunque sia il numero di celle, scenari o barre, con i pointer dell'input sotto `/base_input`,
dopo la propria `FM-W016`. `analysis.goal_seek` riporta invece le avvertenze della valutazione
risolta, con i pointer del suo risultato sotto `/value/inner_value`. Una cella, uno scenario o
una barra che fallisce mantiene il suo errore compatto `{code, message}` e non aggiunge
avvertenze.

## Come è mantenuto il catalogo

I codici sono dichiarati una sola volta nel core (`crates/fairmark-core/src/diagnostics.rs`) e
scritti solo tramite la sua API delle avvertenze. Un test fallisce quando un codice dichiarato dal
motore manca da questa pagina, quando un codice di questa pagina non è dichiarato o non ha un
punto di emissione, o quando il codice di produzione scrive una convenzione che inizia con
`warning` al di fuori dell'API.
