# Plogic - Testing plan

What has to be verified, stage by stage, and which properties are worth
testing instead of examples. The plan is written against the design documents:
every case below comes from a promise made in one of them.

Levels 1–5 test the pure core and live next to the code (`#[cfg(test)]`
modules). Level 6 tests the CLI as a process (integration tests, one binary
invocation per case). Level 7 covers the product examples end to end.

## 1. Scanner

- One case per spelling of `TokenType.map`, including `¬`, `∧`, `∨`, `→`, `↔`
  and both spellings of every literal.
- Multi-character lexemes: `->` and `<->` with and without spaces, and a lone
  `-` or `<` (both are errors).
- Positions: `line` / `col` on the first line, after a newline, and after a
  multi-byte symbol (a formal symbol counts as one column).
- Line breaks: LF, CRLF counted as one, blank lines producing a bare
  `Newline`, and `Eof` appended at the end.
- Error recovery: `p % q` records one error *and* returns the tokens of
  `p q`; a run of digits like `12` is one `literal inválido '12'` error.
- Non-ASCII outside a literal or word (`ñ`, `ä`, `_`) is `carácter inválido`.

## 2. Parser

- Precedence: one case per adjacent level pair (`|` over `&`, `&` over `!`,
  `->` over `|`, `<->` over `->`).
- Associativity: `p -> q -> r` is `p -> (q -> r)`; `p <-> q <-> r` is
  `(p <-> q) <-> r`; `p & q & r` and `p | q | r` are left-nested.
- The product example `p | q & s -> t <-> q | 0` parses as
  `((p | (q & s)) -> t) <-> (q | 0)`.
- Grouping: parens override precedence; a redundant `Grouping` node is kept in
  the AST and dropped by `InfixPrinter` when unnecessary.
- Script loop: several propositions in one source, blank lines skipped, a
  final proposition without a trailing newline, an empty source.
- Recovery: a bad line between two good ones yields two parse errors and the
  stage `Err` (whole-run gate), not a partial list.
- Error positions: each example in `error_handling.md` reproduced exactly.

## 3. Simplifier

- One case per law in the table, in the direction the document fixes.
- Conditional and biconditional elimination: the output of `simplify` contains
  no `->` or `<->`, whatever the input.
- Canonical order: total (two different expressions always compare), and
  structural — reversing operand order or nesting differently gives the same
  output, e.g. `q & (p & r)` and `r & (q & p)` reduce to the same tree.
- Idempotence: `simplify(simplify(e)) == simplify(e)`.
- Termination on the shapes that could oscillate: factoring then expansion,
  `p & (q | r)`, nested De Morgan, double negation.
- Expansion phase accepts only strict improvements — the rejected candidate of
  `p & (q | r)` must not be returned.

Property tests over generated expressions:

- Equivalence: the simplified expression has the same truth column as the
  original (evaluated with the same variable order).
- Weight: each applied rule lowers `W`; no simplified formula is heavier than
  its input.

## 4. Interpreter and TruthTable

- Variable collection: order of first appearance (`q & p` gives columns
  `q, p`), duplicates counted once.
- Row order: binary counting, last variable varying fastest; `p -> q` matches
  the table in the product description, cells written `V` / `F`.
- Smallest and largest tables: a proposition with no variables (one row, no
  variable columns) and one with 12 variables (4096 rows).
- Size limit: 13 variables returns
  `Err(EvalError("resultado muy largo: más de 4096 filas"))` and enumerates
  nothing.
- Classification: a tautology, a contradiction and a contingency (for example
  `p | !p`, `p & !p`, `p`).

## 5. Orchestrator

- Stage gates: a scan error returns no parse errors; a parse error returns no
  tables; an eval error returns no tables.
- Mixed error kinds never appear in one `Err` list.
- `simplify = false` skips the Simplifier: the table still has no
  `Simplificada:` text, and the results match `simplify = true`.
- Batch order: tables come back in input order, one per proposition.
- `report`: `línea {line}, columna {col}: {message}` for positioned errors and
  the bare message for `EvalError`.

## 6. CLI

- Modes: bare argument, `-p`, `-a`, `-i`; usage errors (no argument, two mode
  flags, unknown flag, `-s` with `-i`) print usage and exit 2.
- Exit codes: 0 success, 1 evaluation failure or a file with no propositions,
  2 usage.
- Empty and blank-only files: `el archivo no contiene proposiciones`, exit 1.
- Output: tables on stdout, errors on stderr; `-s` writes the file and
  leaves stdout empty; a failing run with `-s` writes nothing and leaves an
  existing file untouched.
- Automatic redirect: a batch with a table above 1024 rows writes
  `plogic-output.txt` and notifies; an interactive session never redirects.
- Interactive: blank line shows a prompt again, `exit`, `salir` and Ctrl-D all
  end the session, an error does not end it.

## 7. Product examples

- `((p o (q y s)) entonces t) sisolosi (q o 0)` runs end to end and its label
  reads `p ∨ q ∧ s → t ↔ q ∨ F`.
- `(p o q) y r` is labelled `(p ∨ q) ∧ r`.
- Round trip: for any source, parsing the printed label gives an equivalent
  formula — `parse(print(parse(source)))` has the same truth column. This is
  the property that catches a missing pair of parentheses in `InfixPrinter`.

## What is not automated

- Column alignment of the printed table (checked by eye on a few tables,
  including a 12-variable one).
- Terminal rendering of `¬`, `∧`, `∨`, `→`, `↔` on the target consoles; a
  non-UTF-8 terminal is a known limitation, not a test target.
