---
source: methods/rates-convert.md
source_hash: fbdbf60c15ec
---
# Conversione di tassi (`rates.convert`)

Converte un tasso annuo espresso con una convenzione di capitalizzazione nel tasso annuo
equivalente espresso con un'altra. Due tassi sono equivalenti quando producono la stessa
crescita di un'unità di capitale in un anno. Serve per riconciliare il tasso di un
finanziamento a capitalizzazione mensile con un costo del capitale espresso come tasso annuo
effettivo, oppure per passare da tassi periodali a tassi a capitalizzazione continua prima
dell'attualizzazione.

## Formula

La crescita di un'unità in un anno al tasso $r$ con $m$ periodi di capitalizzazione all'anno è

$$
G = \left(1 + \frac{r}{m}\right)^{m}
\qquad\text{or}\qquad
G = e^{r}\ \text{(continuous compounding)}
$$

Il tasso equivalente con capitalizzazione di destinazione $m'$ è

$$
r' = m' \left(G^{1/m'} - 1\right)
\qquad\text{or}\qquad
r' = \ln G\ \text{(continuous compounding)}
$$

Per una destinazione annuale ($m' = 1$) si ha semplicemente $r' = G - 1$.

| Capitalizzazione | Valore di `from` / `to` | $m$ |
| --- | --- | --- |
| Annuale (tasso annuo effettivo) | `annual` | 1 |
| Semestrale | `semi_annual` | 2 |
| Trimestrale | `quarterly` | 4 |
| Mensile | `monthly` | 12 |
| Continua | `continuous` | nessuno, crescita $e^{r}$ |

## Esempio

Un tasso del 12% annuo a capitalizzazione mensile corrisponde a un tasso annuo effettivo di
$(1 + 0.12/12)^{12} - 1 = 12.6825\ldots\%$. Il calcolo usa solo potenze intere, quindi il
risultato è esatto alle 28 cifre del tipo decimale.

```python
from decimal import Decimal

import fairmark as fm

r = fm.rates.convert("0.12", "monthly", "annual")
print(r.rate)                  # 0.126825030131969720661201
print(r.annual_growth_factor)  # 1.126825030131969720661201
assert r.rate == Decimal("1.01") ** 12 - 1

# 10% annuo effettivo come tasso a capitalizzazione continua: ln(1.1).
c = fm.rates.convert("0.10", "annual", "continuous")
assert abs(c.rate - Decimal("0.09531017980432486004")) < Decimal("1e-15")
```

In Python l'argomento per la capitalizzazione di partenza è `from_` (con il trattino basso
finale, perché `from` è una parola riservata di Python); in JSON, nella CLI e nel server MCP il
campo è `from`.

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `rate` | Tasso annuo come frazione decimale (0.12 = 12% annuo), espresso con la capitalizzazione `from` | obbligatorio |
| `from` | Capitalizzazione del tasso fornito | obbligatorio |
| `to` | Capitalizzazione del tasso da restituire | obbligatorio |

## Output

| Campo | Significato |
| --- | --- |
| `rate` | Tasso annuo equivalente con capitalizzazione `to`, frazione decimale |
| `annual_growth_factor` | Crescita di un'unità in un anno, identica per i due tassi |

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Capitalizzazione di partenza (`from`) | annuale, semestrale, trimestrale, mensile, continua | nessuno: un tasso senza la sua capitalizzazione è ambiguo |
| Capitalizzazione di destinazione (`to`) | stesse opzioni | nessuno |
| Espressione del tasso | i tassi sono sempre espressi su base annua: un tasso di 0.12 a capitalizzazione mensile significa 1% al mese | fissa |

Quando `from` e `to` coincidono il tasso viene restituito invariato.

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | Un campo è mancante o sconosciuto, oppure una capitalizzazione non è una delle cinque opzioni |
| `FM-E003` | Il tasso per periodo di capitalizzazione, $r/m$, è pari o inferiore a −100%; il messaggio indica `/rate` e, con più di un periodo di capitalizzazione all'anno, il limite $-m$ |
| `FM-E008` | L'esponenziale o il logaritmo va in overflow (tassi molto al di fuori di un intervallo realistico) |

## Precisione

La conversione in tasso annuo a partire da un tasso periodale usa potenze intere esatte;
rispetto a un calcolo indipendente a 60 cifre, concorda entro circa $10^{-24}$ in termini
relativi o meglio. Le conversioni che coinvolgono la capitalizzazione continua o una
destinazione non annuale usano l'esponenziale e il logaritmo decimali, che concordano entro
circa $10^{-18}$ in termini relativi o meglio su input tipici. Una conversione seguita dalla
sua inversa restituisce il tasso originale entro $10^{-20}$. I casi di riferimento sono
verificati entro $10^{-19}$.

## Verificato con

- Casi di riferimento (golden case) in `tests/golden/rates.convert/`, da un calcolo
  indipendente a 50 cifre (`scripts/reference_time.py`).
- Le conversioni tra una capitalizzazione periodale e `annual` concordano con le funzioni
  `EFFECT` e `NOMINAL` dei fogli di calcolo (Excel, LibreOffice).
