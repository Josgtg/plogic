# Plogic - Testing plan

What has to be verified, stage by stage, and which properties are worth
testing instead of examples. The plan is written against the design documents:
every case below comes from a promise made in one of them.

Levels 1–5 test the pure core and live next to the code (`#[cfg(test)]`
modules). Level 6 tests the CLI as a process (integration tests, one binary
invocation per case). Level 7 covers the product examples end to end. Level 8
covers the GUI's state and pure helpers; the rendered window is checked by eye.

## 1. Tokens and scanner

- Tokens, spellings: one case per spelling of `TokenType.map`, including `¬`,
  `∧`, `∨`, `→`, `↔` and both spellings of every literal; `map` is
  case-sensitive (`V` is a literal, `v` is not) and never produces an
  identifier, a `Newline` or an `Eof`.
- Tokens, errors: the message is chosen by the lexeme's shape — `12` gives
  `literal inválido '12'`, `%` gives `carácter inválido '%'` (ADR-036).
- Tokens, display: every kind renders its normalized spelling (`¬`, `∧`, `∨`,
  `→`, `↔`, `V`, `F`, `(`, `)`), an identifier renders its own name, and
  `Newline` / `Eof` render as the empty string; `Token::to_string()` equals its
  kind's.
- Multi-character lexemes: `->` and `<->` with and without spaces, and a lone
  `-` or `<` (both are errors).
- Positions: `line` / `col` on the first line, after a newline, after a
  multi-byte symbol (a formal symbol counts as one column), and after a tab
  (also one column); a `Newline` token carries the line it ends.
- Line breaks: LF, CRLF counted as one, blank lines producing a bare
  `Newline`, and `Eof` appended just past the last character, with no final
  `Newline` when the source does not end with one. A lone CR (not followed by
  LF) is not a line break but a `carácter inválido` error (ADR-038).
- Identifiers are case-sensitive: `p` and `P` are different variables, and
  lowercase `v` / `f` are identifiers, not literals. `V` / `F` are literals.
  An underscore continues an identifier (`p_1`, `a_b`) but never starts one,
  so `_p` and bare `_` are `carácter inválido '_'` errors (ADR-039).
- Error recovery: `p % q % r` records two errors — at columns 3 and 7 — and keeps
  scanning internally; `scan` still returns the whole stage `Err` and discards
  the partial token list. A run of digits like `12` is one `literal inválido '12'`
  error.
- Non-ASCII outside a literal or word (`ñ`, `ä`, `_`) is `carácter inválido`.
- Errors report the line they happen on: a bad character after a line break
  is `línea 2, columna 1: …`.

## 2. Parser

- Precedence: one case per adjacent level pair (`|` over `&`, `&` over `!`,
  `->` over `|`, `<->` over `->`).
- Associativity: `p -> q -> r` is `p -> (q -> r)`; `p <-> q <-> r` is
  `(p <-> q) <-> r`; `p & q & r` and `p | q | r` are left-nested.
- The product example `p | q & s -> t <-> q | 0` parses as
  `((p | (q & s)) -> t) <-> (q | 0)`.
- Grouping: parens override precedence; a redundant `Grouping` node is kept in
  the AST and dropped by `InfixPrinter` when unnecessary.
- AST equality is structural and ignores the operator token's position: trees
  with the same shape and operator kinds are equal even when they were written
  at different lines and columns, and a `Grouping` is never equal to its bare
  child (ADR-040).
- Printers: `!p & q` gives `ASTPrinter` output `(∧ (¬ p) q)` and
  `InfixPrinter` output `¬p ∧ q`; `(p | q) & r` keeps its parentheses;
  `(p)` prints `p` in infix but `(group p)` in the prefix dump, because
  `ASTPrinter` renders every `Grouping` as `(group …)` (ADR-040).
  Associativity-aware parentheses: `(p -> q) -> r` prints `(p → q) → r`,
  while `p -> (q -> r)` prints `p → q → r` and `p ↔ (q ↔ r)` keeps its
  parentheses; a right-nested `∧` or `∨` loses them, so `p & (q & r)` prints
  `p ∧ q ∧ r` (ADR-040).
- Script loop: several propositions in one source, blank lines skipped, a
  final proposition without a trailing newline, an empty source.
