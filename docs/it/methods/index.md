---
source: methods/index.md
source_hash: 074a12726840
---
# Metodi

Ogni metodo ha un identificativo stabile, un'implementazione versionata, casi di
riferimento e una pagina metodologica. Il [riferimento dei metodi](../reference/methods.md)
elenca tutti gli input.

| Metodo | Stato | Che cosa calcola |
| --- | --- | --- |
| [`dates.year_fraction`](dates-year-fraction.md) | Disponibile | Frazione d'anno tra due date secondo una convenzione di calcolo dei giorni ISDA |
| [`rates.convert`](rates-convert.md) | Disponibile | Tasso annuo equivalente con un'altra convenzione di capitalizzazione |
| [`rates.real_nominal`](rates-real-nominal.md) | Disponibile | Tasso reale da un tasso nominale, o viceversa (relazione di Fisher) |
| [`tvm.present_value`](tvm-present-value.md) | Disponibile | Valore attuale di flussi periodici con un unico tasso o un tasso per periodo |
| [`tvm.npv`](tvm-npv.md) | Disponibile | Valore attuale netto di una serie di flussi |
| [`tvm.xnpv`](tvm-xnpv.md) | Disponibile | Valore attuale netto di flussi datati |
| [`tvm.irr`](tvm-irr.md) | Disponibile | Tasso interno di rendimento |
| [`tvm.xirr`](tvm-xirr.md) | Disponibile | Tasso interno di rendimento di flussi datati |
| [`tvm.mirr`](tvm-mirr.md) | Disponibile | Tasso interno di rendimento modificato |
| [`tvm.annuity`](tvm-annuity.md) | Disponibile | Valore attuale e valore futuro di una rendita costante o crescente |
| [`tvm.perpetuity`](tvm-perpetuity.md) | Disponibile | Valore attuale di una rendita perpetua costante o crescente |
| [`dcf.fcff`](dcf-fcff.md) | Anteprima | Enterprise value ed equity value dai flussi di cassa operativi |

## Prossimamente

- **M2, DCF completo**: FCFF e FCFE dal bilancio d'esercizio, WACC e CAPM, valore terminale
  con multiplo di uscita, griglie di sensitività.
- **M4, prassi europea**: metodi patrimoniale, reddituale e misto con stima autonoma
  dell'avviamento, EVA, multipli di mercato.
- **Real estate**: DCF immobiliare, capitalizzazione diretta, debito e waterfall.

Vuoi un metodo? Apri una
[richiesta di metodo](https://github.com/salvo8519/fairmark/issues/new/choose).
