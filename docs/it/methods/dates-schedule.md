---
source: methods/dates-schedule.md
source_hash: 895f6bb5dfa8
---
# Schedule dei periodi (`dates.schedule`)

Costruisce lo schedule (il piano dei periodi) di un modello a partire da una data di inizio, una
lunghezza del periodo (months per period, cioè i mesi per periodo) e un numero di periodi: la
data di inizio e la data di fine di ogni periodo e, con una convenzione di calcolo dei giorni, la
sua frazione d'anno. Usalo al posto di date scritte a mano, così che fine
mese, febbraio e anni bisestili siano trattati una volta sola, con una regola dichiarata, e le
date alimentino [`tvm.xnpv`](tvm-xnpv.md), [`tvm.xirr`](tvm-xirr.md) e
[`series.project`](series-project.md) senza essere ribattute.

## Che cos'è un periodo: [inizio, fine)

Ogni periodo va **dalla data di inizio inclusa alla data di fine esclusa**, come i calculation
period (i periodi su cui maturano gli interessi) delle 2006 ISDA Definitions. La data di fine di un periodo è il
primo giorno del periodo successivo: appartiene al periodo successivo, non a questo. Non esiste
un campo "ultimo giorno del periodo", quindi una data di fine non viene mai contata due volte.

```text
data di inizio                                                 data di fine del periodo 3
    |                                                                         |
    [ periodo 1 ...............)[ periodo 2 ...............)[ periodo 3 .....)
 2025-12-31              2026-03-31                  2026-06-30          2026-09-30
    ^ inclusa                   ^ inizio del periodo 2, non fa parte del periodo 1
```

Uno schedule trimestrale che inizia il 1° gennaio ha le date di fine al 1° aprile, 1° luglio,
1° ottobre e 1° gennaio: i suoi periodi sono i trimestri di calendario (dal 1° gennaio al
31 marzo inclusi, dal 1° aprile al 30 giugno inclusi, e così via), e ogni data di fine è il primo
giorno del trimestre successivo. Uno schedule trimestrale che inizia il 31 dicembre ha le date
di fine al 31 marzo, 30 giugno, 30 settembre e 31 dicembre: lo schedule dal 31 dicembre copre
dal 31 dicembre al 30 marzo inclusi (90 giorni), un giorno prima del trimestre di calendario, e
così per ogni periodo. Entrambi sono esatti. Inizia il 1° gennaio quando i periodi devono essere
i trimestri di calendario; inizia il 31 dicembre quando i valori di fine periodo devono essere
datati a fine mese (31 marzo, 30 giugno, ...).

## Formula

Con data di inizio $d_0$, $p$ mesi per periodo e $n$ periodi:

$$
d_k = d_0 + k \times p \text{ mesi}, \qquad
\text{periodo } k = [d_{k-1},\ d_k), \qquad k = 1, \dots, n
$$

Ogni data è generata a partire dalla data di inizio, mai dalla data precedente, così un giorno
accorciato in un mese viene recuperato nel successivo: dal 31 gennaio, con periodicità mensile,
le date sono il 29 febbraio (2024) e poi il 31 marzo, non il 29 marzo. Il giorno di ogni data
segue un'unica regola fissa, la end-of-month rule (regola di fine mese) della prassi di mercato,
la regola `endOfMonth` di QuantLib `Schedule`:

- se la data di inizio è l'**ultimo giorno del suo mese**, ogni data è l'ultimo giorno del suo
  mese (30 giugno, trimestrale: 30 settembre, 31 dicembre, 31 marzo, 30 giugno);
- altrimenti ogni data mantiene il **giorno della data di inizio**, oppure l'ultimo giorno del
  mese quando quel mese è più corto (30 gennaio, mensile: 28 febbraio, 30 marzo, 30 aprile).

Con una convenzione di calcolo dei giorni, la frazione d'anno del periodo $k$ è quella di
[`dates.year_fraction`](dates-year-fraction.md) da $d_{k-1}$ a $d_k$: la stessa funzione, non
una seconda implementazione.

## Lunghezza del periodo

Indica la lunghezza del periodo in esattamente una di due forme. Una periodicità con nome e il
numero di mesi equivalente danno risultati identici.

| `frequency` | Mesi per periodo | Periodi all'anno |
| --- | --- | --- |
| `monthly` (mensile) | 1 | 12 |
| `bimonthly` (ogni due mesi, non due volte al mese) | 2 | 6 |
| `quarterly` (trimestrale) | 3 | 4 |
| `four_monthly` (quadrimestrale: ogni quattro mesi) | 4 | 3 |
| `semi_annual` (semestrale) | 6 | 2 |
| `annual` (annuale) | 12 | 1 |

