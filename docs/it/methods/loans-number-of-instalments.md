---
source: methods/loans-number-of-instalments.md
source_hash: 589d9c07385e
---
# Numero delle rate (`loans.number_of_instalments`)

Restituisce quante rate di un importo dato rimborsano un finanziamento: il numero esatto di
periodi, il numero intero delle rate e l'importo dell'ultima rata, che porta il debito
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
quattro rate, l'ultima inferiore alle altre:

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
| Numero intero di periodi | un $n^*$ entro $10^{-20}$ da un intero è considerato pari a quell'intero; $N$ è poi confermato dalla ricorrenza del debito residuo: la prima rata il cui importo dovuto supera $A$ al massimo di $10^{-20}$ del capitale | fissa |
| Ultima rata | importo dovuto alla sua data di pagamento, debito residuo esattamente zero: inferiore ad $A$ quando $n^*$ non è intero, superiore ad $A$ al massimo di $10^{-20}$ del capitale quando $n^*$ è considerato intero | fissa |
| Tasso zero | $n^* = P / A$, mostrato come passaggio separato | applicato quando $i = 0$ |

La tolleranza sui numeri interi e la ricorrenza esistono perché una rata calcolata a piena
precisione da [`loans.instalment`](loans-instalment.md) restituisca il proprio numero di rate,
invece di una rata in più pari a una frazione di centesimo.

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | Capitale pari a 0 o negativo, più di 10000 rate necessarie, forma di tasso incompleta o doppia, un campo `balloon_amount`, oppure un campo mancante o sconosciuto |
| `FM-E003` | `periodic_rate` è pari o inferiore a −100%, oppure `rate` è pari o inferiore a −100% per periodo di capitalizzazione ($r/m$); il messaggio indica il campo, `/periodic_rate` o `/rate` |
| `FM-E021` | La rata è pari a 0 o negativa, oppure non supera $i\,P'$, la quota interessi del primo periodo, qualunque sia il segno del tasso: il debito residuo non arriva mai a zero |
| `FM-E008` | Overflow aritmetico (input molto al di fuori di un intervallo realistico) |

## Precisione

$n^*$ si calcola come $q \times \dfrac{\ln(1 - i q)/(-i q)}{\ln(1 + i)/i}$ con $q = P'/A$: i
rapporti $\ln(1 + x)/x$ si sommano dalla loro serie quando $|x| < 0.01$, quindi anche i tassi
piccoli, positivi o negativi, conservano tutte le cifre; $n^*$ coincide con un riferimento
indipendente a 50 cifre entro circa $10^{-24}$, anche con tassi di $\pm 10^{-9}$ e $-10^{-11}$
per periodo. Il numero delle rate e l'ultima rata non dipendono da questa precisione: $N$ è
confermato dalla ricorrenza del debito residuo, che usa solo moltiplicazioni e sottrazioni ed è
esatta fino all'ultima delle 28 cifre significative del motore. I casi di riferimento sono
verificati a $10^{-20}$.

## Verificato con

- Casi di riferimento in `tests/golden/loans.number_of_instalments/`, da un riferimento
  indipendente a 50 cifre (`scripts/reference_loans.py`), compresa la rata dell'esempio svolto
  di ECMA-376, che restituisce esattamente 10 rate, e tassi negativi piccoli su 1200 e 10000
  periodi.
- La funzione del numero di periodi di `numpy_financial` su finanziamenti casuali, per entrambi
  i momenti di pagamento.