- Recovery: bad lines between good ones report each bad line's error and the
  stage `Err` (whole-run gate), not a partial list. For `p\np &\nq &\nr`, the
  errors are `línea 2, columna 4: se esperaba una expresión` and
  `línea 3, columna 4: se esperaba una expresión`; the stage returns `Err`
  with both and discards the partial expressions. The second error proves
  recovery resumed after the first bad line, from the state carried by the
  failing rule at the failing token.
- Error positions: each example in `error_handling.md` reproduced exactly.

## 3. Simplifier

- Grouping is erased first: `((p))` reduces to `p`, `q & (p & r)` reduces to
  `p & q & r`, and the output contains no `Grouping` node.
- One case per law in the table, in the direction the document fixes.
- Conditional and biconditional elimination: the output of `simplify` contains
  no `->` or `<->`, whatever the input.
- Canonical order: total (two different expressions always compare), and
  structural — reversing operand order or nesting differently gives the same
  output, e.g. `q & (p & r)` and `r & (q & p)` reduce to the same tree.
  Variable order is ASCII code-point, case-sensitive, prefix first: `P` before
  `p`, and `p` before `p1`.
- Canonical examples: `p -> q` gives `q | !p`; `p <-> q` gives
  `(p | !q) & (q | !p)`; `!(p | q)` gives `!p & !q`.
- Idempotence: `simplify(simplify(e)) == simplify(e)`.
- Determinism: the rules apply in the law table's order and the expansion
  visits the tree pre-order, so the same input gives the same output
  (ADR-042); a chain rule matches any pair of operands of a longer chain.
- Process list: `simplify` returns the final expression and the applied steps.
  `!(!p | !q)` gives `De Morgan` (`!p`/`!q` rewritten), then two
  `Doble negación` steps; `p -> q` gives one `Condicional` step; a formula that
  needs no law (`p`) has no steps. Each step's formula is canonical, a
  discarded expansion leaves no step, and the steps' final formula equals the
  returned expression (ADR-047).
- Termination on the shapes that could oscillate: factoring then expansion,
  `p & (q | r)`, nested De Morgan, double negation.
- Expansion phase accepts only strict improvements — the rejected candidate of
  `p & (q | r)` must not be returned.

Property tests over generated expressions:

- Equivalence: the simplified expression has the same truth column as the
  original (evaluated with the same variable order).
- Weight: each applied rule lowers `W`; no simplified formula is heavier than
  its input. `W` is not the connective count: a law may add connectives while
  lowering weight (ADR-023).

## 4. Interpreter and TruthTable

- Variable collection: order of first appearance (`q & p` gives columns
  `q, p`), duplicates counted once.
- Row order: binary counting, last variable varying fastest; `p -> q` matches
  the table in the product description, cells written `V` / `F`.
- Smallest and largest tables: a proposition with no variables (one row, no
  variable columns) and one with 12 variables (4096 rows).
- Size limit: 13 variables returns
  `Err([EvalError("resultado muy largo: más de 4096 filas")])` and enumerates
  nothing. A 64-variable expression returns the same error without an
  overflow (the check is `n > 12`, not an unchecked `2^n`).
- Classification: a tautology, a contradiction and a contingency (for example
  `p | !p`, `p & !p`, `p`). An empty hand-built table, which `interpret`
  never produces, is a contingency (ADR-043).
- `TruthTable.with_display` returns a new table with the label and optional
  simplified text; the table returned by the Interpreter keeps them empty and
  is not modified. A base table without simplified text is valid.

## 5. Orchestrator

- Default table: the simplified expression decides variables, rows, size check
  and label. `p -> q` gives columns `q, p`, label `q ∨ ¬p`, simplified text
  `q ∨ ¬p`, and an `Evaluation` whose `simplification` is `Some` with one
  `Condicional` step.
- `simplify = false`: the original expression decides everything. `p -> q`
  gives columns `p, q`, label `p → q`, no simplified text, and
  `simplification == None`; its rows still have the same results as the
  default table.
- `p | !p` default: no variable columns, one row, label `V`, simplified text
  `V`, classification tautology. With `simplify = false`: columns `p`, rows `F`
  and `V`, result all `V`, label `p ∨ ¬p`, no simplified text.
- A 13-variable original that simplifies to `V` (for example
  `p1 | !p1 | p2 | ... | p13`): the default succeeds with one row; with
  `simplify = false` it returns
  `Err([EvalError("resultado muy largo: más de 4096 filas")])`.