`period_months` sono i months per period (i mesi per periodo), un numero intero positivo: 1, 2, 3, 4, 6, 12 e
qualsiasi altro come 5, 18 o 24. Indica i mesi per periodo, non i periodi all'anno, quindi non
può essere confuso con una frequenza di capitalizzazione. Quando i mesi per periodo dividono 12
il risultato riporta anche `periods_per_year`; per 5, 18 o 24 mesi è assente.

## Esempi

Date di fine della end-of-month rule, ciascuna verificata con QuantLib e python-dateutil:

| Inizio | Lunghezza del periodo | Date di fine |
| --- | --- | --- |
| 2024-01-31 | `monthly` | 2024-02-29, 2024-03-31, 2024-04-30, 2024-05-31 |
| 2025-01-30 | `monthly` | 2025-02-28, 2025-03-30, 2025-04-30 (non è un fine mese: il giorno 30 è mantenuto) |
| 2023-02-28 | `monthly` | 2023-03-31, 2023-04-30, 2023-05-31 (il 28 febbraio 2023 è un fine mese) |
| 2024-02-29 | `annual` | 2025-02-28, 2026-02-28, 2027-02-28, 2028-02-29 |
| 2025-01-31 | `four_monthly` | 2025-05-31, 2025-09-30, 2026-01-31 |
| 2025-01-31 | `period_months` 5 | 2025-06-30, 2025-11-30, 2026-04-30 |
| 2025-03-15 | `period_months` 18 | 2026-09-15, 2028-03-15 |

```python
from decimal import Decimal

import fairmark as fm

q = fm.dates.schedule("2025-12-31", 4, frequency="quarterly", day_count="act_365_fixed")
print(q.end_dates)       # ['2026-03-31', '2026-06-30', '2026-09-30', '2026-12-31']
print(q.start_dates[1])  # 2026-03-31: la data di fine del periodo 1 apre il periodo 2
print(q.year_fractions)  # 90/365, 91/365, 92/365, 92/365
assert q.start_dates[1:] == q.end_dates[:-1]
print(q.steps[-1].formula)  # 365 / 365: la frazione d'anno totale, numeratori sommati prima
assert q.steps[-1].value == Decimal(365) / 365  # giorni totali / 365 esatti

jan = fm.dates.schedule("2026-01-01", 4, frequency="quarterly")
print(jan.end_dates)     # ['2026-04-01', '2026-07-01', '2026-10-01', '2027-01-01']

# Una lunghezza senza nome, e la periodicità con nome uguale a un numero di mesi.
assert fm.dates.schedule("2025-03-15", 2, period_months=18).end_dates == [
    "2026-09-15",
    "2028-03-15",
]
assert (
    fm.dates.schedule("2025-12-31", 4, period_months=3).end_dates == q.end_dates
)
```

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `start_date` | Data di inizio del primo periodo, ISO 8601 `YYYY-MM-DD` | obbligatorio |
| `frequency` | Periodicità con nome: `monthly`, `bimonthly`, `quarterly`, `four_monthly`, `semi_annual` oppure `annual` | esattamente uno tra `frequency` e `period_months` |
| `period_months` | Mesi per periodo, un numero intero positivo | esattamente uno tra `frequency` e `period_months` |
| `number_of_periods` | Numero di periodi, da 1 a 10000 | obbligatorio |
| `day_count` | `act_365_fixed`, `act_360`, `act_act_isda`, `thirty_360` oppure `thirty_e_360` | nessuno: non viene calcolata alcuna frazione d'anno |

## Output

| Campo | Significato |
| --- | --- |
| `start_dates` | Data di inizio di ogni periodo, inclusa nel periodo |
| `end_dates` | Data di fine di ogni periodo, primo giorno del periodo successivo, esclusa dal periodo |
| `year_fractions` | Frazione d'anno di ogni periodo su [inizio, fine), solo con una convenzione di calcolo dei giorni |
| `months_per_period` | Mesi per periodo |
| `periods_per_year` | 12 / mesi per periodo, solo quando i mesi per periodo dividono 12 |

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Valore | Default |
| --- | --- | --- |
| Definizione del periodo | [inizio, fine): dalla data di inizio inclusa alla data di fine esclusa | fissa |
| Generazione delle date | dalla data di inizio, $k \times p$ mesi (in avanti, senza deriva) | fissa |
| End-of-month rule | un inizio a fine mese mantiene i fine mese; altrimenti il giorno di inizio, ridotto alla lunghezza del mese; `explain` indica se l'inizio è un fine mese e quante date di fine cadono in un altro giorno | fissa |
| Giorni lavorativi | nessuno: unadjusted dates (date non spostate al giorno lavorativo) | fissa nella versione 1 |
| Convenzione di calcolo dei giorni | le cinque convenzioni di [`dates.year_fraction`](dates-year-fraction.md) | nessuna: senza di essa non viene calcolata alcuna frazione d'anno |

