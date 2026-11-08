---
source: methods/dcf-fcff.md
source_hash: 76ea7bfef5f5
---
# DCF, flussi di cassa operativi (`dcf.fcff`)

Valuta l'attività operativa attualizzando i flussi di cassa operativi disponibili (FCFF) al
costo medio ponderato del capitale (WACC) lungo da uno a tre stage (fasi del periodo di
previsione esplicita), aggiunge il valore terminale e passa dal value of operations (valore
delle attività operative) all'enterprise value e all'equity value. Ogni scelta che modifica il
valore è un input: gli stage e il modo in cui se ne ottengono i flussi, i termini dei flussi e
dei tassi, la collocazione temporale dei flussi, il valore terminale e il momento in cui questo
viene attualizzato. Una scelta non fornita assume un valore predefinito documentato, riportato
nel risultato.

Versione del metodo: 3. La versione 3 ha sostituito l'unica lista di flussi della versione 2
con gli stage e ha rinominato `enterprise_value` della versione 2 (solo valori attuali) in
`value_of_operations`; gli esempi della versione 2 danno gli stessi numeri nella versione 3. Dalla
Decision 24 di SPEC-0001 (issue #197) sono obbligatori soltanto i flussi e il WACC: una modifica
additiva, ogni input precedente dà lo stesso risultato.

## Input minimo

Bastano una lista di FCFF e un WACC, per esempio flussi letti da un business plan o da un
documento:

```python
import fairmark as fm

r = fm.dcf.fcff(flows=[120, 135, 150, 160, 170], wacc="0.09")
print(r.value_of_operations)  # 563.38243366056148746628356553
print([d["field"] for d in r.meta["defaults_applied"]])
```

Lo stesso input in JSON, per la CLI, MCP o il function calling:

```json
{"stages": [{"flows": ["120", "135", "150", "160", "170"]}], "wacc": "0.09"}
```

Il motore assegna a ogni campo non fornito il suo valore predefinito documentato, una sola
volta, prima del calcolo:

| Campo | Valore predefinito | Motivo |
| --- | --- | --- |
| `stages[].projection` | `given`, per uno stage con `flows` o `lines` | uno stage con flussi è uno stage di flussi forniti |
| `stages[].periods` | il numero di `flows` o `lines` di uno stage `given` | i flussi indicano i periodi; se sono forniti entrambi devono coincidere (`FM-E025`) |
| `cash_flow_terms`, `discount_rate_terms` | `nominal` | piani e tassi di attualizzazione sono di solito espressi a prezzi correnti |
| `timing` | `end_of_period` | ogni flusso alla fine del suo periodo, come in [`tvm.npv`](tvm-npv.md) |
| `terminal` | `none`: nessun valore terminale | il valore copre soltanto i flussi forniti; fornire un valore terminale per aggiungerlo |

Ogni valore predefinito applicato è elencato in `meta.defaults_applied` (il JSON pointer del
campo, il valore assegnato e il motivo) e indicato per primo in `explain.conventions`, con il
prefisso "default:". Un campo fornito esplicitamente prevale sempre e non è elencato. Il valore
predefinito si ottiene omettendo il campo: un `null` esplicito per uno di questi campi è
rifiutato con `FM-E001`. Senza
valore terminale la valutazione copre soltanto i flussi forniti, un periodo di detenzione pari
al loro numero: il valore terminale, il suo valore attuale e il `terminal_value_weight` sono
zero, e un'avvertenza segnala che il valore terminale non è incluso. Includere un valore di
uscita nell'ultimo flusso, fornirne uno come importo (`given`) oppure calcolarlo con una
tecnica. Con questi valori predefiniti il value of operations è il valore attuale netto (VAN,
NPV) dei flussi al WACC, lo stesso di `tvm.npv` con un primo flusso pari a zero in t = 0.

`meta.input_hash` è calcolato sull'input con i valori predefiniti assegnati: l'input minimo e
lo stesso input con ogni valore predefinito scritto per esteso hanno lo stesso hash e lo stesso
valore. Quando i termini assumono il valore predefinito e soltanto uno dei due è fornito come
`real`, l'input è rifiutato con `FM-E024`, il cui messaggio indica quale campo ha assunto il
valore predefinito.

## Formula

$$
V_{op} = \sum_{k=1}^{N} FCFF_k \, DF_k + TV_N \, DF_{TV}
\qquad
DF_k = \prod_{i=1}^{k} (1 + r_i)^{-1}
$$

$$
EV = V_{op} + \sum \text{attività non operative}
$$

$$
Equity = EV - \text{posizione finanziaria netta} - \sum \text{debt-like items} - \sum
\text{partecipazioni di minoranza}
$$

L'orizzonte esplicito N è la somma dei periodi degli stage; un periodo è un anno. I flussi di
ciascuno stage sono:

| Proiezione | Flusso del periodo k dello stage (m periodi) |
| --- | --- |
| `given` | come forniti in `flows`, uno per periodo, oppure costruiti dalle voci in `lines` come in [`dcf.free_cash_flows`](dcf-free-cash-flows.md) |
| `constant_growth` | $CF_k = CF_{k-1} (1 + g)$ |
| `linear_decline` | $CF_k = CF_{k-1} (1 + g_k)$, con $g_k = g_{from} + (g_{to} - g_{from}) \, k / m$ |

Uno stage generato parte dall'ultimo flusso dello stage precedente oppure, se è il primo stage,
dal base flow $CF_0$ (flusso dell'anno base). Nella riduzione lineare della crescita
(`linear_decline`), $g_{from}$ è la crescita prima dello stage e $g_{to}$ è raggiunta
nell'ultimo periodo: "dal 10% al 3% in 5 anni" dà 8,6%, 7,2%, 5,8%, 4,4% e 3,0%.

Uno stage `given` può ricevere i flussi come voci invece che come importi: `lines` contiene un
oggetto per periodo con `ebit`, `depreciation_and_amortisation`, `capital_expenditure`,
`change_in_net_working_capital`, `other_adjustments` facoltative e, nella forma a importi,
`operating_taxes`. Il FCFF di ogni periodo è costruito come in
[`dcf.free_cash_flows`](dcf-free-cash-flows.md), con le operating taxes (imposte sull'EBIT) da
`notional_tax_rate` e `negative_ebit`, oppure da `operating_taxes` in ogni voce. Oneri
finanziari e net borrowing sono rifiutati: il FCFF è prima dei flussi finanziari; il DCF
equity side [`dcf.fcfe`](dcf-fcfe.md) li accetta e attualizza il FCFE al costo del capitale
proprio. Ogni passo della costruzione è nell'explain, con il prefisso "Build-up:", e le sue
convenzioni indicano la versione della costruzione (`dcf.free_cash_flows v2`). Con `zero` la
versione 2 limita lo scudo fiscale sugli oneri finanziari al reddito imponibile disponibile; il
FCFF esclude gli oneri finanziari, quindi flussi e valori di `dcf.fcff` non cambiano. Il
beneficio fiscale del debito è invece nel WACC: un costo del debito Kd × (1 − t) presuppone lo
scudo pieno ogni anno. Negli anni con EBIT negativo o basso e trattamento `zero` l'aliquota
effettiva sugli oneri finanziari è più bassa, quindi si fornisca `wacc_per_period` con un WACC
che ne tenga conto in quegli anni, oppure si motivi l'ipotesi di scudo pieno; Fairmark non
rettifica il tasso.

Con la convenzione di metà periodo il flusso del periodo k è attualizzato mezzo periodo prima,
$DF_k = DF_{k-1} (1 + r_k)^{-0.5}$. Il valore terminale $TV_N$ è calcolato dalla tecnica
scelta (si veda "Tecniche del valore terminale" più avanti), per esempio
$TV_N = CF_{N+1} / (r_{TV} - g)$ con [`tv.gordon`](tv-gordon.md), e attualizzato
all'orizzonte, $DF_{TV} = DF_N$, oppure, con la convenzione di metà periodo e se scelto, mezzo
periodo prima, $DF_{TV} = DF_N (1 + r_{TV})^{0.5}$; lo spostamento di mezzo periodo usa il
tasso terminale $r_{TV}$, che con un unico tasso di attualizzazione è il tasso di ogni periodo.

I valori attuali si calcolano dividendo ogni importo per il suo fattore di capitalizzazione
$\prod_{i \le k} (1 + r_i)$ (con la convenzione di metà periodo, $\prod_{i < k} (1 + r_i) \,
(1 + r_k)^{0.5}$): è la stessa grandezza che si ottiene moltiplicando per $DF_k$, ma conserva
tutte le cifre significative anche su orizzonti lunghi; i fattori di sconto sono riportati come
calcolati da [`tvm.present_value`](tvm-present-value.md).

## Tecniche del valore terminale

L'oggetto `terminal` indica esattamente un'opzione per il valore terminale: una tecnica con
l'identificativo del suo metodo, un valore terminale fornito dall'analista, oppure nessuno.

| Opzione | Forma | Valore terminale in N |
| --- | --- | --- |
| Una tecnica | `{"tv.gordon": {...}}` e le altre indicate sotto | calcolato dalla tecnica |
| Fornito dall'analista | `{"given": {"value": "2000"}}` | l'importo come fornito: il value of operations in N, con approccio asset side, nei termini dei flussi di cassa; di qualunque segno, un importo negativo (per esempio un costo di smantellamento) è riportato come avvertenza, con una seconda avvertenza che il `terminal_value_weight` non è allora significativo |
| Nessuno | `{"none": {}}`, il valore predefinito | zero; il valore copre soltanto i flussi espliciti, con un'avvertenza; nulla è attualizzato, quindi momento e fattore di attualizzazione sono `null` |

Un valore terminale fornito è attualizzato come quello di una tecnica: con il fattore $DF_N$,
oppure, con la convenzione di metà periodo, in N − 0.5 con lo spostamento di mezzo periodo al
tasso terminale quando `terminal_discount_time` è `mid_period`. Il tasso terminale serve quindi
soltanto a una tecnica o a quello spostamento: negli altri casi `terminal_wacc` e
`terminal_inflation` sono rifiutati e il tasso terminale è riportato come `null`. Senza valore
terminale è rifiutato anche `terminal_discount_time`, perché non c'è nulla da attualizzare. Un
valore terminale fornito attualizzato in N − 0.5 riceve la stessa avvertenza di un multiplo di
uscita: è il valore terminale dell'analista all'orizzonte N, anticipato di mezzo periodo.

Ogni tecnica è calcolata dal proprio metodo, sugli stessi parametri, al tasso terminale $r_{TV}$
(`wacc`, oppure `terminal_wacc` con tassi per periodo), e il risultato riporta in `terminal`
l'identificativo del metodo, la sua versione e il suo output. Gli importi sono nei termini dei
flussi di cassa.

| Tecnica | Parametri all'interno del DCF | Valore terminale in N |
| --- | --- | --- |
| [`tv.gordon`](tv-gordon.md) | `growth`, `first_flow`, `normalised_next_flow`, `growth_cap` | $CF_{N+1} / (r_{TV} - g)$, con $CF_{N+1} = CF_N (1 + g)$ oppure un flusso normalizzato |
| [`tv.h_model`](tv-h-model.md) | `short_term_growth`, `long_term_growth`, `half_life`, `first_flow`, `normalised_base_flow`, `growth_cap` | $CF_N [(1 + g_l) + H (g_s - g_l)] / (r_{TV} - g_l)$ sull'ultimo flusso esplicito o su un flusso base normalizzato in N |
| [`tv.value_driver`](tv-value-driver.md) | `nopat_next`, `growth`, `ronic`, `growth_cap` | $NOPAT_{N+1} (1 - g / RONIC) / (r_{TV} - g)$ |
| [`tv.exit_multiple`](tv-exit-multiple.md) | `multiple`, `metric` (`ebitda`, `ebit`, `revenue`), `metric_value`, `next_flow`, `growth_cap` | multiplo × grandezza in N; con `next_flow`, la crescita implicita $r_{TV} - CF_{N+1} / TV_N$ |
| [`tv.limited_horizon`](tv-limited-horizon.md) | `flow`, `periods`, `growth`, `residual_value`, `growth_cap` | i flussi dei periodi da N + 1 a N + n e il valore residuo in N + n, attualizzati in N al tasso $r_{TV}$ |
| [`tv.normalised_income`](tv-normalised-income.md) | `income`, `basis` (`enterprise`), `growth`, `growth_cap` | $R / (r_{TV} - g)$: il tasso di capitalizzazione è il tasso terminale |
| [`tv.liquidation`](tv-liquidation.md) | `assets`, `liabilities`, `liquidation_costs`, `tax_on_gains`, `basis` (`enterprise`) | valori di realizzo − passività − costi di liquidazione − imposte sulle plusvalenze |

La tecnica deve seguire l'approccio asset side (enterprise value) di `dcf.fcff`: un multiplo di
uscita (exit multiple) sugli utili (P/E) o sul valore contabile del capitale proprio (P/BV),
oppure un reddito normalizzato o una liquidazione dichiarati con approccio equity side, è
rifiutato con `FM-E019`.
`tv.gordon` e `tv.h_model` fanno crescere l'ultimo flusso esplicito, quindi il primo flusso
terminale è una scelta obbligatoria: per `tv.gordon`, `last_flow_grown` oppure un flusso
normalizzato in N + 1 (`normalised_next_flow`); per `tv.h_model`, `last_flow_grown` oppure un
flusso base normalizzato in N (`normalised_base_flow`) che sostituisce l'ultimo flusso
esplicito nella formula standard. All'interno
del DCF i campi `growth` e `residual_value` di `tv.limited_horizon` e il campo `growth` di
`tv.normalised_income` sono obbligatori (indicare 0 se assenti). `tv.normalised_income` è
capitalizzato al tasso terminale, come ogni tecnica che usa un tasso: un `capitalisation_rate`
separato non è accettato all'interno del DCF.

## Termini nominali e reali

`cash_flow_terms` indica se ogni importo e ogni tasso di crescita dell'input è espresso in
termini nominali (inflazione attesa inclusa) oppure in termini reali (inflazione attesa
esclusa); `discount_rate_terms` indica lo stesso per i tassi di attualizzazione. Entrambi sono
obbligatori. Quando coincidono, i tassi sono usati come forniti. Quando sono diversi:

- senza inflazione attesa l'input è rifiutato con `FM-E024`: attualizzare flussi reali a un
  tasso nominale, o flussi nominali a un tasso reale, altera il valore;
- con l'inflazione attesa ogni tasso di attualizzazione, compreso il tasso terminale, è
  convertito nei termini dei flussi di cassa con la relazione di Fisher esatta, calcolata da
  [`rates.real_nominal`](rates-real-nominal.md):

$$
(1 + r_{nominal}) = (1 + r_{real}) \, (1 + i)
\qquad
r_{real} = \frac{1 + r_{nominal}}{1 + i} - 1
\qquad
r_{nominal} = (1 + r_{real}) (1 + i) - 1
$$

I flussi di cassa e i tassi di crescita non sono mai convertiti: solo i tassi passano nei
termini dei flussi. L'inflazione attesa è un unico tasso per tutti i periodi e per la fase
terminale (`inflation`), oppure un tasso per ogni periodo dell'orizzonte esplicito
(`inflation_per_period`) con l'inflazione della fase terminale (`terminal_inflation`); il tasso
del periodo k è convertito con l'inflazione del periodo k, il tasso terminale con l'inflazione
della fase terminale. Ogni periodo riporta il tasso fornito (`discount_rate`), l'inflazione e il
tasso usato (`rate_used`), e l'explain mostra ogni conversione con i suoi numeri.

Ogni volta che l'inflazione è fornita, anche con termini uguali (quando non converte nulla), la
crescita perpetua della tecnica del valore terminale (`growth`, oppure `long_term_growth` per
`tv.h_model`) è espressa anche negli altri termini con l'inflazione della fase terminale, in
`terms`; con flussi nominali, una crescita perpetua inferiore a quell'inflazione è riportata
come avvertenza, perché i flussi diminuiscono in termini reali dopo l'orizzonte. Un multiplo di
uscita, un orizzonte finito e una liquidazione non indicano una crescita perpetua, e le
convenzioni riportano che nessuna crescita è riespressa.

Un piano in termini reali attualizzato a un WACC nominale del 10% con un'inflazione attesa del
2%:

```python
import fairmark as fm

r = fm.dcf.fcff(
    [fm.dcf.given([100, 104, 108, 112])],
    cash_flow_terms="real",
    discount_rate_terms="nominal",
    wacc="0.10",
    inflation="0.02",
    timing="end_of_period",
    terminal=fm.dcf.gordon("0.01", first_flow="last_flow_grown"),
)
print(r.periods[0]["rate_used"])  # (1 + 0.10) / (1 + 0.02) − 1 = 0.07843137...
print(r.terminal["rate"])  # lo stesso tasso reale per il valore terminale
print(r.value["terms"]["perpetual_growth_other_terms"])  # 1.01 × 1.02 − 1 = 0.0302 nominale
print(r.value_of_operations)  # 1573.18265943050768644554905...
```

Lo stesso valore si ottiene fornendo direttamente il tasso reale con
`discount_rate_terms="real"`. I flussi reali sono espressi ai prezzi della data di valutazione
(t = 0). Con la convenzione di fine periodo il valore coincide anche con quello dei flussi
riespressi in termini nominali, ogni flusso reale per l'indice cumulato dell'inflazione
$\prod_{i \le k} (1 + i_i)$, attualizzati al tasso nominale con la crescita nominale; con la
convenzione di metà periodo l'equivalenza vale con l'indice calcolato in t = k − 0.5,
$\prod_{i < k} (1 + i_i) \, (1 + i_k)^{0.5}$. Quando il tasso terminale è stato convertito, la
convenzione sul tasso della tecnica del valore terminale indica "converted" (convertito) invece
di "given" (fornito).

Aspetti della prassi che il motore non verifica:

- Un WACC reale deve essere il WACC nominale dopo le imposte deflazionato con la relazione di
  Fisher: lo scudo fiscale si applica agli interessi nominali, quindi il WACC va calcolato in
  termini nominali e convertito dal motore (o deflazionato dall'utente), invece di applicare
  l'aliquota fiscale a un costo del debito reale.
- Con voci di costruzione (`lines`) in termini reali, le imposte calcolate sull'EBIT reale
  sottostimano le imposte effettivamente pagate, perché gli ammortamenti fiscali sono sul costo
  storico e non sono indicizzati all'inflazione; quando l'inflazione è rilevante è preferibile
  un piano in termini nominali (come raccomanda Damodaran). È una nota di documentazione:
  nessuna avvertenza è riportata.
- Una rendita perpetua a crescita costante (`tv.gordon`) dà lo stesso valore in termini reali
  e in termini nominali. L'H-model approssima una crescita decrescente e il value driver
  sottrae un reinvestimento g / RONIC, quindi una valutazione reale e una nominale della stessa
  impresa differiscono leggermente con `tv.h_model` e `tv.value_driver`: è una proprietà delle
  tecniche, non della conversione. I loro parametri, compreso il RONIC (rendimento del nuovo
  capitale investito), sono nei termini dei flussi di cassa (Decision 5).

## Dal value of operations all'enterprise value e all'equity value

Il value of operations (valore delle attività operative) è il valore attuale dei FCFF
espliciti e del valore terminale. Le attività non operative (per esempio liquidità investita
al di fuori dell'attività operativa, terreni destinati alla vendita, partecipazioni in società
non consolidate) si aggiungono per ottenere l'enterprise value. I diritti sull'impresa diversi
dal capitale proprio si sottraggono per ottenere l'equity value:

| Voce | Input | Segno |
| --- | --- | --- |
| Attività non operative | `non_operating_assets`: `{"label", "value"}`, valori pari a zero o positivi | aggiunte |
| Posizione finanziaria netta | `net_financial_position`: debiti finanziari lordi − liquidità + altre voci con il loro segno, come in SPEC-0007 | sottratta; positiva per un indebitamento netto |
| Debt-like items | `debt_like_items`: `{"label", "value", "measurement_basis", "basis_note"}`, valori pari a zero o positivi; per esempio obblighi pensionistici non finanziati o fondi per rischi e oneri | sottratti |
| Partecipazioni di minoranza | `non_controlling_interests`: `{"label", "value", "measurement_basis", "basis_note"}`, valori pari a zero o positivi; la quota delle controllate consolidate che appartiene ad altri soci | sottratte |

Ogni importo è misurato alla data di valutazione e preso come fornito: Fairmark non sceglie
quali voci includere né come misurarle. Il criterio di valutazione di ogni debt-like item e di
ogni partecipazione di minoranza è una scelta dell'analista, dichiarata nel campo obbligatorio
`measurement_basis`:

| `measurement_basis` | Significato |
| --- | --- |
| `book_value` | Valore contabile risultante dal bilancio alla data di valutazione |
| `market_value` | Valore di mercato: prezzo quotato alla data di valutazione |
| `fair_value` | Fair value come definito dall'IFRS 13 |
| `present_value` | Valore attuale dell'obbligazione, per esempio obblighi pensionistici non finanziati o fondi per rischi e oneri |
| `other` | Un altro criterio, descritto in `basis_note` (obbligatorio) |

Il campo facoltativo `basis_note` precisa il criterio, per esempio "net of tax" o "49% of the
book value of equity of the subsidiary"; con `other` è obbligatorio. Il criterio e la nota sono
riportati accanto all'importo in `explain` e non sono mai usati nel calcolo. Una voce già inclusa tra le `other_items` della posizione finanziaria netta (per
esempio i debiti per leasing) non va ripetuta in `debt_like_items`: il motore non controlla i
doppi conteggi. Le liste non fornite sono riportate come "none given". Ogni voce è un passo di
`explain` con la sua descrizione, ogni lista ha un passo con il totale e il passo dell'equity
value mostra ogni termine sottratto.

```python
import fairmark as fm

r = fm.dcf.fcff(
    [fm.dcf.given([250, 270, 285, 300])],
    cash_flow_terms="nominal",
    discount_rate_terms="nominal",
    wacc="0.09",
    timing="end_of_period",
    terminal=fm.dcf.gordon("0.02", first_flow="last_flow_grown"),
    net_financial_position={"gross_debt": 900, "cash": 150},
    non_operating_assets=[{"label": "Land held for sale", "value": 80}],
    debt_like_items=[
        fm.dcf.bridge_claim(
            "Unfunded pension obligation",
            95,
            measurement_basis="present_value",
            basis_note="net of plan assets, after tax",
        )
    ],
    non_controlling_interests=[
        fm.dcf.bridge_claim("30% of Subsidiary A", 210, measurement_basis="fair_value")
    ],
)
print(r.bridge["debt_like_items_total"], r.bridge["non_controlling_interests_total"])  # 95 210
assert r.enterprise_value - r.equity_value == 750 + 95 + 210
```

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

Con un multiplo di uscita il valore terminale è un prezzo all'orizzonte; indicando il flusso in
N + 1 si ottiene anche la crescita perpetua implicita nel multiplo:

```python
r = fm.dcf.fcff(
    [fm.dcf.given([150, 165, 180, 190])],
    cash_flow_terms="nominal",
    discount_rate_terms="nominal",
    wacc="0.10",
    timing="end_of_period",
    terminal=fm.dcf.exit_multiple("7.5", "ebitda", 410, next_flow=260),
)
print(r.terminal["technique"], r.terminal["terminal_value"])  # tv.exit_multiple 3075.0
print(r.terminal["output"]["implied_growth"])  # 0.10 − 260 / 3075 = 0.01544...
```

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `stages` | Da uno a tre stage, in ordine: `{"periods", "projection", ...}` con `flows` o `lines` (`given`), `growth` (`constant_growth`) oppure `from_growth` e `to_growth` (`linear_decline`); uno stage di flussi forniti può essere soltanto `{"flows": [...]}` | obbligatorio |
| `stages[].projection` | `given`, `constant_growth` oppure `linear_decline` | `given` per uno stage con `flows` o `lines`; obbligatoria negli altri casi |
| `stages[].periods` | Numero di periodi dello stage, almeno 1 | il numero di `flows` o `lines` di uno stage `given`; obbligatorio per uno stage generato |
| `base_flow` | FCFF del periodo base $CF_0$, quando il primo stage è generato | obbligatorio per un primo stage generato; rifiutato altrimenti |
| `notional_tax_rate` | Aliquota fiscale sull'EBIT per gli stage con `lines`, frazione decimale da 0 a meno di 1; in alternativa a `operating_taxes` in ogni voce | nessuno; rifiutata senza `lines` |
| `negative_ebit` | `credit` o `zero`, per gli stage con `lines` e `notional_tax_rate` | obbligatorio con `notional_tax_rate`; rifiutato altrimenti |
| `cash_flow_terms` | `nominal` oppure `real`: termini di ogni importo e di ogni tasso di crescita | `nominal`, riportato |
| `discount_rate_terms` | `nominal` oppure `real`: termini dei tassi di attualizzazione; se diversi da `cash_flow_terms`, l'inflazione attesa è obbligatoria e i tassi sono convertiti nei termini dei flussi di cassa | `nominal`, riportato |
| `inflation` | Inflazione attesa per tutti i periodi e per la fase terminale, frazione decimale, maggiore di −1 | nessuna; questo oppure `inflation_per_period` |
| `inflation_per_period` | Inflazione attesa per ogni periodo dell'orizzonte esplicito, frazioni decimali | nessuna; richiede `terminal_inflation` |
| `terminal_inflation` | Inflazione attesa della fase terminale | obbligatoria con `inflation_per_period` quando il valore terminale usa un tasso terminale; rifiutata altrimenti |
| `wacc` | Un unico WACC per tutti i periodi e per il valore terminale, frazione decimale | questo oppure `wacc_per_period` |
| `wacc_per_period` | Un WACC per ogni periodo dell'orizzonte esplicito, frazioni decimali | questo oppure `wacc` |
| `terminal_wacc` | WACC della fase terminale | obbligatorio con `wacc_per_period` quando il valore terminale usa un tasso terminale (una tecnica, oppure un valore fornito attualizzato in N − 0.5); rifiutato con `wacc` e quando non è usato alcun tasso terminale |
| `timing` | `end_of_period` oppure `mid_period` | `end_of_period`, riportato |
| `terminal_discount_time` | `horizon` (t = N) oppure `mid_period` (t = N − 0.5) | obbligatorio con `mid_period` e un valore terminale; rifiutato con `end_of_period` e senza valore terminale |
| `terminal` | Una tecnica indicata con l'identificativo del suo metodo, per esempio `{"tv.gordon": {"growth", "first_flow", "normalised_next_flow", "growth_cap"}}`, un importo fornito `{"given": {"value"}}`, oppure `{"none": {}}`; si veda "Tecniche del valore terminale" | `none`, riportato con un'avvertenza |
| `net_financial_position` | `{"gross_debt", "cash", "other_items"}` come in [`ratios.debt_service`](ratios-debt-service.md), positiva per indebitamento netto | nessuna, riportato |
| `non_operating_assets` | Lista di `{"label", "value"}`, valori pari a zero o positivi | nessuna, riportato |
| `debt_like_items` | Lista di `{"label", "value", "measurement_basis", "basis_note"}`, valori pari a zero o positivi, sottratti dall'enterprise value; `measurement_basis` obbligatorio, `basis_note` obbligatorio con `other` | nessuno, riportato |
| `non_controlling_interests` | Partecipazioni di minoranza: lista di `{"label", "value", "measurement_basis", "basis_note"}`, valori pari a zero o positivi, sottratte dall'enterprise value; `measurement_basis` obbligatorio, `basis_note` obbligatorio con `other` | nessuna, riportato |

## Output

| Campo | Significato |
| --- | --- |
| `value_of_operations` | Valore attuale dei FCFF espliciti più valore attuale del valore terminale |
| `enterprise_value` | Value of operations più le attività non operative |
| `equity_value` | Enterprise value meno la posizione finanziaria netta, i debt-like items e le partecipazioni di minoranza |
| `pv_explicit_period` | Valore attuale dei FCFF espliciti |
| `terminal_value_weight` | Valore attuale del valore terminale diviso per il value of operations |
| `periods` | Per periodo: stage, proiezione, crescita applicata, flusso, tasso di attualizzazione fornito, tasso usato (nei termini dei flussi di cassa), inflazione usata per convertirlo (`null` senza conversione), fattore di sconto, valore attuale |
| `terms` | Termini dei flussi e dei tassi, conversione (`none`, `to_real`, `to_nominal`), tasso terminale fornito e usato, inflazione della fase terminale, crescita perpetua nei termini dei flussi e negli altri termini |
| `terminal` | Tecnica (`given` o `none` per le altre opzioni; con `none` momento e fattore di attualizzazione sono `null`) e versione del suo metodo, tasso terminale, valore in N, momento di attualizzazione, fattore, valore attuale, output proprio della tecnica (per esempio la crescita implicita di un multiplo di uscita); versione, tasso e output sono `null` quando non si applicano |
| `bridge` | Ogni voce dal value of operations all'equity value, con le descrizioni e i criteri di valutazione, e il totale di ciascuna lista |

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Approccio | asset side (enterprise value): FCFF attualizzati al WACC | fissato dal metodo |
| Proiezione dello stage | `given`, `constant_growth`, `linear_decline` | `given` per uno stage con `flows` o `lines`; obbligatoria negli altri casi |
| Periodi di uno stage | un intero pari almeno a 1 | il numero di flussi o di `lines` di uno stage `given`; obbligatorio per uno stage generato |
| Flussi di uno stage `given` | importi (`flows`) o voci (`lines`), con le convenzioni fiscali di `dcf.free_cash_flows` | uno dei due, obbligatorio |
| Passo della riduzione lineare della crescita | $g_k = g_{from} + (g_{to} - g_{from}) \, k / m$, $g_{to}$ raggiunta nell'ultimo periodo | fissa |
| Tasso di attualizzazione | un unico tasso (usato anche per il valore terminale), oppure uno per periodo con un tasso terminale | obbligatorio |
| Termini di flussi e tassi | `nominal`, `real` | `nominal`, entrambi |
| Termini diversi | rifiutati senza inflazione attesa; con essa, ogni tasso di attualizzazione è convertito nei termini dei flussi di cassa con la relazione di Fisher esatta; flussi e tassi di crescita mai convertiti | fissa |
| Inflazione attesa | un unico tasso, oppure uno per periodo con un'inflazione terminale | nessuna |
| Collocazione temporale dei flussi | `end_of_period`, `mid_period` | `end_of_period` |
| Terminal discounting time (momento di attualizzazione del valore terminale) | `horizon`, `mid_period` (solo con la convenzione di metà periodo; il mezzo periodo al tasso terminale) | obbligatorio con un valore terminale e la convenzione di metà periodo |
| Opzione del valore terminale | `tv.gordon`, `tv.h_model`, `tv.value_driver`, `tv.exit_multiple`, `tv.limited_horizon`, `tv.normalised_income`, `tv.liquidation`, al tasso terminale, con approccio asset side; `given`, un importo in N con approccio asset side; `none` | `none` |
| Primo flusso terminale (`tv.gordon`, `tv.h_model`) | ultimo flusso fatto crescere; oppure flusso normalizzato in N + 1 (`tv.gordon`), flusso base normalizzato in N (`tv.h_model`) | obbligatorio |
| Voci del passaggio all'equity value | misurate alla data di valutazione come fornite; nessuna voce significa nessuna | nessuna |
| Segni delle voci del passaggio all'equity value | attività non operative aggiunte; posizione finanziaria netta (positiva per un indebitamento netto), debt-like items e partecipazioni di minoranza sottratti | fissi |
| Criterio di valutazione dei debt-like items e delle partecipazioni di minoranza | `book_value`, `market_value`, `fair_value`, `present_value`, `other` (con `basis_note`); dichiarato dall'analista, riportato, non usato nel calcolo | obbligatorio per ogni voce |

Le avvertenze sono riportate in `explain.conventions` e non comportano mai il rifiuto
dell'input: una crescita applicata a un flusso negativo; una crescita dell'ultimo periodo
esplicito diversa dalla crescita perpetua del valore terminale (`growth`, oppure
`long_term_growth` per `tv.h_model`; la crescita implicita in un multiplo di uscita è riportata,
non confrontata); con la convenzione di metà periodo, un valore terminale a crescita perpetua
(`tv.gordon`, `tv.h_model`, `tv.value_driver`, `tv.normalised_income`) attualizzato all'orizzonte N,
mentre la prassi lo attualizza di solito in N − 0.5; con la convenzione di metà periodo, un
multiplo di uscita, una liquidazione, un orizzonte finito o un valore terminale fornito
attualizzati in N − 0.5, benché siano o includano un importo riferito a una data (per
l'orizzonte finito, il valore residuo; per `given`, il valore terminale dell'analista
all'orizzonte N); con flussi
nominali e inflazione attesa, una crescita perpetua inferiore all'inflazione della fase
terminale (una diminuzione in termini reali); un equity value negativo, riportato come calcolato e mai riportato a zero; l'assenza del valore terminale (`none`), con il
valore che copre soltanto i flussi espliciti; un valore terminale negativo fornito
dall'analista, attualizzato come fornito. Un `terminal_value_weight` elevato segnala che la
valutazione dipende soprattutto dalle ipotesi di lungo periodo.

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | Un campo è mancante, sconosciuto o non usato dall'opzione scelta; un `null` esplicito per un campo con valore predefinito (`stages[].projection`, `stages[].periods`, `cash_flow_terms`, `discount_rate_terms`, `timing`, `terminal`): omettere il campo per usare il valore predefinito; sia `inflation` sia `inflation_per_period`, `inflation_per_period` senza `terminal_inflation`, `terminal_inflation` senza `inflation_per_period`, `terminal_inflation` insieme a `inflation`; nessuno stage o più di tre; stage che sommano più di 10.000 periodi; una crescita pari o inferiore a −100%; un tasso terminale insieme a un unico tasso, oppure nessuno con tassi per periodo quando il valore terminale lo usa; un tasso terminale o un'inflazione terminale quando il valore terminale non usa alcun tasso terminale; un momento di attualizzazione del valore terminale mancante con la convenzione di metà periodo e un valore terminale, oppure fornito con quella di fine periodo o senza valore terminale; nessuna opzione del valore terminale o più di una (per esempio `given` insieme a una tecnica); uno stage senza proiezione e senza `flows` o `lines`; uno stage generato senza `periods`; un `normalised_base_flow` di `tv.h_model` mancante con `first_flow` = `normalised_base_flow` o fornito negli altri casi; un limite alla crescita di un multiplo di uscita senza `next_flow`; una liquidazione senza attività o con una voce senza descrizione; un'attività non operativa negativa o con descrizione vuota; un debt-like item o una partecipazione di minoranza negativi, o con descrizione vuota; un criterio di valutazione mancante o diverso dalle opzioni previste; `other` senza `basis_note`, o un `basis_note` vuoto (per esempio `/non_controlling_interests/0/basis_note`); uno stage `given` con entrambi o nessuno tra `flows` e `lines`, `lines` in uno stage generato, campi fiscali senza `lines`, oneri finanziari o net borrowing in `lines`, e ogni rifiuto di [`dcf.free_cash_flows`](dcf-free-cash-flows.md) sulle voci, indicato con il suo pointer (ad esempio `/stages/0/lines/2/capital_expenditure`) |
| `FM-E003` | Un tasso di attualizzazione, fornito o convertito, oppure un'inflazione attesa è pari o inferiore a −100%; il messaggio indica il campo e il periodo (lo stesso codice di [`rates.real_nominal`](rates-real-nominal.md) per lo stesso valore) |
| `FM-E004` | La crescita perpetua (`growth`, oppure `long_term_growth` per `tv.h_model`) è pari o superiore al tasso terminale nei termini dei flussi di cassa (dopo la conversione, se i termini sono diversi) |
| `FM-E008` | Overflow aritmetico (input molto al di fuori di un intervallo realistico) |
| `FM-E018` | Un parametro della tecnica del valore terminale è fuori dal suo dominio: rendimento del nuovo capitale investito, multiplo o grandezza di riferimento pari a zero o negativi, un tasso terminale pari a zero o negativo con `tv.normalised_income`, orizzonte finito fuori dall'intervallo da 1 a 10.000 periodi, H negativo, voce di liquidazione o imposte sulle plusvalenze negative |
| `FM-E019` | La tecnica del valore terminale segue l'approccio equity side: un multiplo di uscita su `earnings` (P/E) o `book_value` (P/BV), oppure un reddito normalizzato o una liquidazione con `basis` equity |
| `FM-E024` | `cash_flow_terms` e `discount_rate_terms` sono diversi e non è fornita l'inflazione attesa; il messaggio indica entrambe le correzioni e, quando uno dei due termini non è stato fornito, che ha assunto il valore predefinito (`nominal`) e come indicarlo |
| `FM-E025` | Uno stage `given` con più o meno flussi o voci dei suoi periodi, tassi o inflazione attesa per periodo non uno per periodo, un base flow mancante per un primo stage generato o fornito per uno `given` |

Ogni errore indica il campo con il suo JSON Pointer, per esempio `/stages/1/flows`, e la
correzione da apportare, compresi gli errori rilevati durante la lettura dell'input: un campo
mancante o sconosciuto (`/terminal/tv.gordon/first_flow`), un tipo errato o una scelta
sconosciuta (`/stages/0/projection`, con i valori ammessi).

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
- Damodaran, *Investment Valuation*, 2a edizione, capitolo 16, Illustration 16.6 (Segovia
  Entertainment, approccio consolidato): valore delle attività operative 2.205, più la
  partecipazione del 15% in LatinWorks al valore contabile 22,5, meno i debiti consolidati 500,
  meno le partecipazioni di minoranza in Seville al valore contabile 122,5, equity value 1.605,
  riprodotti esattamente (`damodaran-segovia-consolidated.yaml`).
- Un'implementazione indipendente a 50 cifre, `scripts/reference_dcf.py`, per ogni caso di
  riferimento (golden case) in `tests/golden/dcf.fcff/`, e le proprietà: uno stage a crescita
  costante equivale agli stessi flussi forniti; una riduzione lineare con crescite uguali
  equivale alla crescita costante; i fattori di sconto coincidono con quelli di
  [`tvm.present_value`](tvm-present-value.md).
- Termini nominali e reali: la conversione riespressa nel calcolo di riferimento con la
  relazione di Fisher scritta lì (`terms-*.yaml`); flussi reali a tassi nominali con
  l'inflazione coincidono esattamente con gli stessi flussi ai tassi reali calcolati da
  `rates.real_nominal` e forniti direttamente, e con la convenzione di fine periodo con i flussi
  nominali ottenuti applicando l'inflazione in modo indipendente, ai tassi nominali, entro
  $10^{-20}$ in termini relativi (test di proprietà, con un'unica inflazione e con l'inflazione
  per periodo).
- Stage con `lines`: la costruzione a 50 cifre di `scripts/reference_free_cash_flows.py`
  composta con `scripts/reference_dcf.py` (`stage-with-build-up-lines*.yaml`).
- Ogni tecnica del valore terminale: il valore terminale e l'intero output della tecnica
  all'interno del DCF coincidono con quelli del metodo `tv.*` eseguito da solo sugli stessi
  parametri e al tasso terminale, e il suo valore attuale coincide con quel valore attualizzato
  al momento di attualizzazione del valore terminale (test di proprietà, per ogni tecnica,
  forma del tasso e collocazione temporale dei flussi).
- Il VAN di numpy-financial sui flussi espliciti più il valore di Gordon, in virgola mobile
  binaria (concordanza delle convenzioni, non delle cifre).
- Valori predefiniti documentati (Decision 24): l'input minimo, il suo equivalente esplicito,
  ogni valore predefinito sostituito da solo (collocazione temporale dei flussi, termini dei
  flussi, termini dei tassi, tecnica del valore terminale) e il valore terminale fornito
  dall'analista in N, in N − 0.5 e negativo, confrontati con `scripts/reference_dcf.py`, che
  assegna i valori predefiniti con un proprio codice scritto dalla specifica
  (`flows-only-*.yaml`, `defaults-override-*.yaml`, `terminal-given-*.yaml`). L'input minimo
  coincide con [`tvm.npv`](tvm-npv.md) degli stessi flussi con un primo flusso pari a zero,
  fino all'ultima cifra al 9% (`tests/golden/tvm.npv/dcf-flows-only-cross-check.yaml`, da
  `scripts/reference_npv_irr.py`) ed entro $10^{-25}$ in termini relativi agli altri tassi,
  dove ciascun metodo arrotonda le proprie operazioni.