- `Orchestrator.simplify` (formula-only): `p -> q` returns one
  `Simplification` whose expression prints `q ∨ ¬p` and whose steps name
  `Condicional`; the 13-variable tautology returns `V` without the size check;
  batch order is input order; an empty source returns `Ok([])`; scan and parse
  errors gate it exactly like `evaluate`.
- `Orchestrator.assess` with `max_variables = None`: `p -> q` returns one
  `Assessment` with formula `q ∨ ¬p`, `variables == 2`, one `13a. Condicional`
  step, and `table == None`; with `simplify = false` the formula is `p → q`,
  `simplification == None` and the table is still `None`.
- `Orchestrator.assess` with `Some(10)`: a 2-variable proposition builds a
  table (label `q ∨ ¬p`); a 9-variable proposition builds one; a 10-variable
  proposition has `table == None`, `variables == 10`, and still returns its
  formula and steps; a 13-variable tautology with `simplify = false` is not
  refused because no table is built (with `simplify = true` it collapses to
  `V` (zero variables, one row), and is tabulated). Batch order, empty `Ok([])` and
  scan/parse gates match `evaluate`; the table is only built when
  `variables < limit`.
- Stage gates: a scan error returns no parse errors; a parse error returns no
  tables; an eval error returns no tables.
- Mixed error kinds never appear in one `Err` list.
- Batch order: tables come back in input order, one per proposition. An
  empty or whitespace-only source has no propositions, so it returns
  `Ok([])`.
- `report`: `línea {line}, columna {col}: {message}` for positioned errors and
  the bare message for `EvalError`.

## 6. CLI

- Modes: bare argument, `-p`, `-a`, `-i`.
- Help and version: `-h` and `--help` print the exact help screen from
  `ui.md` on stdout and exit 0; `-V` and `--version` print `plogic 0.1.0` and
  exit 0. The parser prints them when it reaches them (ADR-033): an earlier
  error wins, so `-x --help` and `-p --help` are usage errors, while
  `--help -x`, `-hV` and `--help --version` print the help, and `-V --help`
  prints the version. `-- --help` is a proposition.
- Strict usage errors (message, blank line and the full help screen on stderr,
  exit 2; ADR-034): modifier-only invocation, two mode flags, unknown flag,
  missing flag value,
  extra positional argument, empty `-p ''` / bare empty string, a single
  proposition that is whitespace only, `-o` with `-i`,
  `-s` with `--no-simplificar`, and `-m` with `--no-simplificar`.
- Dash-prefixed arguments: `-x` and `->` are `opción desconocida`; `--`
  followed by `->` passes it through as a proposition, which then fails at
  parse time with `se esperaba una expresión`; `plogic -p p --` is valid;
  `plogic --` is a missing-proposition usage error; `plogic -p -- p` is
  `falta el valor de '-p'`; two positionals after `--` are an
  extra-positional usage error. A proposition with surrounding spaces
  (`-p '  p  '`) still scans normally.
- Repeated flags (ADR-031): `-p p -p q`, `-i -i` and `-a f -a g` report
  `solo se puede usar un modo a la vez`; `-s -s` reports
  `argumento inesperado: '-s'`, `-m -m` reports
  `argumento inesperado: '-m'`, `--no-simplificar --no-simplificar` reports
  `argumento inesperado: '--no-simplificar'`, and `-o a -o b` reports
  `argumento inesperado: '-o'`; all exit 2.
- Usage-error precedence: `-x -i -o out.txt` reports
  `opción desconocida: '-x'` (unknown flag before the `-o` conflict);
  `-i -o out.txt -s --no-simplificar` reports
  `'--simplificar' no se puede usar con '--no-simplificar'` before
  `'-o' no se puede usar con '--interactivo'`, and prints exactly one message;
  `-m --no-simplificar` reports
  `'--mostrar-proceso' no se puede usar con '--no-simplificar'`.
  A `-p` missing its value wins over the extra positional that follows.
- Exact strings: a table-driven test asserts the layout of a usage error — the
  specific message, a blank line, then the exact help screen from `ui.md` — for
  `falta el valor de '<flag>'`, `opción desconocida: '<flag>'`,
  `solo se puede usar un modo a la vez`, `argumento inesperado: '<arg>'`,
  `no se proporcionó ninguna proposición`, and the three conflict messages
  (`'--simplificar' no se puede usar con '--no-simplificar'`,
  `'--mostrar-proceso' no se puede usar con '--no-simplificar'` and
  `'-o' no se puede usar con '--interactivo'`), with exit 2; the read/write
  messages with exit 1; and the redirect notification
  `la salida se escribió en plogic-output.txt` on stderr.
