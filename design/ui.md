# Plogic - UI / CLI design

Role: the only impure component. It receives propositions, drives the
Orchestrator and presents results. `CLI` implements the `UI` interface,
which is unchanged: `receive_file(path)`, `receive_proposition(proposition)`,
`start_interactive()`, `present(table)`.

It presents all its content in Spanish.

## Modes — explicit flags

The CLI has the following flags. With no mode flag, one bare argument is
evaluated as a proposition (single proposition mode):

| Flag | Mode | Maps to |
| --- | --- | --- |
| `none` | single proposition (default) | `receive_proposition` |
| `-p`, `--proposicion <proposition>` | single proposition | `receive_proposition` |
| `-a`, `--archivo <path>` | one proposition per line | `receive_file` |
| `-i`, `--interactivo` | interactive session | `start_interactive` |

Modifier flags (they combine with `-p`, `-a`, the bare argument and, except
for `-s`, with `-i`):

- `--no-simplificar` disables the Simplifier; by default propositions are
  simplified.
- `--tabla-simplificada` builds the truth table from the simplified
  expression instead of the original. It is long-only, and it cannot be
  combined with `--no-simplificar`.
- `-s <path>` (long form `--salida <path>`) writes the output to the file
  in `<path>` rather than to the console.

By default the table is built from the original proposition, with the
simplified formula shown only as a reference line. With
`--tabla-simplificada` the table is built from the simplified expression:
its variables, rows, size check and result-column label all come from the
simplified form, and the `Simplificada:` line repeats that label. The
classification is shown in both cases.

The flag names are Spanish. With no mode and no proposition (a bare
invocation or a modifier-only invocation), more than one mode flag, an unknown
flag, a missing flag value, an extra positional argument, or a single
proposition that is empty after trimming, the CLI prints a usage message and
exits with code 2.
`--tabla-simplificada` together with `--no-simplificar`, and `--salida`
together with `--interactivo`, are also usage errors: a session produces many
tables and there is no single file for them.

An argument that begins with `-` is always an option, except `--`. `--` ends option
parsing and is never an option value: a following argument is the bare
proposition even if it begins with `-`, and at most one positional argument is
allowed after it. A trailing `--` is harmless when a proposition already
exists (`plogic -p p --`), while `plogic -p -- p` reports
`falta el valor de '-p'`. So `plogic ->` is `opción desconocida: '->'`, while
`plogic -- '->'` scans `->`. A single proposition, bare or from `-p`, is
trimmed; if nothing remains, the CLI prints `no se proporcionó ninguna
proposición` and exits 2. Only the emptiness check uses the trimmed value; the
scanner receives the original text.

## File mode

`receive_file(path)` is where file mode starts: the CLI reads the file (it
is the impure component) and passes the text to `Orchestrator.evaluate`, so
the Orchestrator stays pure. A file with no propositions — empty, or only
blank lines — is not evaluated: the CLI prints `el archivo no contiene
proposiciones` on stderr and exits with code 1.

## Output destination

Truth tables go to stdout and errors to stderr. The output is written to a
file instead of the console in two cases (the UI does the writing — the
Orchestrator stays pure):

- `-s` / `--salida` writes the output to the given path; the explicit
  flag wins over the automatic file name. The file is written only when
  there is output: a failing run writes nothing and leaves an existing file
  at `<path>` untouched. A write failure prints
  `no se pudo escribir el archivo '<path>': <detalle>` on stderr and exits
  1.
- When one of a run's truth tables has more than 2^10 rows, the output goes
  to `plogic-output.txt` and the user is notified on stderr with
  `la salida se escribió en plogic-output.txt`. This applies to every
  non-interactive run, single proposition or file. (A table of more than
  4096 rows never reaches presentation: the Interpreter refuses it with an
  `EvalError`, see interpreter.md.) In an interactive session every table
  is printed to the console and no redirect happens.

## Presentation

`present(table: TruthTable)` prints, in this order:

1. when simplification ran: the simplified formula in canonical form, e.g.
   `Simplificada: p ∧ ¬q`. With `--tabla-simplificada` this line repeats
   the table's label.
2. the truth table — the variable columns and the result column, aligned,
   with `V` and `F` as values, following the layout in truth_table.md; the
   result column is labelled with the evaluated proposition in normalized
   form. By default that is the original proposition, keeping the order and
   grouping the user wrote (redundant parentheses removed); with
   `--tabla-simplificada` it is the simplified formula in canonical order.
3. always: a classification line, e.g.
   `Esta proposición es una tautología.`

When the user passed `--no-simplificar`, the simplified formula is not
printed (item 1); the table and the classification are still printed.
A blank line separates the printed items: after the simplified line, after
the table, and before the classification. With `--no-simplificar` only the
last separator remains, between the table and the classification.
Formulas are shown in infix notation by `InfixPrinter` — a static printer
alongside `ASTPrinter` — and always in the one normalized form: `¬`, `∧`,
`∨`, `→`, `↔`, `V` / `F`, parentheses only where precedence or
associativity requires (see truth_table.md).

## Interactive mode

`start_interactive()` runs a read-eval-print session:

- The prompt `> ` accepts one proposition per line; each line is evaluated
  on its own (`evaluate` over that single line) and presented immediately.
- `--no-simplificar` and `--tabla-simplificada` each apply to every line;
  using them together is a usage error, as in the other modes.
- An error on one line prints its formatted errors and the session
  continues — a bad proposition does not end the session.
- A blank or whitespace-only line evaluates nothing and just shows a new
  prompt; an empty session is not an error.
- `exit`, `salir`, or end-of-input (Ctrl-D) ends the session.
- Interactive output always goes to the console; the automatic
  `plogic-output.txt` redirect never applies.

## Errors and exit codes

- Formatted errors (via `Orchestrator.report`) go to stderr, one per line;
  truth tables go to stdout.
- Usage errors print the usage line
  `uso: plogic [<proposición> | -p <proposición> | -a <archivo> | -i] [--no-simplificar | --tabla-simplificada] [-s <archivo>]`
  followed by the specific message, and exit 2. The specific messages are
  `falta el valor de '<flag>'`, `opción desconocida: '<flag>'`, `solo se
  puede usar un modo a la vez`, `argumento inesperado: '<arg>'`,
  `'-s' no se puede usar con '--interactivo'`,
  `'--tabla-simplificada' no se puede usar con '--no-simplificar'` and
  `no se proporcionó ninguna proposición`. When several usage errors apply,
  the CLI reports one message, the first in this order: unknown flag, missing
  flag value, more than one mode, extra positional, no proposition,
  `--tabla-simplificada` with `--no-simplificar`, `-s` with `--interactivo`
  (ADR-027).
- Exit codes: 0 on success, 1 on any failing run (scan, parse, evaluation, a
  file with no propositions, or an I/O failure), 2 for usage errors.
- I/O problems are reported by the CLI directly; they are not part of the
  `Error` model. A read failure prints
  `no se pudo leer el archivo '<path>': <detalle>` and exits 1; an empty
  file prints `el archivo no contiene proposiciones` and exits 1.
