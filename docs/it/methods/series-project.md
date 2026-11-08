---
source: methods/series-project.md
source_hash: dc3a9af350a5
---
# Proiezione di una serie di valori (`series.project`)

Fa crescere un base value (valore di partenza) periodo per periodo con una growth rule (la
regola con cui cresce) dichiarata e restituisce il valore di ogni periodo: ricavi di 1.000 che crescono del 12% annuo su dodici
periodi mensili, una voce di costo che cresce di un tasso diverso ogni anno, un flusso la cui
crescita si riduce in modo lineare fino a un tasso di lungo periodo. Usalo al posto di calcolare
la riga da solo, così che il base period (il periodo a cui appartiene il base value), la
conversione di un tasso annuo alla lunghezza del periodo e la data di ogni valore siano
dichiarati, non presunti. Con una `timeline` datata il
risultato alimenta direttamente [`tvm.xnpv`](tvm-xnpv.md) e [`tvm.xirr`](tvm-xirr.md).

## Formula

Con base value $V_0$ e $g_k$ il tasso di crescita per periodo verso il periodo $k$:

$$
V_k = V_{k-1} \times (1 + g_k)
$$

calcolato periodo per periodo, mai con una potenza in forma chiusa. Il base period indica a
quale valore corrisponde la base:

| `base_period` | Il base value è | Periodo 1 | Passi di crescita $s$ |
| --- | --- | --- | --- |
| `period_0` | il valore precedente al periodo 1, per esempio l'ultimo anno consuntivo | $V_1 = V_0 (1 + g_1)$ | $n$ |
| `period_1` | il valore del periodo 1 | $V_1 = V_0$ | $n - 1$, verso i periodi da 2 a $n$ |

Il base period è obbligatorio: le due letture differiscono di un intero periodo di crescita su
ogni valore. `period_0` è la convenzione degli stadi del DCF ([`dcf.fcff`](dcf-fcff.md)).

### Growth rule

| `growth` | Tasso del passo di crescita $j = 1, \dots, s$ |
| --- | --- |
| `{"constant": {"growth": g}}` | $g$ |
| `{"linear": {"from_growth": a, "to_growth": b}}` | $a + (b - a) \times j / s$: $a$ è la crescita prima del primo passo, $b$ è raggiunto nell'ultimo passo |
| `{"per_period": {"growth_rates": [...]}}` | i tassi elencati, in ordine, esattamente $s$ |

La regola lineare è quella dello stadio `linear_decline` di [`dcf.fcff`](dcf-fcff.md)
(Damodaran, *Investment Valuation*, 2a edizione, Illustration 15.3, "linear increments"), quindi
una serie e uno stadio del DCF con la stessa base e gli stessi tassi danno gli stessi valori,
bit per bit.

### Growth basis (come leggere i tassi): tassi annui e lunghezza del periodo

Con $p$ mesi per periodo, i tassi della regola sono letti secondo `growth_basis`:

