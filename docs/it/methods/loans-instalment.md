---
source: methods/loans-instalment.md
source_hash: 4c23dbb99426
---
# Rata di un finanziamento (`loans.instalment`)

Restituisce la rata costante di un finanziamento a partire dal capitale, dal tasso, dal numero
delle rate e dal momento di pagamento, con un balloon facoltativo rimborsato insieme all'ultima
rata e un arrotondamento facoltativo della rata. Si usa per inserire un nuovo debito bancario
in un business plan o per verificare la rata indicata da un finanziatore.

## Formula

Con il tasso periodico $i$, il numero delle rate $n$, il capitale $P$ e il balloon $B$ (pagato
alla fine del periodo $n$), la rata $A$ rende il valore attuale delle rate e del balloon pari al
capitale:

$$
A_{\text{posticipata}} = \frac{\left(P - B\,(1 + i)^{-n}\right) i}{1 - (1 + i)^{-n}}
\qquad
A_{\text{anticipata}} = \frac{A_{\text{posticipata}}}{1 + i}
\qquad
A = \frac{P - B}{n} \text{ quando } i = 0
$$

Il motore calcola $A$ come $(P - B\,(1 + i)^{-n})$ diviso il fattore di rendita di
[`tvm.annuity`](tvm-annuity.md): i due metodi condividono un'unica implementazione.

### Il tasso periodico

Il tasso si indica in una sola di due forme:

- **Forma (a)**: un tasso annuo nominale (TAN) `rate` con la sua capitalizzazione
  `compounding` e la periodicità della rata `payment_frequency` ($p$ rate all'anno). Il tasso
  periodico è il tasso equivalente con capitalizzazione pari alla periodicità della rata (come
  lo calcola [`rates.convert`](rates-convert.md)), diviso $p$. Quando la capitalizzazione
  coincide con la periodicità della rata è semplicemente `rate` / $p$: il 6% con
  capitalizzazione mensile e rate mensili dà lo 0,5% al mese.
- **Forma (b)**: un `periodic_rate`, il tasso per periodo di pagamento, come lo accetta
  [`tvm.annuity`](tvm-annuity.md).

Non esiste una capitalizzazione predefinita: un tasso annuo nominale senza capitalizzazione
non ha significato.

## Esempio

L'esempio svolto di ECMA-376 (10.000 all'8% annuo, capitalizzazione mensile e rate mensili,
10 rate) e un finanziamento decennale con un balloon di 50.000, con la rata arrotondata al
centesimo:

```python
from decimal import Decimal

import fairmark as fm

monthly = {"rate": "0.08", "compounding": "monthly", "payment_frequency": "monthly"}
arrears = fm.loans.instalment(10000, 10, timing="ordinary", **monthly)
advance = fm.loans.instalment(10000, 10, timing="due", **monthly)
assert round(arrears.instalment, 2) == Decimal("1037.03")
assert round(advance.instalment, 2) == Decimal("1030.16")

balloon = fm.loans.instalment(
    250000, 120, rate="0.054", compounding="monthly", payment_frequency="monthly",
    timing="ordinary", balloon_amount=50000, decimal_places=2, rounding_mode="half_up",
)
assert balloon.instalment == Decimal("2385.63")
assert balloon.total_paid == Decimal("336275.60")  # 120 x 2385.63 + 50000
print(balloon.conventions[-2])  # rounding: 2 decimal places, half_up, applied to the instalment

# Lo stesso finanziamento con il tasso periodico indicato direttamente (forma b).
same = fm.loans.instalment(20000, 36, periodic_rate="0.005", timing="ordinary")
again = fm.loans.instalment(20000, 36, rate="0.06", compounding="monthly",
                            payment_frequency="monthly", timing="ordinary")
assert same.instalment == again.instalment
```

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `principal` | Capitale finanziato, in unità di valuta, maggiore di 0 | obbligatorio |
| `rate` | Tasso annuo nominale, frazione decimale (forma a) | una forma di tasso obbligatoria |
| `compounding` | Capitalizzazione di `rate`: `annual`, `semi_annual`, `quarterly`, `monthly`, `continuous` (forma a) | obbligatorio con `rate` |
| `payment_frequency` | Rate all'anno: `annual`, `semi_annual`, `quarterly`, `monthly` (forma a) | obbligatorio con `rate` |
| `periodic_rate` | Tasso per periodo di pagamento, frazione decimale, maggiore di −1 (forma b) | una forma di tasso obbligatoria |
| `number_of_instalments` | Da 1 a 10000 | obbligatorio |
| `timing` | `ordinary` (rata posticipata) o `due` (rata anticipata) | obbligatorio |
| `balloon_amount` | Rimborsato alla fine dell'ultimo periodo oltre l'ultima rata, $0 \le B < P$ | `0` |
| `rounding` | `decimal_places` (da 0 a 6) e `mode` (`half_up` o `half_even`), entrambi obbligatori | nessuno |