Una data di fine dello schedule, una data di termine, un calendario, una convenzione di giorno
lavorativo o un periodo irregolare (stub) non fanno parte della versione 1 e sono rifiutati come
campi sconosciuti.

`explain` riporta il numero di periodi, i mesi per periodo, i periodi all'anno quando definiti,
i giorni effettivi dell'intero schedule e, con una convenzione di calcolo dei giorni, la frazione
d'anno del periodo 1 con il suo calcolo e la frazione d'anno totale dello schedule, con i suoi
termini esatti sostituiti (per esempio `121 / 365`, oppure `a / 365 + b / 366 + w` con
Actual/Actual (ISDA)). I valori di ogni
periodo sono nei vettori di output, quindi il numero di passaggi non cresce con il numero di
periodi.

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | Sia `frequency` sia `period_months`, oppure nessuno dei due (il messaggio indica entrambi i pointer); `period_months` pari a 0; `number_of_periods` fuori dall'intervallo da 1 a 10000; un campo sconosciuto come `end_date`, `calendar`, `business_day_convention` o `stub` |
| `FM-E009` | `start_date` è malformata, inesistente (`2023-02-29`) o fuori intervallo; una data di fine generata cade dopo il 2199-12-31: il messaggio indica `/number_of_periods`, la prima data fuori intervallo e l'ultimo periodo che rientra |

## Precisione

Le date sono aritmetica intera su mesi e giorni: esatte. Le frazioni d'anno sono quelle di
[`dates.year_fraction`](dates-year-fraction.md), rapporti tra interi arrotondati una sola volta.
La frazione d'anno totale dello schedule in `explain` addiziona i numeratori interi prima di
dividere, quindi con Actual/365 (Fixed) è uguale esattamente ai giorni totali dello schedule / 365.
Poiché ogni valore restituito in `year_fractions` è arrotondato una volta a 28 cifre
significative, il totale può differire dalla somma dei valori restituiti fino a $n$ mezze unità
della loro ultima cifra.

## Verificato con

- QuantLib 1.44 `Schedule` (calendario nullo, `Unadjusted`, generazione in avanti, `endOfMonth`
  vero, tenor `Period(p, Months)`) e python-dateutil 3.0.0 (`start + relativedelta(months=k
  × p, day=31)` per un inizio a fine mese, `start + relativedelta(months=k × p)` altrimenti):
  ogni caso di riferimento in `tests/golden/dates.schedule/` è costruito con entrambe le
  librerie, che concordano (`scripts/reference_schedule.py`), e i test con gli oracoli
  confrontano schedule casuali con entrambe. I casi di riferimento sono stati generati con
  python-dateutil 3.0.0 e danno le stesse date con la 4.0.0, la versione che il lock file
  installa da Python 3.11 in poi. La riduzione del giorno di python-dateutil è stata letta nel
  suo codice sorgente (`relativedelta.__add__`: il giorno è il minore tra il giorno
  indicato e la lunghezza del mese).
- Frazioni d'anno: il riferimento ISDA a 50 cifre di `scripts/reference_time.py`, confermato
  dalle convenzioni di QuantLib `Actual365Fixed`, `Actual360`, `ActualActual(ISDA)`,
  `Thirty360(BondBasis)` e `Thirty360(European)`.

## Fonti

- 2006 ISDA Definitions: "Calculation Period" (dalla Period End Date inclusa alla successiva
  esclusa), "Period End Date", "Effective Date"; numeri di sezione da verificare. Le convenzioni
  di calcolo dei giorni, sezione 4.16, sono verificate per
  [`dates.year_fraction`](dates-year-fraction.md).
- End-of-month rule: la regola applicata è quella descritta sopra, il comportamento di QuantLib
  `Schedule` con `endOfMonth` vero; la sua formulazione nelle 2006 ISDA Definitions è da
  verificare.
- Valori di QuantLib `Frequency` (Monthly 12, Bimonthly 6, Quarterly 4, EveryFourthMonth 3,
  Semiannual 2, Annual 1) e `Period(n, Months)`; python-dateutil `relativedelta`.