| `growth_basis` | $p$ divide 12 ($m = 12 / p$ periodi all'anno) | $p$ multiplo di 12 ($k = p / 12$ anni) |
| --- | --- | --- |
| `per_period` | il tasso per periodo, come indicato | come indicato |
| `annual_compound` | $(1 + g)^{1/m} - 1$, il tasso equivalente in capitalizzazione composta: capitalizza a $g$ in un anno (il tasso equivalente di [`rates.convert`](rates-convert.md), diviso per $m$) | $(1 + g)^{k} - 1$, una potenza intera esatta |
| `annual_simple` | $g / m$, il tasso equivalente in regime di interesse semplice: $m$ periodi capitalizzati in regime composto arrivano a $(1 + g/m)^m - 1$, più di $g$ | $g \times k$ |

Dodici periodi mensili al 12% annuo, da 1.000:

| Base | Tasso mensile | Crescita nell'anno | $V_{12}$ |
| --- | --- | --- | --- |
| `annual_compound` | 0,9489% (arrotondato) | 12,00% | 1.120,00 |
| `annual_simple` | 1,00% | 12,68% | 1.126,83 |

La base è obbligatoria quando i periodi non durano 12 mesi. Con periodi di 12 mesi (`annual` o
`period_months` 12) le tre basi coincidono e può essere omessa; senza una lunghezza del periodo
i tassi sono per definizione per periodo. Per una lunghezza del periodo che non divide 12 e non è
multipla di 12 (5, 7 o 18 mesi) un tasso annuo richiederebbe una potenza frazionaria dell'anno
senza una convenzione consolidata, quindi è accettato solo `per_period`: indica i tassi per
periodo.

## Timeline e data dei valori

La `timeline` è:

- `{"periods": {"number_of_periods": n}}`, con `frequency` o `period_months` facoltativi:
  valori senza date, per esempio le colonne di un business plan;
- `{"schedule": {...}}`, l'input di [`dates.schedule`](dates-schedule.md): i periodi sono
  costruiti dalla stessa funzione, quindi le loro date sono identiche a quelle di
  `dates.schedule`.

Con uno `schedule`, `timing` è obbligatorio e data ogni valore: `end_of_period` alla data di
fine del suo periodo, `beginning_of_period` alla data di inizio. I periodi vanno dalla data di
inizio inclusa alla data di fine esclusa, e la data di fine è il primo giorno del periodo
successivo. Quindi i valori di fine periodo di un piano trimestrale che inizia il **1° gennaio**
sono datati **1° aprile, 1° luglio, 1° ottobre e 1° gennaio**; con inizio il **31 dicembre**
sono datati **31 marzo, 30 giugno, 30 settembre e 31 dicembre**. Entrambe le letture sono
esatte; la data di inizio decide quale lettura usa il modello.

## Esempio: schedule, serie, XNPV

Un esborso di 1.000 il 1° gennaio 2026, poi incassi trimestrali che crescono del 2% al
trimestre da una base di 300, valutati al 10% annuo:

```python
from decimal import Decimal

import fairmark as fm

timeline = fm.series.schedule(fm.dates.schedule_input("2026-01-01", 4, frequency="quarterly"))
r = fm.series.project(
    300,
    base_period="period_0",
    growth=fm.series.constant("0.02"),
    growth_basis="per_period",
    timeline=timeline,
    timing="end_of_period",
)
print(r.values)  # 306, 312.12, 318.3624, 324.729648
print([f["date"] for f in r.dated_values])  # 2026-04-01, 2026-07-01, 2026-10-01, 2027-01-01

flows = [("2026-01-01", "-1000")] + [(f["date"], f["amount"]) for f in r.dated_values]
v = fm.tvm.xnpv("0.10", flows)
assert round(v.xnpv, 20) == Decimal("188.27000581025293752184")

# 12% annuo su dodici periodi mensili: la base composta arriva a 1120, quella semplice a 1126,83.
compound = fm.series.project(
    1000,
    base_period="period_0",
    growth=fm.series.constant("0.12"),
    growth_basis="annual_compound",
    timeline=fm.series.periods(12, frequency="monthly"),
)
assert abs(compound.values[-1] - 1120) < Decimal("1e-17")
simple = fm.series.project(
    1000,
    base_period="period_0",
    growth=fm.series.constant("0.12"),
    growth_basis="annual_simple",
    timeline=fm.series.periods(12, frequency="monthly"),
)
assert simple.values[-1] == 1000 * Decimal("1.01") ** 12

# Damodaran, Illustration 15.3: crescita che scende dal 13,08% al 5% in cinque anni.
fade = fm.series.project(
    1000, base_period="period_0", growth=fm.series.linear("0.1308", "0.05"),
    timeline=fm.series.periods(5),
)
print([p["stated_growth"] for p in fade.periods])  # 0.11464, 0.09848, 0.08232, 0.06616, 0.05
```

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `base_value` | Base value $V_0$, nelle unità della serie; può essere negativo | obbligatorio |
| `base_period` | `period_0` oppure `period_1` | obbligatorio |
| `growth` | `constant`, `linear` oppure `per_period`, tassi come frazioni decimali maggiori di −1 | obbligatorio |
| `growth_basis` | `per_period`, `annual_compound` oppure `annual_simple` | obbligatorio salvo periodi di 12 mesi o senza durata |
| `timeline` | `periods` (numero di periodi, `frequency` o `period_months` facoltativi) oppure `schedule` (un input di `dates.schedule`) | obbligatorio |
| `timing` | `end_of_period` oppure `beginning_of_period`, solo con una `timeline` di tipo `schedule` | obbligatorio con una `timeline` di tipo `schedule` |

## Output

| Campo | Significato |
| --- | --- |
| `periods` | Una riga per periodo: `period`, `start_date`, `end_date` e `year_fraction` (con uno `schedule`), `stated_growth` (il tasso della regola), `growth_per_period` (dopo la conversione), `effective_annual_growth` (quando i mesi per periodo dividono 12), `value` |
| `values` | I valori dei periodi da 1 a $n$, nella forma di `cash_flows` di [`tvm.present_value`](tvm-present-value.md) |
| `dated_values` | Con uno `schedule`: un `{date, amount}` per periodo, nella forma di `cash_flows` di [`tvm.xnpv`](tvm-xnpv.md) e [`tvm.xirr`](tvm-xirr.md) |
| `months_per_period` | Mesi per periodo, quando la `timeline` ha una lunghezza del periodo |
| `periods_per_year` | Periodi all'anno, quando i mesi per periodo dividono 12 |

Con `period_1` i campi di crescita del periodo 1 sono assenti: nessun passo di crescita vi
conduce.

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Base period | `period_0`, `period_1` | nessuno: obbligatorio |
| Growth rule | `constant`, `linear`, `per_period` | nessuno: obbligatoria |
| Passo lineare | $g_j = g_\text{from} + (g_\text{to} - g_\text{from}) \times j / s$ | fisso |
| Growth basis | `per_period`, `annual_compound`, `annual_simple` | obbligatoria con periodi diversi da 12 mesi |
| Data dei valori | `end_of_period`, `beginning_of_period` | obbligatoria con uno `schedule` |
| Proiezione | $V_k = V_{k-1}(1 + g_k)$, periodo per periodo, senza arrotondamento | fissa |

Un base value negativo viene proiettato, con un avviso: una crescita positiva rende i valori
più negativi. `explain` riporta la ricorrenza, il base value e il base period, la growth rule
con i suoi parametri, la base e la crescita annua effettiva che ne deriva, la
conversione del primo tasso con i numeri sostituiti, il primo e l'ultimo valore e, con uno
`schedule`, le convenzioni dello schedule di [`dates.schedule`](dates-schedule.md). I
valori di ogni periodo sono nelle righe, quindi il numero di passaggi non cresce con il numero
di periodi.

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | Un tasso di crescita pari a −1 o inferiore, come indicato o dopo la conversione, indicato con il suo pointer (`/growth/constant/growth`, `/growth/per_period/growth_rates/1`); una lista `per_period` senza esattamente $s$ tassi, con il numero atteso ed effettivo e il base period; `growth_basis` mancante con periodi diversi da 12 mesi, una base annua senza lunghezza del periodo o con una durata che non divide 12 e non è multipla di 12; `timing` indicato con una `timeline` di tipo `periods` o mancante con uno `schedule`; sia `frequency` sia `period_months`; `number_of_periods` fuori dall'intervallo da 1 a 10000 |
| `FM-E008` | Un valore oltre l'intervallo dei decimali |
| `FM-E009` | Una data della `timeline` di tipo `schedule` fuori intervallo, indicata in `/timeline/schedule/...` come in [`dates.schedule`](dates-schedule.md) |

## Precisione

La ricorrenza usa la moltiplicazione decimale esatta: con input decimali i valori sono esatti
finché non si esauriscono le 28 cifre significative del tipo decimale. La regola lineare divide
una volta per passo. `annual_simple` è una divisione o una moltiplicazione; `annual_compound`
con periodi di anni interi è una potenza intera esatta; con $m$ periodi all'anno passa per il
logaritmo e l'esponenziale di [`rates.convert`](rates-convert.md), con un errore relativo
vicino a $10^{-27}$, quindi dodici periodi mensili al 12% arrivano a 1.120 entro $10^{-20}$
relativo.

## Verificato con

- Casi di riferimento in `tests/golden/series.project/`, da un calcolo indipendente a 50 cifre
  scritto a partire dalla specifica (`scripts/reference_series.py`): la ricorrenza, entrambi i
  base period, la regola lineare, le tre basi per ogni lunghezza del periodo che divide 12 e per
  24 mesi, e lo XNPV completo di una serie datata.
- I tassi lineari di Damodaran, *Investment Valuation*, 2a edizione, Illustration 15.3
  (Amgen): 11,46%, 9,85%, 8,23%, 6,62%, 5,00%.
- Test di proprietà: `constant` è uguale a `per_period` con il tasso ripetuto, `linear` con
  estremi uguali è uguale a `constant`, e i valori sono uguali ai flussi degli stadi
  `constant_growth` e `linear_decline` di `dcf.fcff`, bit per bit.

## Fonti

- Tassi equivalenti in capitalizzazione composta, $i_m = (1 + i)^{1/m} - 1$, e in regime di
  interesse semplice, $i_m = i / m$ (verificati il 2026-11-08): Università di Trieste,
  A. R. Bacinello, *Matematica finanziaria*, "Tassi equivalenti nel regime dell'interesse
  semplice e della capitalizzazione composta"; M. Sandri, *Matematica finanziaria*,
  "Capitalizzazione frazionata", "Tassi equivalenti". Il tasso equivalente composto è quello di
  [`rates.convert`](rates-convert.md).
- Damodaran, *Investment Valuation*, 2a edizione, capitolo 15, Illustration 15.3.
- Definizione del periodo e regola di fine mese: vedi [`dates.schedule`](dates-schedule.md).