## Output

| Campo | Significato |
| --- | --- |
| `instalment` | Rata costante (arrotondata quando è indicato `rounding`) |
| `periodic_rate` | Tasso per periodo di pagamento utilizzato |
| `total_paid` | $n \times A + B$, con la rata restituita |
| `total_interest` | Totale pagato meno il capitale |

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Forma del tasso | (a) tasso annuo con capitalizzazione e periodicità della rata, oppure (b) tasso periodico | esattamente una, obbligatoria |
| Tasso periodico (forma a) | tasso equivalente con capitalizzazione pari alla periodicità della rata, diviso il numero di rate all'anno | fissa |
| Momento di pagamento | `ordinary`: rata $k$ alla fine del periodo $k$; `due`: all'inizio | nessuno: va scelto |
| Balloon | pagato con l'ultima rata, alla fine del periodo $n$ per entrambi i momenti di pagamento | 0, riportato come default |
| Arrotondamento | nessuno, oppure da 0 a 6 decimali con `half_up` (la metà si arrotonda per eccesso in valore assoluto) o `half_even` (la metà si arrotonda alla cifra pari) | nessuno |
| Tasso zero | forma limite $(P - B) / n$, mostrata come passaggio separato | applicata quando $i = 0$ |

Con l'arrotondamento, l'ultima rata di un piano effettivo differisce dalle altre per
l'arrotondamento accumulato: [`loans.amortisation_schedule`](loans-amortisation-schedule.md)
calcola questa rettifica dell'ultima rata. Qui `total_paid` usa la rata arrotondata per ogni
periodo.

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | Capitale pari a 0 o negativo, `number_of_instalments` pari a 0 o superiore a 10000, entrambe le forme di tasso, nessuna, forma (a) incompleta, `decimal_places` superiore a 6, arrotondamento senza modalità, oppure un campo mancante o sconosciuto |
| `FM-E003` | Il tasso periodico è pari o inferiore a −100% |
| `FM-E020` | Il balloon è negativo o non inferiore al capitale |
| `FM-E008` | Overflow aritmetico (input molto al di fuori di un intervallo realistico) |

## Precisione

La rata usa potenze intere, moltiplicazioni e divisioni. Quando la capitalizzazione è diversa
dalla periodicità della rata, la conversione usa le funzioni in serie della libreria decimale
(errore relativo vicino a $10^{-27}$). I casi di riferimento sono verificati a $10^{-20}$
rispetto a un calcolo indipendente a 50 cifre.

## Verificato con

- Casi di riferimento in `tests/golden/loans.instalment/`: l'esempio svolto di ECMA-376 della
  rata di un finanziamento (posticipata e anticipata) e un calcolo indipendente a 50 cifre
  (`scripts/reference_loans.py`).
- `numpy_financial.pmt` su finanziamenti casuali, per entrambi i momenti di pagamento, con il
  balloon come valore futuro.
