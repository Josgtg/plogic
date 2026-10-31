# Plogic - Orchestrator design

Role: the pipeline driver — the book's `Lox.run` pattern. It owns the
sequence source → tokens → expressions → simplified expressions → truth
tables, and it formats errors for display. The Orchestrator is stateless:
its entry points are pure static methods.

## API

- `Orchestrator.evaluate(source: str, simplify: bool):
  Result<List<TruthTable>>` — runs the whole pipeline over the source text
  of one or more propositions and returns one truth table per proposition,
  in input order. `simplify` reflects the user's choice (the CLI's
  `--no-simplificar` flag); when false the Simplifier is skipped.
- `Orchestrator.report(error: Error): str` — formats one error (see
  error_handling.md).

Failure is signalled by `Err`: the failing stage's error list. `Ok` always
carries one table per proposition. (The class diagram still shows
`evaluate(source: str)`; the signature with `simplify` is pending there.)

## Flow

Matching the components diagram's numbered steps:

1. The UI hands over the source text.
2. `Scanner.scan(source)` tokenizes it.
3. Stage gate: on `Err(scan errors)`, `evaluate` returns that list; the
   parser does not run.
4. `Parser.parse(tokens)` builds one expression per proposition.
5. Stage gate: on `Err(parse errors)`, `evaluate` returns that list;
   nothing is simplified or evaluated.
6. Per proposition: `Simplifier.simplify(expression)` when `simplify` is
   on, otherwise the expression is used as-is.
7. `Interpreter.interpret(reduced)` builds the `TruthTable`.
8. Stage gate: on `Err(eval error)` — the size limit of interpreter.md —
   `evaluate` returns it and no truth tables are produced at all.
9. The Orchestrator collects the tables and returns `Ok(tables)`; the UI
   presents them.

Steps 6–7 repeat once per proposition; step 8 aborts the run on the first
`EvalError`. The Orchestrator also fills each table's display text: the
evaluated formula in normalized form with the operand order the user wrote,
and — when the Simplifier ran — the simplified formula in canonical order
(`InfixPrinter`); the classification is derived from the table itself (see
truth_table.md).

## Purity

`evaluate` and `report` are pure, and so are the stages they call. Only
the UI/CLI performs I/O.

## Limits

Two limits concern long outputs. When one of a run's truth tables has more
than 2^10 rows, the UI writes the run's output to `plogic-output.txt`
instead of the console and notifies the user (see ui.md — the UI does the
writing; the Orchestrator is pure). A truth table of more than 4096 rows is
refused before it is built, with an `EvalError` (see interpreter.md).
