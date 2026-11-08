# SPEC-0009 — Loans: instalment, number of instalments, future value and amortisation schedules

| | |
| --- | --- |
| Status | Approved — 2026-11-08, maintainer decisions recorded below |
| Milestone | M1 |
| Owner | @salvo8519 |
| Formula basis | Closed forms of the level-payment (annuity) loan and the period-by-period recurrence of the outstanding balance, as commonly adopted in lending practice and in the amortisation types defined by Regulation (EU) 2016/867 (AnaCredit); verified against the ECMA-376 worked examples of the spreadsheet loan functions, at least one worked example from a published financial mathematics textbook or a regulatory example, an independent 50-digit reference script and `numpy_financial` |
| Related | Issues #135 (this spec), #136, #137; SPEC-0002 (rates, timing, annuities); issue #85 (M6, debt schedule) |

## Problem
Analysts value companies that carry bank debt and build business plans that include new
loans. They need the instalment of a loan, the number of instalments needed to repay it, the
future value of an amount and a series of payments, and the full amortisation schedule with
the interest portion, the principal portion and the outstanding balance of every period. In
spreadsheets these figures depend on a sign convention, an implicit compounding and an
implicit rounding, and the last instalment rarely brings the balance exactly to zero. Italian
practice also distinguishes constant-instalment (French) from constant-principal
amortisation and uses an initial interest-only period (preammortamento); an analyst must be
able to state which one applies and see it in the audit trail.

## Scope
**In scope:**
- Instalment of a constant-instalment loan, with an optional balloon amount.
- Number of instalments needed to repay a loan with a given instalment.
- Future value of a present amount and a level series of periodic payments.
- Amortisation schedules for three amortisation types: French (constant instalment),
  fixed amortisation schedule (constant principal portion) and bullet; an optional principal
  grace period (interest only) at the start; an optional balloon amount at maturity.
- Payments in arrears or in advance for the instalment and the future value; payments in
  arrears for schedules. The periodic rate is derived from an annual borrowing rate or given
  directly. An explicit rounding convention for the instalment and a final-period adjustment
  that brings the outstanding balance exactly to zero.

**Out of scope:**
- Debt schedule of a company, levered returns and equity waterfall: issue #85 (M6).
- Dated schedules with calendar dates, actual day counts, business-day rolling and a broken
  first period (the "preammortamento tecnico" that aligns the first instalment to a calendar
  date). Periods here are equal fractions of a year.
