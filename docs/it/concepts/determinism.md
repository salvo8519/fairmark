---
source: concepts/determinism.md
source_hash: c8ce6129a22f
---
# Il contratto di determinismo

Fairmark garantisce che lo stesso input produca lo stesso output, fino all'ultima cifra, su
ogni piattaforma e versione di Python supportate, per una data versione del motore e del
metodo. Questo documento è il contratto; la CI lo verifica a ogni commit.

## Regole

1. **Solo aritmetica decimale.** Tutti i calcoli usano `rust_decimal` (mantissa a 96 bit,
   28–29 cifre significative). Nessun `f32`/`f64` compare in un percorso di calcolo.
2. **Lettura esatta.** I numeri JSON sono convertiti direttamente in decimali
   (`arbitrary_precision`), mai tramite la virgola mobile binaria. Gli output serializzano i
   decimali come stringhe, così nessun consumatore (JavaScript, fogli di calcolo) perde
   precisione senza accorgersene.
3. **Le potenze intere sono esatte.** I fattori di sconto con esponente intero usano la
   moltiplicazione ripetuta. Gli esponenti frazionari (convenzione di metà periodo) usano
   `powd` decimale, la cui precisione è verificata nei casi di riferimento contro
   un'implementazione indipendente a 50 cifre.
4. **Algoritmi iterativi riproducibili.** I metodi iterativi (TIR e, in futuro, il WACC
   circolare) usano algoritmo, punto di partenza, tolleranza e numero massimo di iterazioni
   fissi. L'algoritmo usato e il numero di iterazioni sono riportati nel risultato.
5. **Nessun valore predefinito implicito.** I default esistono solo dove documentati nello
   schema di input. I campi sconosciuti sono rifiutati (`FM-E001`): un refuso non ricade mai
   su un valore predefinito.
6. **Impronta dell'input.** `meta.input_hash` è lo SHA-256 dell'input canonico.
   `fairmark verify` riesegue un risultato salvato e ne confronta hash e valore.
7. **Versionamento numerico.** Se un caso di riferimento cambia risultato a parità di input,
   la PR riceve l'etichetta `numerical-change`, incrementa `method_version` e porta nel
   commit la riga `Numerical-change: <metodo> v<N> -> v<N+1>`, che il CHANGELOG generato
   elenca per prima.

## Casi di riferimento (golden case)

`tests/golden/<metodo>/*.yaml` contiene coppie input/risultato atteso revisionate. I valori
attesi provengono da fonti indipendenti: implementazioni di riferimento
(`scripts/reference_values.py`), esempi svolti da professionisti o esempi pubblicati e
ricalcolati a mano. Le suite di test Rust e Python eseguono ogni caso.

Non vanno mai inseriti dati reali di clienti: i casi si anonimizzano o si ricostruiscono.