- Exit codes: 0 on success or help/version, 1 on any failing run (scan, parse,
  evaluation, an empty file, or an I/O failure), 2 usage.
- Empty and blank-only files: `el archivo no contiene proposiciones`, exit 1.
- Output: tables on stdout, errors on stderr; `-o` writes the file and
  leaves stdout empty; a failing run with `-o` writes nothing and leaves an
  existing file untouched.
- `-o` wins over the automatic `plogic-output.txt` name: an 11-variable
  proposition above 1024 rows with `-o path` writes `path` and not
  `plogic-output.txt`.
- Automatic redirect: a non-interactive run (single proposition or file) with
  a table above 1024 rows writes `plogic-output.txt` and notifies; a table of
  exactly 1024 rows does not redirect; an interactive session never redirects.
  The threshold uses the table's expression:
  `(p1 | !p1) & p2 & ... & p11` does not redirect by default (the simplified
  table has 10 variables, 1024 rows) but does with `--no-simplificar` (11
  variables, 2048 rows); `(p1 | !p1) & p2 & ... & p13` succeeds by default
  (12 variables, 4096 rows) and fails with `--no-simplificar` (13 variables,
  8192 rows).
- Presentation: `present` separates the simplified line, the table and the
  classification with blank lines; with `--no-simplificar` only the blank line
  between the table and the classification remains (ADR-029).
- `-m` / `--mostrar-proceso`: after `Simplificada:` and a blank line, a
  `Proceso:` block with one numbered `<law>: <formula>` line per applied step;
  omitted entirely when no law applied, and absent without `-m`. It combines
  with table mode and with `-s`, works in every mode, is written by `-o`, and
  conflicts with `--no-simplificar` (ADR-047). For `p -> q` the block is
  exactly `1. 13a. Condicional: q ∨ ¬p`; for `!(!p | !q)` it lists
  `10a. De Morgan`, then two `7a. Doble negación` steps.
- Table layout: cells are left-aligned to the widest text of their column
  (header included), columns are separated by two spaces, `| ` introduces the
  result column and the last column is unpadded, so no line has trailing
  spaces; a table with no variables has no `|` separator (ADR-045).
- File mode numbering: each block starts with `Proposición n:` and a blank
  line, blocks are separated by one blank line, the counter counts only built
  tables, and single-proposition mode has no heading (ADR-045).
- `-s` / `--simplificar` prints only `Simplificada: <formula>`, with no table
  and no classification, in every mode; it combines with `-o`, with `-m` and
  with the file-mode numbering, never triggers the automatic redirect, and
  conflicts
  with `--no-simplificar`.
- `--no-simplificar` omits only the `Simplificada:` line, never the
  classification.
- I/O failures: an unreadable input prints
  `no se pudo leer el archivo '<path>': <detalle>` and exits 1; an unwritable
  output prints `no se pudo escribir el archivo '<path>': <detalle>` and exits
  1.
- Interactive: a blank or whitespace-only line shows a prompt again, `exit`,
  `salir` and Ctrl-D all end the session, an error does not end it.
  `--no-simplificar` and `--simplificar` apply to every line; using them
  together is a usage error. A
  presentation or an error is followed by a blank line before the next prompt;
  Ctrl-D prints one newline after the pending prompt, while `exit`/`salir` add
  none (ADR-045).

## 7. Product examples

- `((p o (q y s)) entonces t) sisolosi (q o 0)` runs end to end and its label
  reads `p ∨ q ∧ s → t ↔ q ∨ F`.
- `(p o q) y r` is labelled `(p ∨ q) ∧ r`.
- The table label is the simplified formula, not the original.
- With `--no-simplificar`, the table label is the original proposition.
- Round trip: for any source, parsing the printed label gives an equivalent
  formula — `parse(print(parse(source)))` has the same truth column. This is
  the property that catches a missing pair of parentheses in `InfixPrinter`;
  the implication cases are `(p -> q) -> r` and `p -> (q -> r)`.

## 8. Graphical window

