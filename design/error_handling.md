# Plogic - Error handling design

Error handling is designed in from the start, not bolted on: user mistakes
are normal input, not crashes.

## Principles

- Errors are values, never exceptions or panics.
- Every fallible operation returns `Result<T>` (`TokenType.map`,
  `Scanner.scan`, `Parser.parse`, every parser rule, and — for its single
  failure case — `Interpreter.interpret`).
- Failure is signalled by `Err`, never by an empty table or a flag.
- Only the UI/CLI is impure.

## The error type

```
enum Error {
  ScanError(line: int, col: int, message: str)
  ParseError(line: int, col: int, message: str)
  EvalError(message: str)
}
```

A payload enum in the same style as `TokenType`. Positions are 1-based and
point at the offending character (scan) or token (parse); the message is
lower-case and says what was expected. `EvalError` is about size, not
position, so it carries no `line`/`col`.

## Where errors come from

| Stage | Example messages |
| --- | --- |
| Scanner | `carácter inválido '%'`, `literal inválido '12'`, `se esperaba '>' después de '-'`, `se esperaba '->' después de '<'` |
| Parser | `se esperaba una expresión`, `se esperaba ')' después de la expresión`, `se esperaba el fin de la proposición` |
| Interpreter | `resultado muy largo: más de 4096 filas` |

`Simplifier.simplify` and `TruthTable` cannot fail at all. The only error
after parsing is the Interpreter's size limit: a truth table of more than
4096 rows is refused before any row is enumerated (an `EvalError`, see
interpreter.md). I/O problems (an unreadable file, for instance) belong to
the CLI and are handled outside this model.

## Recovery inside a stage, gates between stages

Inside a stage, recovery is maximal — a stage reports as many errors as it
can in one pass:

- The scanner records a `ScanError`, skips the offending character or run,
  and keeps scanning.
- The parser records a `ParseError`, `synchronize`s to the next line and
  keeps parsing.

Between stages, the pipeline stops at the first failing stage. A stage that
recorded any error returns `Err(errors)` and discards its partial value
(the book's `hadError` rule); the next stage never runs:

- scanning failed → `Parser.parse` does not run;
- parsing failed → nothing is simplified or evaluated;
- evaluation failed (the size limit) → the run stops there.

Consequences:

- When a run fails, no truth tables are produced at all — but every error
  of the failing stage is reported.
- An `Err` list holds errors of exactly one stage: `ScanError`,
  `ParseError` and `EvalError` never mix. (This supersedes the earlier idea
  that parsing would run after scan errors.)

## Reporting

`Orchestrator.report(error: Error): str` is a pure formatter:

```
línea {line}, columna {col}: {message}
```

matching the product description's example `línea 2, columna 7: se esperaba ')'
después de la expresión`. An `EvalError` has no position and prints as its
message alone. The CLI prints one formatted error per line on stderr
and exits non-zero (see ui.md).

## Examples

| Source | Error |
| --- | --- |
| `p % q` | `línea 1, columna 3: carácter inválido '%'` |
| `12` | `línea 1, columna 1: literal inválido '12'` |
| `p - q` | `línea 1, columna 3: se esperaba '>' después de '-'` |
| `p < q` | `línea 1, columna 3: se esperaba '->' después de '<'` |
| `p &` | `línea 1, columna 4: se esperaba una expresión` |
| `(p \| q` | `línea 1, columna 7: se esperaba ')' después de la expresión` |
| `p q` | `línea 1, columna 3: se esperaba el fin de la proposición` |
