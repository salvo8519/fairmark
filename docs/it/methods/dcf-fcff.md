---
source: methods/dcf-fcff.md
source_hash: 58f9418f017e
---
# DCF, flussi di cassa operativi (`dcf.fcff`)

Valuta l'attività operativa attualizzando i flussi di cassa operativi disponibili (FCFF) al
costo medio ponderato del capitale (WACC) lungo da uno a tre stage (fasi del periodo di
previsione esplicita), aggiunge il valore terminale e passa dal value of operations (valore
delle attività operative) all'enterprise value e all'equity value. Ogni scelta che modifica il
valore è un input esplicito: gli stage e il modo in cui se ne ottengono i flussi, i termini dei
flussi e dei tassi, la collocazione temporale dei flussi, la tecnica del valore terminale e il
momento in cui questo viene attualizzato.

Versione del metodo: 3. La versione 3 ha sostituito l'unica lista di flussi della versione 2
con gli stage, ha reso obbligatori la collocazione temporale dei flussi e la tecnica del valore
terminale e ha rinominato `enterprise_value` della versione 2 (solo valori attuali) in
`value_of_operations`; gli esempi della versione 2 danno gli stessi numeri nella versione 3.

## Formula

$$
V_{op} = \sum_{k=1}^{N} FCFF_k \, DF_k + TV_N \, DF_{TV}
\qquad
DF_k = \prod_{i=1}^{k} (1 + r_i)^{-1}
$$

$$
EV = V_{op} + \sum \text{attività non operative}
\qquad
Equity = EV - \text{posizione finanziaria netta}
$$

L'orizzonte esplicito N è la somma dei periodi degli stage; un periodo è un anno. I flussi di
ciascuno stage sono:

| Proiezione | Flusso del periodo k dello stage (m periodi) |
| --- | --- |
| `given` | come forniti, uno per periodo |
| `constant_growth` | $CF_k = CF_{k-1} (1 + g)$ |
| `linear_decline` | $CF_k = CF_{k-1} (1 + g_k)$, con $g_k = g_{from} + (g_{to} - g_{from}) \, k / m$ |

