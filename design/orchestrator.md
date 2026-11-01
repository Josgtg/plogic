# Plogic - Orchestrator design

Role: the pipeline driver — the book's `Lox.run` pattern. It owns the
sequence source → tokens → expressions → simplified expressions → truth
tables, and it formats errors for display. The Orchestrator is stateless:
its entry points are pure static methods.

## API

- `Orchestrator.evaluate(source: str, simplify: bool):
  Result<List<TruthTable>, List<Error>>` — runs the whole pipeline over the
  source text of one or more propositions and returns one truth table per
  proposition, in input order. `simplify` reflects the user's choice (the
  CLI's `--no-simplificar` flag); when false the Simplifier is skipped and
  the original is tabulated. When true the table is built from the
  simplified expression, whose label and canonical text are also the table's
  display text.
- `Orchestrator.simplify(source: str): Result<List<str>, List<Error>>` — runs
  scan, parse and simplify only, returning each proposition's normalized
  infix text in input order. It builds no tables, so the size limit does not
  apply (`--simplificar`).
- `Orchestrator.report(error: Error): str` — formats one error (see
  error_handling.md).

All three entry points take their owned argument by value: `evaluate` and
`simplify` consume the source text, which the scanner stores anyway
(ADR-038), and `report` consumes the error, which the UI has already moved
out of a list (ADR-044).

Failure is signalled by `Err`: the failing stage's error list. `Ok` always
carries one table per proposition (`evaluate`) or one formula per
proposition (`simplify`).

## Flow

The pipeline, in order (the components diagram's ten arrows):

1. The UI hands over the source text.
2. `Scanner.scan(source)` tokenizes it.
3. Stage gate: on `Err(scan errors)`, the entry point returns that list; the
   parser does not run.
4. `Parser.parse(tokens)` builds one expression per proposition.
5. Stage gate: on `Err(parse errors)`, the entry point returns that list;
   nothing is simplified or evaluated. `simplify` stops here.
6. Per proposition: `Simplifier.simplify(expression)` when `simplify` is
   on; otherwise this step is skipped.
7. The simplified expression, kept alongside the original when step 6 ran.
8. Choose the expression to tabulate: the simplified expression when
   `simplify` is true, the original otherwise; call
   `Interpreter.interpret(chosen)`.
9. Stage gate: on `Err(eval error)` — the size limit of interpreter.md —
   `evaluate` returns it and no truth tables are produced at all. Otherwise
   the step produces the `TruthTable`.
10. The Orchestrator collects the tables and returns `Ok(tables)`; the UI
    presents them.

Steps 6–9 repeat once per proposition; steps 1–5 and 10 happen once per run.
Step 9 aborts the run on the first `EvalError`. The Orchestrator also adds
each table's display text with `TruthTable.with_display`: the chosen
expression in normalized form — canonical order when it is the simplified
formula, the order the user wrote when `--no-simplificar` keeps the
original — and, when the Simplifier ran, the simplified formula in canonical
order (`InfixPrinter`); the classification is derived from the table itself
(see truth_table.md).

## Purity

`evaluate`, `simplify` and `report` are pure, and so are the stages they
call. Only the UI/CLI performs I/O.

## Limits

Two limits concern long outputs. When one of a run's truth tables has more
than 2^10 rows, the UI writes the run's output to `plogic-output.txt`
instead of the console and notifies the user (see ui.md — the UI does the
writing; the Orchestrator is pure). This applies to every non-interactive
run, single proposition or file; `-o` wins over the automatic name, and
interactive sessions never redirect. A truth table of more than 4096 rows
is refused before it is built, with an `EvalError` (see interpreter.md);
the check uses the expression chosen for the table, so the default checks
the simplified one and `--no-simplificar` checks the original.
`Orchestrator.simplify` builds no table, so neither limit applies to
`--simplificar`.
