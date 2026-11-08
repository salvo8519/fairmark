---
source: guides/coding-agents.md
source_hash: e9cda634f2ec
---
# Claude Code, Codex e altri agenti di sviluppo

Gli agenti di sviluppo lavorano al meglio con una CLI e una skill che spiega come usarla.

## Installa la skill

Copia `skills/fairmark/` dal repository nella cartella delle skill del tuo agente (per Claude
Code: `.claude/skills/fairmark/` nel progetto, oppure `~/.claude/skills/fairmark/`).

La skill chiede all'agente di:

1. scoprire i metodi con `fairmark methods` (ciascuno con il suo stato, `available` o
   `preview`) e gli input con `fairmark schema <metodo>`, e dichiarare quando un risultato
   proviene da un metodo in anteprima;
2. scrivere gli input estratti in un file JSON ed eseguire `fairmark run`;
3. riportare i valori esattamente come restituiti, citando `explain.steps` e le convenzioni;
4. non calcolare mai da solo le cifre della valutazione.

## Senza la skill

La CLI è autodescrittiva, quindi un agente di solito la capisce da `fairmark --help`. La skill
rende il comportamento coerente e aggiunge le regole per il report.