The window itself is not launched in automated tests. `main` dispatches on
`gui::should_launch`; `args::parse([])` keeps its `NoProposition` unit test
because the binary intercepts the empty list before calling it. The GUI state
and the pure helpers live in `src/gui.rs` and are tested there; the rendered
layout is checked by eye.

- Entry: `should_launch` is true only for an empty argument list; false for a
  bare proposition, `-p`, `-a`, `-i`, `-h`, `-V`, `-m`, `--` and a
  modifier-only invocation. `STARTUP_ERROR` is exactly
  `no se pudo iniciar la interfaz gráfica`; the headless path is a manual
  smoke test (run with no display, check stderr and exit 1).
- Default state: `simplify` true, `show_table` true, `show_process` false, no
  outcome, empty input.
- Evaluate `p -> q` with the defaults: `Outcome::Evaluations` stores a `View`
  with `show_table` true, `show_process` false and one `Assessment` whose
  formula is `q ∨ ¬p`, whose `variables` is 2, whose simplification has one
  `13a. Condicional` step and whose table is present with label `q ∨ ¬p` and
  a contingency classification.
- Snapshot options: after an evaluation, changing `simplify`, `show_table` or
  `show_process` leaves the stored `View` and assessments unchanged. Only
  `evaluate`/Enter replaces them. The same applies to editing the input,
  inserting a symbol and `Limpiar` (which clears only the input and cursor).
- Table off: evaluate with `show_table` false. The stored `View.show_table` is
  false and every assessment has `table == None`; no table or classification
  is produced. Turning `show_table` on afterwards leaves the stored result
  unchanged; the next `evaluate` builds the table.
- Table on, fewer than ten variables: `p -> q` has a table and a
  classification. A 9-variable proposition also has one.
- Table on, ten or more variables: a 10-variable proposition has
  `table == None`, `variables == 10`, and still has its formula and steps; the
  result shows the muted note with the variable count. A 13-variable
  tautology is not refused.
- Process snapshot: with `show_process` false the stored `View` is false and
  the process is absent; turning it on afterwards does not change the stored
  result. A later evaluate with it on stores true and shows the steps.
- Simplify snapshot: evaluating with `simplify` true stores the simplified
  formula and steps; changing the checkbox to false afterwards leaves that
  stored result unchanged; the next evaluate stores the original formula.
- Empty input: `evaluate` on an empty or whitespace-only field sets the error
  array to `no se proporcionó ninguna proposición`; it does not clear the
  field. A scan or parse error replaces the result with one red report line per
  error, and a later `evaluate` replaces it again.
- Helpers: `next_variable` on an empty field gives `p`, on `p` gives `q`, on
  `p & q` gives `r`, skips `y` (reserved for conjunction), and gives `p` when
  every usable letter is used or the formula does not scan. `byte_index` maps
  character indices to UTF-8 byte offsets; insertion replaces a selection and
  leaves the cursor after the inserted text. `value_cell` maps true to
  `V`/green and false to `F`/red; the classification mapping is
  tautology/green, contradiction/red, contingency/muted.
- Table data: one column per `table.variables`, one row per `table.rows`, a
  header of the variables plus `Resultado`, and `2^variables` rows. The table
  is a real grid, horizontally scrollable when needed; the result section is
  the only vertical scroll. The visual layout (centered cells, alternating
  rows, header above the body) is checked by eye.
- Layout and help: the header and input section are fixed, the result section
  scrolls and its contents are centered; the `Ayuda` list is collapsed by
  default and, when open, is one column set with the rows
  `¬ negación !, no`, `∧ conjunción &, y`, `∨ disyunción |, o`,
  `→ implicación ->, entonces`, `↔ bicondicional <->, sisolosi`,
  `V verdadero 1` and `F falso 0`, at 12.5 points (body 15). Checked by eye.
- CLI: the integration suite never runs the binary with no arguments (that
  would open a window); `-p ''` and modifier-only invocations still cover the
  `NoProposition` usage error.

## What is not automated

- Column alignment of the printed table (checked by eye on a few tables,
  including a 12-variable one).
- Terminal rendering of `¬`, `∧`, `∨`, `→`, `↔` on the target consoles; a
  non-UTF-8 terminal is a known limitation, not a test target. Color is a
  terminal-only presentation detail: `V` green and `F` red in table cells, the
  message of an error bright red on stderr (ADR-035), plain text in files.
