---
source: methods/tvm-xnpv.md
source_hash: d5f0dfe19d9f
---
# Valore attuale netto di flussi datati (`tvm.xnpv`)

Attualizza a una data di valutazione flussi di cassa che avvengono in date di calendario, come
un closing, una distribuzione o un earn-out. Il tempo di ciascun flusso è la sua frazione d'anno
dalla data di valutazione secondo la convenzione di calcolo dei giorni scelta, e il tasso è un
tasso annuo con capitalizzazione esplicita. Con i valori di default il risultato coincide con
quello della funzione `XNPV` dei fogli di calcolo.

## Formula

$$
XNPV = \sum_{j} CF_j \cdot DF(r, t_j),
\qquad t_j = \text{year fraction}(D_0, D_j)
$$

$$
DF(r, t) = \left(1 + \frac{r}{m}\right)^{-m t}
\qquad\text{or}\qquad
DF(r, t) = e^{-r t}\ \text{(continuous compounding)}
$$

dove $D_0$ è la data di valutazione, $D_j$ la data del flusso $j$, $m$ il numero di periodi di
capitalizzazione all'anno, e la frazione d'anno è calcolata come in
[`dates.year_fraction`](dates-year-fraction.md). Con i valori di default (Actual/365 (Fixed),
capitalizzazione annuale, valutazione alla data del primo flusso):

$$
XNPV = \sum_{j} \frac{CF_j}{(1 + r)^{(D_j - D_1)/365}}
$$

## Esempio

Tasso del 9%, cinque flussi dal 1° gennaio 2008 al 1° aprile 2009. Il valore pubblicato è
2086.65, arrotondato a due decimali.

```python
from decimal import Decimal

import fairmark as fm

flows = [
    ("2008-01-01", -10000),
    ("2008-03-01", 2750),
    ("2008-10-30", 4250),
    ("2009-02-15", 3250),
    ("2009-04-01", 2750),
]
r = fm.tvm.xnpv("0.09", flows)
assert round(r.xnpv, 2) == Decimal("2086.65")
assert abs(r.xnpv - Decimal("2086.64760203153662166361")) < Decimal("1e-15")
print(r.valuation_date)  # 2008-01-01, la data del primo flusso
print(r.conventions[0])  # day count: Actual/365 (Fixed), ... (default, ECMA-376)

# Gli stessi flussi valutati sei mesi prima.
early = fm.tvm.xnpv("0.09", flows, valuation_date="2007-07-01")
assert abs(early.xnpv - Decimal("1997.93821626701985989269")) < Decimal("1e-15")
```

Le date sono stringhe ISO 8601 oppure oggetti `datetime.date`; i flussi possono essere in
qualsiasi ordine.

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `rate` | Tasso di attualizzazione annuo, frazione decimale (0.09 = 9%), espresso con la capitalizzazione `compounding` | obbligatorio |
| `cash_flows` | Flussi datati `{date, amount}`; gli importi negativi sono pagamenti, quelli positivi incassi | obbligatorio |
| `valuation_date` | Data alla quale è calcolato il valore; nessun flusso può precederla | data del primo flusso |
| `day_count` | `act_365_fixed`, `act_360`, `act_act_isda`, `thirty_360` oppure `thirty_e_360` | `act_365_fixed` |
| `compounding` | `annual`, `semi_annual`, `quarterly`, `monthly` oppure `continuous` | `annual` |

## Output

| Campo | Significato |
| --- | --- |
| `xnpv` | Valore attuale netto alla data di valutazione |
| `valuation_date` | La data di valutazione usata, fornita o di default |
| `flows` | Per ciascun flusso: `date`, `year_fraction` (anni dalla data di valutazione), `discount_factor`, `present_value` |

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Calcolo dei giorni | cinque convenzioni, vedi [`dates.year_fraction`](dates-year-fraction.md) | Actual/365 (Fixed), il default dei fogli di calcolo ECMA-376 |
| Capitalizzazione | annuale, semestrale, trimestrale, mensile, continua | annuale, il default dei fogli di calcolo ECMA-376 |
| Data di valutazione | qualsiasi data non successiva al primo flusso | data del primo flusso |

Ogni convenzione è riportata indicando se è stata fornita o se è il default. ECMA-376 valuta
alla data del primo flusso elencato e rifiuta i flussi anteriori; il default alla data del
primo flusso dà lo stesso valore ogni volta che il foglio di calcolo accetta l'input.

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | Un campo è mancante o sconosciuto, oppure una convenzione non è tra le opzioni elencate |
| `FM-E002` | `cash_flows` è vuoto |
| `FM-E003` | Il tasso per periodo di capitalizzazione, $r/m$, è pari o inferiore a −100%; il messaggio indica `/rate` e, con più di un periodo di capitalizzazione all'anno, il limite $-m$ |
| `FM-E008` | Overflow aritmetico (input molto al di fuori di un intervallo realistico) |
| `FM-E009` | Una data è malformata, non esiste o è fuori dall'intervallo 1900-01-01 - 2199-12-31 |
| `FM-E010` | Un flusso ha una data anteriore alla data di valutazione |

## Precisione

Le frazioni d'anno sono rapporti esatti, e i flussi che cadono a un numero intero di periodi
di capitalizzazione usano potenze intere esatte; rispetto a un calcolo indipendente a 60
cifre, concordano entro circa $10^{-24}$ in termini relativi o meglio. Gli esponenti
frazionari usano la funzione potenza decimale e la capitalizzazione continua l'esponenziale
decimale; questi concordano entro circa $10^{-18}$ in termini relativi o meglio su input
tipici. I tassi negativi sono attualizzati tramite il reciproco di $1 + r/m$, mantenendo la
piena precisione relativa. I casi di riferimento a piena precisione sono verificati entro
$10^{-19}$.

## Verificato con

- Esempio pubblicato: esempio della funzione `XNPV` di ECMA-376 / Excel, 2086.65.
- Concorda con la funzione `XNPV` di Excel e LibreOffice e con `pyxirr.xnpv`.
- Convenzioni di calcolo dei giorni: definizioni delle 2006 ISDA Definitions, sezione 4.16.