Uno stage generato parte dall'ultimo flusso dello stage precedente oppure, se è il primo stage,
dal base flow $CF_0$ (flusso dell'anno base). Nella riduzione lineare della crescita
(`linear_decline`), $g_{from}$ è la crescita prima dello stage e $g_{to}$ è raggiunta
nell'ultimo periodo: "dal 10% al 3% in 5 anni" dà 8,6%, 7,2%, 5,8%, 4,4% e 3,0%.

Con la convenzione di metà periodo il flusso del periodo k è attualizzato mezzo periodo prima,
$DF_k = DF_{k-1} (1 + r_k)^{-0.5}$. Il valore terminale
$TV_N = CF_{N+1} / (r_{TV} - g)$ è calcolato da [`tv.gordon`](tv-gordon.md) e attualizzato
all'orizzonte, $DF_{TV} = DF_N$, oppure, con la convenzione di metà periodo e se scelto, mezzo
periodo prima, $DF_{TV} = DF_N (1 + r_N)^{0.5}$; lo spostamento di mezzo periodo usa il tasso
del periodo N, $r_N$, non il tasso terminale.

I valori attuali si calcolano dividendo ogni importo per il suo fattore di capitalizzazione
$\prod_{i \le k} (1 + r_i)$ (con la convenzione di metà periodo, $\prod_{i < k} (1 + r_i) \,
(1 + r_k)^{0.5}$): è la stessa grandezza che si ottiene moltiplicando per $DF_k$, ma conserva
tutte le cifre significative anche su orizzonti lunghi; i fattori di sconto sono riportati come
calcolati da [`tvm.present_value`](tvm-present-value.md).

## Esempio

Un piano con tre anni di flussi forniti, poi quattro anni in cui la crescita si riduce in modo
lineare dal 12% al 3%, poi una crescita stabile del 3%:

```python
import fairmark as fm

r = fm.dcf.fcff(
    [
        fm.dcf.given([100, 112, 125]),
        fm.dcf.linear_decline(4, from_growth="0.12", to_growth="0.03"),
    ],
    cash_flow_terms="nominal",
    discount_rate_terms="nominal",
    wacc="0.095",
    timing="end_of_period",
    terminal=fm.dcf.gordon("0.03", first_flow="last_flow_grown"),
    net_financial_position={"gross_debt": 500, "cash": 120},
    non_operating_assets=[{"label": "Land held for sale", "value": 60}],
)
print([row["growth"] for row in r.periods][3:])  # 0.0975, 0.075, 0.0525, 0.03
print(r.value_of_operations)    # 1985.96014516242602644247722...
print(r.enterprise_value)       # value of operations + 60
print(r.equity_value)           # enterprise value − (500 − 120)
print(r.terminal_value_weight)  # share of the value of operations from the terminal value
assert r.enterprise_value - r.value_of_operations == 60
```

Con un tasso per periodo, si forniscono `wacc_per_period` (un tasso per ogni periodo
dell'orizzonte esplicito) e `terminal_wacc` al posto di `wacc`. Con la convenzione di metà
periodo si indica quando il valore terminale è attualizzato:

```python
r = fm.dcf.fcff(
    [fm.dcf.constant_growth(3, "0.10"), fm.dcf.linear_decline(3, "0.10", "0.025")],
    base_flow=80,
    cash_flow_terms="real",
    discount_rate_terms="real",
    wacc_per_period=["0.11", "0.11", "0.105", "0.10", "0.095", "0.09"],
    terminal_wacc="0.09",
    timing="mid_period",
    terminal_discount_time="horizon",
    terminal=fm.dcf.gordon("0.025", first_flow="last_flow_grown"),
)
print(r.terminal["discount_time"])  # 6
```

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `stages` | Da uno a tre stage, in ordine: `{"periods", "projection", ...}` con `flows` (`given`), `growth` (`constant_growth`) oppure `from_growth` e `to_growth` (`linear_decline`) | obbligatorio |
| `base_flow` | FCFF del periodo base $CF_0$, quando il primo stage è generato | obbligatorio per un primo stage generato; rifiutato altrimenti |
| `cash_flow_terms` | `nominal` oppure `real`: termini di ogni importo e di ogni tasso di crescita | obbligatorio |
| `discount_rate_terms` | `nominal` oppure `real`: termini dei tassi di attualizzazione; devono coincidere con `cash_flow_terms` | obbligatorio |
| `wacc` | Un unico WACC per tutti i periodi e per il valore terminale, frazione decimale | questo oppure `wacc_per_period` |
| `wacc_per_period` | Un WACC per ogni periodo dell'orizzonte esplicito, frazioni decimali | questo oppure `wacc` |
| `terminal_wacc` | WACC della fase terminale | obbligatorio con `wacc_per_period`; rifiutato con `wacc` |
| `timing` | `end_of_period` oppure `mid_period` | obbligatorio |
| `terminal_discount_time` | `horizon` (t = N) oppure `mid_period` (t = N − 0.5) | obbligatorio con `mid_period`; rifiutato con `end_of_period` |
| `terminal` | `{"tv.gordon": {"growth", "first_flow", "normalised_next_flow", "growth_cap"}}`; `first_flow` è `last_flow_grown` oppure `normalised_next_flow` | obbligatorio |
| `net_financial_position` | `{"gross_debt", "cash", "other_items"}` come in [`ratios.debt_service`](ratios-debt-service.md), positiva per indebitamento netto | nessuna, riportato |
| `non_operating_assets` | Lista di `{"label", "value"}`, valori pari a zero o positivi | nessuna, riportato |

## Output

| Campo | Significato |
| --- | --- |
| `value_of_operations` | Valore attuale dei FCFF espliciti più valore attuale del valore terminale |
| `enterprise_value` | Value of operations più le attività non operative |
| `equity_value` | Enterprise value meno la posizione finanziaria netta |
| `pv_explicit_period` | Valore attuale dei FCFF espliciti |
| `terminal_value_weight` | Valore attuale del valore terminale diviso per il value of operations |
| `periods` | Per periodo: stage, proiezione, crescita applicata, flusso, tasso di attualizzazione fornito, tasso usato, fattore di sconto, valore attuale |
| `terminal` | Tecnica e sua versione, tasso terminale, valore in N, momento di attualizzazione, fattore, valore attuale, output proprio della tecnica |
| `bridge` | Ogni voce dal value of operations all'equity value, con le descrizioni |

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Approccio | asset side (enterprise value): FCFF attualizzati al WACC | fissato dal metodo |
| Proiezione dello stage | `given`, `constant_growth`, `linear_decline` | obbligatoria per ogni stage |
| Passo della riduzione lineare della crescita | $g_k = g_{from} + (g_{to} - g_{from}) \, k / m$, $g_{to}$ raggiunta nell'ultimo periodo | fissa |
| Tasso di attualizzazione | un unico tasso (usato anche per il valore terminale), oppure uno per periodo con un tasso terminale | obbligatorio |
| Termini di flussi e tassi | `nominal`, `real`; termini diversi sono rifiutati | obbligatori, entrambi |
| Collocazione temporale dei flussi | `end_of_period`, `mid_period` | obbligatoria |
| Terminal discounting time (momento di attualizzazione del valore terminale) | `horizon`, `mid_period` (solo con la convenzione di metà periodo) | obbligatorio |
| Tecnica del valore terminale | `tv.gordon` al tasso terminale, con approccio asset side | obbligatoria |
| Primo flusso terminale | ultimo flusso fatto crescere di g, oppure normalised flow (flusso normalizzato) in N + 1 | obbligatorio |
| Voci del passaggio all'equity value | misurate alla data di valutazione come fornite; nessuna voce significa nessuna | nessuna |

Le avvertenze sono riportate in `explain.conventions` e non comportano mai il rifiuto
dell'input: una crescita applicata a un flusso negativo; una crescita dell'ultimo periodo
esplicito diversa dalla crescita perpetua del valore terminale; con la convenzione di metà
periodo, un valore terminale di Gordon attualizzato all'orizzonte N, mentre la prassi lo
attualizza di solito in N − 0.5; un equity value negativo, riportato come calcolato e mai
riportato a zero. Un `terminal_value_weight`
elevato segnala che la valutazione dipende soprattutto dalle ipotesi di lungo periodo.

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | Un campo è mancante, sconosciuto o non usato dall'opzione scelta; nessuno stage o più di tre; stage che sommano più di 10.000 periodi; una crescita pari o inferiore a −100%; un tasso terminale insieme a un unico tasso, oppure nessuno con tassi per periodo; un momento di attualizzazione del valore terminale mancante con la convenzione di metà periodo o fornito con quella di fine periodo; un'attività non operativa negativa o con descrizione vuota |
| `FM-E003` | Un tasso di attualizzazione è pari o inferiore a −100%; il messaggio indica il campo e il periodo |
| `FM-E004` | La crescita perpetua è pari o superiore al tasso terminale |
| `FM-E008` | Overflow aritmetico (input molto al di fuori di un intervallo realistico) |
| `FM-E024` | `cash_flow_terms` e `discount_rate_terms` sono diversi |
| `FM-E025` | Uno stage `given` con più o meno flussi dei suoi periodi, tassi per periodo non uno per periodo, un base flow mancante per un primo stage generato o fornito per uno `given` |

Gli errori di validazione indicano il campo con il suo JSON Pointer, per esempio
`/stages/1/flows`, e la correzione da apportare; gli errori rilevati durante la lettura
dell'input (un campo mancante o sconosciuto, un tipo errato) indicano invece la riga e la
colonna dell'input.

## Precisione

I fattori di sconto sono calcolati da `tvm.present_value` (`tvm::period_discount_factors`): a
fine periodo con divisioni esatte, a metà periodo con una radice quadrata decimale. I valori attuali
dividono per il fattore di capitalizzazione e conservano quindi le cifre significative anche su
orizzonti lunghi: rispetto a un calcolo indipendente a 50 cifre, il value of operations concorda
entro circa $10^{-24}$ in termini relativi o meglio, a fine e a metà periodo, anche su 300-1.000
periodi. I casi di riferimento derivati dal calcolo indipendente sono verificati entro
$10^{-20}$.

## Verificato con

- Damodaran, *Investment Valuation*, 2a edizione, capitolo 15, Illustration 15.3 (Amgen, FCFF
  in tre fasi con un costo del capitale che cambia ogni anno): valore attuale del periodo
  esplicito 8.327, valore delle attività operative 39.161, equity value 40.867 e valore
  terminale 81.364 sono riprodotti entro 2 (valore terminale: 20) dai valori pubblicati, che
  derivano da input arrotondati; i file dei casi di riferimento scompongono le differenze.
  Queste tolleranze sono in attesa di approvazione del maintainer.
- Un'implementazione indipendente a 50 cifre, `scripts/reference_dcf.py`, per ogni caso di
  riferimento (golden case) in `tests/golden/dcf.fcff/`, e le proprietà: uno stage a crescita
  costante equivale agli stessi flussi forniti; una riduzione lineare con crescite uguali
  equivale alla crescita costante; i fattori di sconto coincidono con quelli di
  [`tvm.present_value`](tvm-present-value.md).
- Il VAN di numpy-financial sui flussi espliciti più il valore di Gordon, in virgola mobile
  binaria (concordanza delle convenzioni, non delle cifre).
