---
source: methods/loans-number-of-instalments.md
source_hash: 0d63ed881f78
---
# Numero delle rate (`loans.number_of_instalments`)

Restituisce quante rate di un importo dato rimborsano un finanziamento: il numero esatto di
periodi, il numero intero delle rate e l'importo dell'ultima rata, ridotta, che porta il debito
residuo esattamente a zero. Si usa quando il debitore può sostenere una certa rata e la
domanda è quanto dura il finanziamento.

## Formula

Con il tasso periodico $i$, il capitale $P$ e la rata $A$:

$$
n^* = \frac{-\ln\left(1 - \dfrac{i\,P'}{A}\right)}{\ln(1 + i)}
\qquad
P' =
\begin{cases}
P & \text{rata posticipata} \\
P / (1 + i) & \text{rata anticipata}
\end{cases}
\qquad
n^* = \frac{P}{A} \text{ quando } i = 0
$$

Il numero delle rate è $N = \lceil n^* \rceil$. L'ultima rata è l'importo dovuto alla sua data
di pagamento, dalla ricorrenza del debito residuo:

$$
b_1 = P\,(1 + i) \text{ (posticipata) oppure } P \text{ (anticipata)}, \qquad
b_k = (b_{k-1} - A)(1 + i), \qquad
\text{ultima rata} = b_N
$$

Il tasso periodico si ricava come in [`loans.instalment`](loans-instalment.md): un tasso annuo
nominale con capitalizzazione e periodicità della rata, oppure un tasso periodico.

## Esempio

Un finanziamento di 3.000 al 2% trimestrale rimborsato con rate trimestrali di 800 richiede
quattro rate, l'ultima ridotta:

```python
from decimal import Decimal

import fairmark as fm

term = fm.loans.number_of_instalments(3000, 800, periodic_rate="0.02", timing="ordinary")
assert term.number_of_instalments == 4
assert term.final_instalment == Decimal("750.01008")  # (((3000 x 1.02 - 800) x 1.02 - 800) x 1.02 - 800) x 1.02
assert abs(term.exact_number_of_periods - Decimal("3.936929191448856778307291")) < Decimal("1e-20")

# Rate mensili anticipate di 500, tasso annuo effettivo del 7%.
advance = fm.loans.number_of_instalments(
    15000, 500, rate="0.07", compounding="annual", payment_frequency="monthly", timing="due"
)
assert advance.number_of_instalments == 33
```

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `principal` | Capitale finanziato, in unità di valuta, maggiore di 0 | obbligatorio |
| `rate`, `compounding`, `payment_frequency` | Tasso annuo nominale con la sua capitalizzazione e il numero di rate all'anno (forma a) | una forma di tasso obbligatoria |
| `periodic_rate` | Tasso per periodo di pagamento, frazione decimale, maggiore di −1 (forma b) | una forma di tasso obbligatoria |
| `instalment` | Rata costante, in unità di valuta | obbligatorio |
| `timing` | `ordinary` (rata posticipata) o `due` (rata anticipata) | obbligatorio |

Questa versione del metodo non accetta un balloon.

## Output

| Campo | Significato |
| --- | --- |
| `periodic_rate` | Tasso per periodo di pagamento utilizzato |
| `exact_number_of_periods` | $n^*$ |
| `number_of_instalments` | $N = \lceil n^* \rceil$, da 1 a 10000 |
| `final_instalment` | Importo dell'ultima rata, che porta il debito residuo a zero |

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Forma del tasso | (a) tasso annuo con capitalizzazione e periodicità della rata, oppure (b) tasso periodico | esattamente una, obbligatoria |
| Momento di pagamento | `ordinary`, `due` | nessuno: va scelto |
| Numero intero di periodi | un $n^*$ entro $10^{-15}$ da un intero è considerato pari a quell'intero | fissa |
| Ultima rata | importo dovuto alla sua data di pagamento, debito residuo esattamente zero | fissa |
| Tasso zero | $n^* = P / A$, mostrato come passaggio separato | applicato quando $i = 0$ |

La tolleranza sui numeri interi esiste perché i logaritmi sono approssimazioni in serie: una
rata calcolata a piena precisione da [`loans.instalment`](loans-instalment.md) restituisce il
proprio numero di rate invece di una rata in più pari a una frazione di centesimo.

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | Capitale pari a 0 o negativo, più di 10000 rate necessarie, forma di tasso incompleta o doppia, un campo `balloon_amount`, oppure un campo mancante o sconosciuto |
| `FM-E003` | Il tasso periodico è pari o inferiore a −100% |
| `FM-E021` | La rata è pari a 0 o negativa, oppure non supera $i\,P'$, la quota interessi del primo periodo, qualunque sia il segno del tasso: il debito residuo non arriva mai a zero |
| `FM-E008` | Overflow aritmetico (input molto al di fuori di un intervallo realistico) |

## Precisione

$n^*$ usa la serie del logaritmo della libreria decimale (errore relativo vicino a
$10^{-27}$); l'ultima rata usa solo moltiplicazioni e sottrazioni. I casi di riferimento sono
verificati a $10^{-20}$ rispetto a un calcolo indipendente a 50 cifre.

## Verificato con

- Casi di riferimento in `tests/golden/loans.number_of_instalments/`, da un calcolo
  indipendente a 50 cifre (`scripts/reference_loans.py`), compresa la rata dell'esempio svolto
  di ECMA-376, che restituisce esattamente 10 rate.
- `numpy_financial.nper` su finanziamenti casuali, per entrambi i momenti di pagamento.