- Variable or indexed rates, rate resets, caps and floors, prepayments and renegotiation.
- Fees, charges and the annual percentage rate of charge (APRC). The rate input of this
  spec is the borrowing rate; the APRC is a different quantity (see "Terminology and
  sources").
- Schedules with payments in advance: planned for a later method version.
- Amortisation types other than the three above (for example the AnaCredit "German" type,
  rising instalments, sinking funds).

## User stories
- As an analyst, I want the instalment of a loan from its principal, annual borrowing rate,
  payment frequency and number of instalments, so that I can put new debt into a business
  plan.
- As an analyst, I want the full amortisation schedule of an existing loan, French or
  Italian-style constant principal, with an initial interest-only period, so that the interest
  and the outstanding balance of every year feed the plan and the net financial position.
- As an analyst, I want instalments rounded to the cent as the bank does and a last instalment
  that clears the balance exactly, so that the schedule reconciles with the bank statement.
- As an AI agent, I want each convention (timing, rate conversion, rounding, final adjustment)
  to be an explicit input or a reported convention, so that I never state a figure that rests
  on an assumption the user did not see.

## Requirements (EARS)

### Common to all loan methods
- **R1** — THE ENGINE SHALL accept the rate in exactly one of two forms: (a) an annual
  borrowing rate `rate`, decimal fraction, with its `compounding` (the five options of
  SPEC-0002 R3) and the `payment_frequency` (annual, semi-annual, quarterly, monthly), all three
  required; or (b) a `periodic_rate`, decimal fraction per payment period, as `tvm.annuity`
  accepts. In form (a) it SHALL derive the periodic rate as the equivalent annual rate with
  compounding equal to the payment frequency, divided by the number of payments per year,
  reusing the conversion of `rates.convert`. There is no default compounding.
- **R1a** — IF both forms, neither form, or form (a) without its compounding or payment
  frequency is given, THEN THE ENGINE SHALL return `FM-E001` naming the missing or
  conflicting fields.
- **R2** — THE ENGINE SHALL accept the payment timing `ordinary` (in arrears: each instalment
  at the end of its period) or `due` (in advance: each instalment at the beginning of its
  period), the same values as `tvm.annuity` (SPEC-0002 R11). The timing is required.
  `loans.instalment` and `tvm.future_value` accept both values; `loans.number_of_instalments`
  accepts both; `loans.amortisation_schedule` accepts `ordinary` only in version 1 and returns
  `FM-E001` for `due`.
- **R3** — IF the principal is zero or negative, the number of instalments is 0 or above
  10000, or a field is missing or unknown, THEN THE ENGINE SHALL return `FM-E001`; IF the
  periodic rate is −100% or lower, THEN it SHALL return `FM-E003`.
- **R4** — WHEN the periodic rate is zero, THE ENGINE SHALL use the limit forms (instalment
  (P − B) / n, number of instalments (P − B) / A) and show them as a separate step.
- **R5** — Every loan result SHALL list in `explain.conventions` the timing, the rate form
  used, the source compounding and payment frequency (form (a)), the periodic rate, the
  rounding convention and every default applied; and in `explain.steps` the periodic-rate
  derivation with the numbers substituted (form (a)) or the periodic rate as given (form (b)).

### Instalment (issue #136)
- **R6** — WHEN given a principal P, the rate inputs of R1, a number of instalments n, a
  timing and an optional balloon amount B (default 0), THE ENGINE SHALL return the constant
  instalment A such that the present value at the periodic rate i of the n instalments and of
  B paid at the end of period n equals P:
  A = (P − B (1 + i)^−n) · i / (1 − (1 + i)^−n) for `ordinary`, divided by (1 + i) for `due`;
  together with the total amount paid and the total interest.
- **R7** — IF the balloon amount is negative or not lower than the principal, THEN THE ENGINE
  SHALL return `FM-E020`.

### Number of instalments (issue #136)
- **R8** — WHEN given a principal, the rate inputs of R1, an instalment A and a timing (no
  balloon amount in version 1; a `balloon_amount` field is rejected as unknown), THE
  ENGINE SHALL return the exact number of periods n* = −ln(1 − i P' / A) / ln(1 + i), where
  P' = P for `ordinary` and P / (1 + i) for `due`; the number of instalments ⌈n*⌉; and the
  amount of the final, reduced instalment that brings the outstanding balance exactly to zero.
- **R9** — IF A ≤ i P' (the instalment does not exceed the interest accruing on the
  balance, so the balance never decreases), THEN THE ENGINE SHALL return `FM-E021`; IF ⌈n*⌉
  exceeds 10000, THEN it SHALL return `FM-E001`.

### Future value (issue #136)
- **R10** — WHEN given a present amount PV (default 0), a level periodic payment PMT
  (default 0), a rate per period r, a number of periods n and a timing, THE ENGINE SHALL return
  the future value at t = n: FV = PV (1 + r)^n + PMT · ((1 + r)^n − 1) / r, the payment term
  multiplied by (1 + r) for `due`, and n · PMT for the payment term when r = 0. Amounts of the
  same sign are accumulated in the same direction; a negative amount is a withdrawal. The rate
  per period follows the `tvm` convention of SPEC-0002 (no annual-rate conversion). Both
  timings are accepted. There is no separate outstanding-balance method: the balance after any
  instalment is read from the schedule (R12).
- **R11** — THE ENGINE SHALL report the sign convention of R10 in `explain.conventions`, and
  `FM-E001` IF both PV and PMT are omitted.

### Amortisation schedule (issue #137)
- **R12** — WHEN given a principal, the rate inputs of R1, a number of instalments n (the full
  term, grace period included), the timing `ordinary`, an amortisation type, an optional principal grace
  period g (default 0) and an optional balloon amount B (default 0), THE ENGINE SHALL return
  one row per period with: period number, payment time, instalment, interest portion,
  principal portion and outstanding balance after the instalment; and the totals of
  instalments, interest and principal.
- **R13** — WHEN the amortisation type is `french`, THE ENGINE SHALL amortise P − B over the
  n − g periods after the grace period with the constant instalment of R6, the interest
  portion of each period being i times the outstanding balance at the start of the period.
- **R14** — WHEN the amortisation type is `fixed_amortisation_schedule`, THE ENGINE SHALL
  repay (P − B) / (n − g) of principal in each period after the grace period, the interest
  portion being i times the outstanding balance at the start of the period.
- **R15** — WHEN the amortisation type is `bullet`, THE ENGINE SHALL charge interest only in
  every period and repay the full principal with the last instalment; IF a balloon amount or a
  grace period is given with `bullet`, THEN it SHALL return `FM-E020` or `FM-E022`
  respectively.
- **R16** — WHEN a principal grace period g > 0 is given, THE ENGINE SHALL charge interest
  only in the first g periods; IF g is not lower than n, THEN it SHALL return `FM-E022`.
- **R17** — WHEN a balloon amount B > 0 is given, THE ENGINE SHALL repay B together with the
  last instalment and report it as a separate field of the last row; IF B is negative or not
  lower than the principal, THEN it SHALL return `FM-E020`.
- **R18** — Instalment k of a schedule is paid at the end of period k (time k). Schedules with
  payments in advance are not part of version 1 (R2).
- **R19** — THE ENGINE SHALL accept an optional rounding convention: number of decimal places
  from 0 to 6 and a rounding mode, `half_up` or `half_even`, both stated explicitly when
  rounding is chosen (no default mode); without the field there is no rounding. Rounding
  applies to the amount that the schedule type keeps constant, so that the type keeps its
  meaning: the instalment of `loans.instalment` and of a French schedule, and the principal
  portion of a fixed amortisation schedule. Interest portions are not rounded. In a French
  schedule the principal portion of each period is the rounded instalment minus the interest
  portion; in a fixed amortisation schedule the instalment is the rounded principal portion
  plus the interest portion. A bullet schedule has no constant amount to round other than
  its interest, which is not rounded, so rounding does not change it. The outstanding balance
  follows from the principal portions.
- **R20** — THE ENGINE SHALL set the principal portion of the last period equal to the
  outstanding balance at its start (the balloon amount included), so that the final outstanding balance
  is exactly zero, and SHALL report the difference between the last instalment and the regular
  instalment as `final_adjustment`, with a step in `explain`. This applies with and without
  rounding.
- **R21** — IF, with rounding, the outstanding balance would become negative before the last
  period, or the final adjustment would make the last principal portion or the last
  instalment negative (rounding too coarse for the loan), THEN THE ENGINE SHALL return
  `FM-E023`.
- **R22** — THE ENGINE SHALL include in `explain.steps` the instalment or principal-portion
  formula with the numbers substituted; the recurrence stated once with its formula
  (interest = i × opening balance; principal = instalment − interest; closing balance =
  opening balance − principal); the first period with the numbers substituted; the final
  adjustment; and the totals. The rows of the output carry the per-period numbers.

## Conventions and parameters
| Convention | Options | Default | Source |
| --- | --- | --- | --- |
| Rate form | (a) annual borrowing rate with compounding and payment frequency, or (b) periodic rate | exactly one, required | Directive (EU) 2023/2225 Art. 3(8): rate "applied on an annual basis"; `tvm.annuity` |
| Source compounding of the rate (form a) | annual, semi_annual, quarterly, monthly, continuous | required, no default | SPEC-0002 R3; constitution III |
| Payment frequency (form a) | annual, semi_annual, quarterly, monthly | required | Regulation (EU) 2016/867, "payment frequency" |
| Periodic rate (form a) | equivalent rate with compounding = payment frequency, divided by payments per year | fixed | SPEC-0002 `rates.convert` |
| Payment timing | ordinary (in arrears), due (in advance); schedules: ordinary only in version 1 | required | SPEC-0002 annuity timing |
| Amortisation type | french, fixed_amortisation_schedule, bullet | required | Regulation (EU) 2016/867 Annex IV; Delegated Regulation (EU) 2020/1224 RREL35 |
| Principal grace period | 0 to n − 1 periods, interest only | 0 | Delegated Regulation (EU) 2020/1224 RREL36; World Bank loan terms |
| Balloon amount | 0 ≤ B < P, currency units | 0 | Delegated Regulation (EU) 2020/1224 RREL41 |
| Rounding | none, or decimal places 0 to 6 with an explicit mode half_up or half_even; applies to the amount the schedule type keeps constant (instalment, or principal portion for a fixed amortisation schedule) | none | practice varies: an explicit input |
| Final-period adjustment | last principal portion = opening balance of the last period | always applied, reported | this spec (R20) |
| Future value sign convention | same sign accumulates, negative is a withdrawal | fixed | this spec (R10); differs from the spreadsheet cash-flow sign convention |
| Interest of a period | i × opening balance (compound interest regime) | fixed | Corte di Cassazione 2024 (see sources) |

## Acceptance criteria
- **AC1** (R6, R1) — ECMA-376 worked example: principal 10000, rate 0.08 with monthly
  compounding and monthly payments, 10 instalments: instalment 1037.03 in arrears and 1030.16
  in advance (published to two decimals).
- **AC2** (R6, R7) — Instalments with a balloon amount match the 50-digit reference within
  1e-19; B < 0 and B ≥ P return `FM-E020`.
- **AC3** (R8, R9) — The number of instalments recomputes the instalment of AC1 within 1e-15
  (n* = 10); a non-integer case gives ⌈n*⌉ and a final instalment that matches the reference
  schedule; an instalment equal to i P' returns `FM-E021`; a `balloon_amount` field returns
  `FM-E001`; agreement with `numpy_financial.nper` on ten random loans.
- **AC4** (R10, R11) — ECMA-376 worked examples, converted to this sign convention:
  payment 1000, rate 0.01, 12 periods, ordinary: 12682.50; present amount 500 and payment 200,
  rate 0.005, 10 periods, due: 2581.40; payment 2000, rate 0.11 / 12 (entered to 28 significant digits), 35 periods, due: 82846.25;
  present amount 1000 and payment 100, rate 0.005, 12 periods, due: 2301.40. Omitting both
  amounts returns `FM-E001`.
- **AC5** (R13) — ECMA-376 worked examples of the principal portion: principal 200000, rate
  0.08 annual, 10 annual instalments in arrears: principal portion of period 10 = 27598.05;
  principal 2000, rate 0.10 monthly compounding, 24 monthly instalments: principal portion of
  period 1 = 75.62. Full French schedules match the reference within 1e-19.
- **AC6** (R14, R15) — Fixed amortisation schedule and bullet schedules match the reference;
  `bullet` with a balloon returns `FM-E020`, with a grace period `FM-E022`.
- **AC7** (R16, R17) — A French schedule with a 2-period grace period and a balloon of 20% of
  the principal matches the reference: interest-only rows 1 and 2, the balloon in the last row;
  g ≥ n returns `FM-E022`.
- **AC8** (R2, R18) — `loans.amortisation_schedule` with timing `due` returns `FM-E001`;
  instalment k of a schedule is at time k.
- **AC9** (R19, R20) — With rounding to 2 decimal places `half_up`, every French instalment
  except the last equals the rounded instalment, every fixed-amortisation-schedule principal
  portion except the last equals the rounded constant principal portion, interest portions are
  not rounded, the final balance is
  exactly 0 and `final_adjustment` equals the reference. Rounding without a mode returns
  `FM-E001`. Without rounding, |`final_adjustment`| < 1e-18 × principal while (1 + i)^n ≤ 1e10
  (for example 1200 periods at 1.9% or 470 periods at 5%); beyond that range its size is
  documented in the Accuracy section; the final outstanding balance is exactly 0 in every
  case.
- **AC10** (R21) — Principal 10, rate 0, 20 instalments, French, rounding to 0 decimal places
  `half_up` (instalment 0.5 rounds to 1, the balance reaches zero after 10 periods) returns
  `FM-E023`.
- **AC11** (R4) — Rate zero: instalment (P − B) / n, number of instalments P / A, future value
  PV + n · PMT; each shown as a separate step.
- **AC12** (R1a, R3) — Principal 0, n = 0, n = 10001, an unknown field, both rate forms,
  neither rate form, and an annual rate without compounding return `FM-E001`; a periodic rate
  of −1 returns `FM-E003`.
- **AC12a** (R1) — The same loan given with form (a) and with the equivalent periodic rate in
  form (b) gives the same instalment and schedule.
- **AC13** (R5, R11, R22) — Every new method reports its conventions and the steps listed;
  the schedule explain has a bounded number of steps independent of n (unit test per method).
- **AC14** (property) — For every schedule: the row identities (instalment = interest portion +
  principal portion; closing balance = opening balance − principal portion) and the sum of
  principal portions equal to the principal hold to the last of the engine's 28 significant
  digits; the sum of instalments equals the sum of interest and principal portions; balances
  are non-increasing after the grace period; with no rounding, the present value of the
  instalments at the periodic rate equals the principal within 1e-18 relative.
- **AC15** (R6, R13, R14) — At least one golden case reproduces a worked example from a
  published financial mathematics textbook or a regulatory example (French or fixed
  amortisation schedule), with the source, edition and page cited in the golden file.

## Golden-case sources
- **Reference script**: `scripts/reference_loans.py`, written independently of the Rust code
  from the closed forms and the recurrence above, computed at 50 significant digits with the
  Python `decimal` module (the pattern of `scripts/reference_time.py`).
- **Published worked examples**: ECMA-376 Part 1, SpreadsheetML formulas, the examples of the
  loan instalment, principal portion and future value functions (AC1, AC4, AC5), viewed at
  `https://webapp.docx4java.org/OnlineDemo/ecma376/SpreadsheetML/PMT.html`, `.../PPMT.html`,
  `.../FV.html`. These are used as worked examples only; their function names are not used as
  names in Fairmark.
- **Textbook or regulatory worked example (required)**: at least one schedule from a
  published financial mathematics textbook or a regulatory example, cited with edition and
  page in the golden file (AC15). It is selected by the test engineer before implementation;
  no source is invented, and if none can be obtained the maintainer is asked.
- **Implementations cross-checked**: `numpy_financial` (`pmt`, `nper`, `fv`, `ipmt`, `ppmt`)
  and the spreadsheet functions of Excel and LibreOffice.

## Interfaces
- New method ids:
  - `loans.instalment` — constant instalment (R6, R7).
  - `loans.number_of_instalments` — number of instalments and final instalment (R8, R9).
  - `tvm.future_value` — future value of a present amount and level payments (R10, R11).
    It belongs to the `tvm` theme because its rate is per period, like `tvm.annuity`.
  - `loans.amortisation_schedule` — schedule for the three amortisation types (R12 to R22).
- New module `crates/fairmark-core/src/loans.rs` (theme `loans`); `tvm.future_value` in
  `tvm.rs`. All schema changes are additive (new methods, version 1).
- Input fields (snake_case, units in the schema doc comments): `principal` (currency units),
  `rate` (annual borrowing rate, decimal fraction), `compounding`, `payment_frequency`
  (form (a)) or `periodic_rate` (form (b)),
  `number_of_instalments`, `timing`, `instalment` (currency units), `amortisation_type`,
  `principal_grace_periods` (number of periods), `balloon_amount` (currency units),
  `rounding` (`decimal_places`, `mode`); for `tvm.future_value`: `present_value`, `payment`,
  `rate` (per period), `periods`, `timing`.
- Output fields: `instalment`, `periodic_rate`, `total_paid`, `total_interest`;
  `exact_number_of_periods`, `number_of_instalments`, `final_instalment`; `future_value`;
  `rows` (`period`, `time`, `instalment`, `interest_portion`, `principal_portion`,
  `balloon_amount`, `outstanding_balance`), `totals`, `final_adjustment`.
- New error codes, provisional (`FM-E018` and `FM-E019` are taken by SPEC-0006; the final
  numbers are fixed when the variants are added to `error.rs`):
  - `FM-E020` balloon amount out of range (negative, not lower than the principal, or given
    with `bullet`);
  - `FM-E021` the instalment does not exceed the interest of the first period: the loan is
    never repaid;
  - `FM-E022` principal grace period not lower than the number of instalments, or given with
    `bullet`;
  - `FM-E023` with rounding, the balance would become negative before the last period or the
    final-period adjustment would be negative (rounding too coarse for the loan).
- Python helpers in `python/fairmark/loans.py` and `fm.tvm.future_value`; methodology pages
  `docs/en/methods/loans-instalment.md`, `loans-number-of-instalments.md`,
  `loans-amortisation-schedule.md`, `tvm-future-value.md`.

## Terminology and sources
Method, field and value names use the terminology of international and European financial
practice, verified on the sources below. Spreadsheet function names are not used as names.

| English term (as used in Fairmark) | Italian equivalent | Source (verified 2026-11-08) |
| --- | --- | --- |
| instalment | rata | Directive 2014/17/EU, Annex II Part B, ESIS Section 6 "Amount of each instalment" and Section 7 "amount of the instalment": https://www.legislation.gov.uk/eudr/2014/17/annex/II/part/B ; Banca d'Italia, guide "Comprare una casa: il mutuo ipotecario in parole semplici" (glossary, "Rata"): https://www.bancaditalia.it/PUBBLICAZIONI/GUIDE-BI/GUIDA-MUTUO/INDEX.HTML |
| interest portion | quota interessi | ESIS Section 7, "interest to be paid per instalment" (link above); Banca d'Italia guide (link above): "quota interessi, cioè l'interesse maturato" |
| principal portion | quota capitale | ESIS Section 7, "capital repaid per instalment"; Banca d'Italia guide: "quota capitale, cioè l'importo del finanziamento restituito" |
| outstanding balance | debito residuo | ESIS Section 7, "outstanding capital after each instalment"; Delegated Regulation (EU) 2020/1224 RREL30 "Current Principal Balance": https://www.legislation.gov.uk/eur/2020/1224/pdfs/eur_20201224_adopted_en.pdf ; Corte di Cassazione, abstract 31 January 2023, "la quota interessi è calcolata sul debito residuo": https://www.cortedicassazione.it/resources/cms/documents/Abstract_Intervento_31_gennaio_2023.pdf |
| amortisation schedule | piano di ammortamento | ESIS Section 7, "amortisation table"; Banca d'Italia guide: "Il piano di ammortamento è il progetto di restituzione del debito" |
| amortisation type | tipo di ammortamento | Regulation (EU) 2016/867 (AnaCredit), Annex IV, "Amortisation type": https://www.legislation.gov.uk/eur/2016/867/adopted/data.htm |
| French (constant instalment) | ammortamento alla francese (a rata costante) | Regulation (EU) 2016/867 Annex IV: "the total amount — principal plus interest — repaid in each instalment is the same"; Corte di Cassazione, convegno 31 January 2024, "rata costante": https://www.cortedicassazione.it/resources/cms/documents/2024_01_31_Convegno_31_gennaio_2024_Brochure.pdf |
| fixed amortisation schedule (constant principal portion) | ammortamento all'italiana (a quota capitale costante) | Regulation (EU) 2016/867 Annex IV: "the principal amount repaid in each instalment is the same"; World Bank, "Level Repayment: The principal is repaid in equal installments": https://thedocs.worldbank.org/en/doc/b2536913e9f6e0104c4ecab5e42ca96e-0340012021/original/Instructions-IFL-Loan-Choice-Worksheet-EN.pdf ; Corte di Cassazione abstract 2023, "all'italiana con quote capitali costanti" (link above) |
| bullet | bullet (rimborso integrale del capitale con l'ultima rata) | Regulation (EU) 2016/867 Annex IV: "the full principal amount is repaid in the last instalment"; CFA Institute, refresher reading "Fixed-Income Cash Flows and Types": https://www.cfainstitute.org/insights/professional-learning/refresher-readings/2026/fixed-income-cash-flows-and-types |
| principal grace period | preammortamento | Delegated Regulation (EU) 2020/1224 RREL36 "Principal Grace Period End Date" (link above); World Bank: grace period "in which the borrower pays interest on the loan, but makes no principal repayment" (link above); Corte di Cassazione abstract 2023, "interessi di ammortamento e preammortamento" (link above) |
| balloon amount | balloon (capitale rimborsato alla scadenza oltre la rata) | Delegated Regulation (EU) 2020/1224 RREL41 "Balloon Amount": "principal repayment to be paid at the maturity date" (link above) |
| borrowing rate | tasso annuo nominale (TAN) | Directive (EU) 2023/2225 Art. 3(8), "the interest rate expressed as a fixed or variable percentage applied on an annual basis" (text consulted at https://lawplayer.com/eu/article/32023L2225/art_3); Banca d'Italia, "I tassi di interesse bancari. Guida ai dati pubblicati dalla Banca d'Italia" (Metodi e fonti, 3 March 2023), TAN: "il tasso effettivamente concordato tra creditore e debitore, al netto degli oneri aggiuntivi" |
| annual percentage rate of charge (APRC), not computed here | tasso annuo effettivo globale (TAEG) | Directive (EU) 2023/2225 Art. 3(7); Testo unico bancario art. 121(1)(m), text consulted at https://brocardi.it/testo-unico-bancario/titolo-vi/capo-ii/art121.html |
| payment frequency | periodicità della rata | Regulation (EU) 2016/867 Annex IV, "Payment frequency"; Banca d'Italia guide: "la periodicità delle singole rate (mese, trimestre, semestre)" |
| payment in arrears / in advance | rata posticipata / anticipata | SPEC-0002 annuity timing (`ordinary` / `due`) and `i18n/glossary.yaml` (rendita posticipata / anticipata) |
| number of instalments | numero delle rate | Directive 2008/48/EC Art. 5(1)(h), "the amount, number and frequency of payments": https://www.legislation.gov.uk/eudr/2008/48/chapter/II/data.pdf |
| future value | valore futuro (montante) | CFA Institute curriculum, time value of money (term in common use; accepted by the maintainer, decision 6) |

Notes on the sources: the Banca d'Italia mortgage guide was read in a copy of its text
(https://www.fpcgil.it/wp-content/uploads/2021/06/COMPRARE-UNA-CASA-IL-MUTUO-IPOTECARIO-IN-PAROLE-SEMPLICI.pdf)
because the official PDF could not be retrieved in this session; the Banca d'Italia rate guide
was read in a copy at
https://www.dirittobancario.it/wp-content/uploads/2023/03/I-tassi-di-interesse-Guida-Banca-dItalia.pdf.
Requirement: before the methodology pages and the Italian edition are published, every
quotation from Banca d'Italia texts SHALL be checked on the official files published at
bancaditalia.it, and the citations SHALL point to those files, not to copies.

## Decisions (maintainer, 2026-11-08)
1. **Constant principal type value**: the AnaCredit term `fixed_amortisation_schedule` is kept.
   The documentation explains it as the Italian "ammortamento all'italiana" (constant
   principal portion).
2. **Compounding** is required in form (a); there is no default, consistent with "no silent
   defaults" (R1).
3. **Periodic rate**: the loan methods also accept a periodic rate directly, as `tvm.annuity`
   does; exactly one of the annual rate with compounding and payment frequency, or the
   periodic rate (R1, R1a).
4. **Timing**: schedules support payments in arrears only in version 1; payments in advance
   come in a later method version. `loans.instalment` and `tvm.future_value` keep both timings
   (R2, R18).
5. **Balloon and number of instalments**: `loans.number_of_instalments` does not accept a
   balloon amount in version 1 (R8).
6. **Future value**: `tvm.future_value` is accepted; there is no separate outstanding-balance
   method, the balance is read from the schedule (R10).
7. **Italian terms**: "bullet" and "balloon" stay as English terms with an Italian explanation
   in the glossary; "maxirata" is not used as a term.
8. **Explain**: the recurrence is stated once with its formula, plus the first period with the
   numbers substituted, the final adjustment and the totals; the rows carry the per-period
   numbers (R22).
9. **Rounding** applies to the instalment only; the default is no rounding; when chosen, the
   mode `half_up` or `half_even` is stated explicitly (R19). Clarification recorded on
   2026-11-08: "the instalment" means the amount the schedule type keeps constant, which is
   the principal portion in a fixed amortisation schedule, so that its principal portions stay
   equal before the final adjustment.

Amendment approved by the maintainer on 2026-11-08: AC9 and AC14 restated to the engine's
28-digit precision, after the independent numerical audit.

Further instructions recorded with the approval: error codes renumbered from `FM-E020`
(provisional, because SPEC-0006 takes `FM-E018` and `FM-E019`); at least one textbook or
regulatory worked example is required among the golden cases (AC15); Banca d'Italia texts are
checked on the official files.

### Questions as asked
- [x] 1. Amortisation type values: the proposal uses the AnaCredit names `french`,
  `fixed_amortisation_schedule` and `bullet` verbatim. "Fixed amortisation schedule" is the
  official European name but reads ambiguously in plain English. Keep it, or use a described
  name such as `constant_principal` (not a regulatory term)?
- [x] 2. Compounding default: when omitted, the compounding equals the payment frequency
  (periodic rate = annual rate / payments per year, as for an Italian TAN). Keep this
  documented default, or make the compounding required as in `rates.convert`?
- [x] 3. Should the loan methods also accept a periodic rate directly (as `tvm.annuity`
  does), as an alternative to the annual rate and frequency?
- [x] 4. Payments in advance in the schedule (R18): the proposed convention places the first
  instalment at t = 0 with no interest. Italian textbooks also describe "ammortamento a rate
  anticipate" with interest paid in advance at a discount rate. Is R18 the convention wanted,
  or should schedules support arrears only in version 1?
- [x] 5. Balloon amount: repaid with the last instalment (R17). Should `loans.number_of_instalments`
  also accept a balloon amount?
- [x] 6. Future value: no European regulatory source defines it; it is a CFA Institute
  curriculum term. Is `tvm.future_value` acceptable, and should a loan-specific
  `loans.outstanding_balance` (balance after k instalments) be added, or is the schedule enough?
- [x] 7. Italian terms for "bullet" and "balloon amount": no Banca d'Italia definition was found;
  the glossary keeps the English terms with an Italian explanation. "Maxirata" is common in
  consumer credit but was not verified on an official source. Confirm.
- [x] 8. Explain size: one step per period means up to 10000 steps for a monthly schedule.
  Accept, or use a compact step that references the rows with the recurrence stated once?
- [x] 9. Rounding scope: the proposal rounds the instalment (or principal portion) and the
  interest portions only. Should it also round the outstanding balance shown in each row (it is
  then exact by construction), and is `half_up` the right example default for Italian banks?
