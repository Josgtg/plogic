# Plogic - UI / CLI design

Role: the only impure component. It receives propositions, drives the
Orchestrator and presents results. `CLI` implements the `UI` interface,
which is unchanged: `receive_file(path)`, `receive_proposition(proposition)`,
`start_interactive()`, `present(table)`.

It presents all its content in Spanish.

## Modes — explicit flags

The CLI has the following flags. With no flag at all, one bare argument is
evaluated as a proposition (single proposition mode):

| Flag | Mode | Maps to |
| --- | --- | --- |
| `none` | single proposition (default) | `receive_proposition` |
| `-p`, `--proposicion <proposition>` | single proposition | `receive_proposition` |
| `-a`, `--archivo <path>` | one proposition per line | `receive_file` |
| `-i`, `--interactivo` | interactive session | `start_interactive` |

Modifier flags (they combine with `-p`, `-a` and the bare argument):

- `--no-simplificar` disables the Simplifier; by default propositions are
  simplified.
- `-s <path>` (long form `--salida <path>`) writes the output to the file
  in `<path>` rather than to the console.

The flag names are Spanish. With no argument at all, more than one mode
flag, or an unknown flag, the CLI prints a usage message and exits with
code 2. `--salida` together with `--interactivo` is also a usage error: a session
produces many tables and there is no single file for them.

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
  at `<path>` untouched.
- When one of a run's truth tables has more than 2^10 rows, the run's
  output goes to `plogic-output.txt` and the user is notified. (A table of
  more than 4096 rows never reaches presentation: the Interpreter refuses
  it with an `EvalError`, see interpreter.md.) This redirect belongs to
  batch runs; in an interactive session every table is printed to the
  console.

## Presentation

`present(table: TruthTable)` prints, in this order:

1. when simplification ran: the simplified formula in canonical form, e.g.
   `Simplificada: p ∧ ¬q`.
2. the truth table — the variable columns and the result column, aligned,
   with `V` and `F` as values, following the layout in truth_table.md; the
   result column is labelled with the proposition in normalized form,
   keeping the order and grouping the user wrote (redundant parentheses
   removed);
3. when simplification ran: a classification line, e.g.
   `Esta proposición es una tautología.`

When the user passed `--no-simplificar`, the simplified formula is not
printed. Formulas are shown in infix notation by `InfixPrinter` — a static
printer alongside `ASTPrinter` — and always in the one normalized form:
`¬`, `∧`, `∨`, `→`, `↔`, `V` / `F`, parentheses only where precedence
requires (see truth_table.md).

## Interactive mode

`start_interactive()` runs a read-eval-print session:

- The prompt `> ` accepts one proposition per line; each line is evaluated
  on its own (`evaluate` over that single line) and presented immediately.
- An error on one line prints its formatted errors and the session
  continues — a bad proposition does not end the session.
- A blank line evaluates nothing and just shows a new prompt; an empty
  session is not an error.
- `exit`, `salir`, or end-of-input (Ctrl-D) ends the session.

## Errors and exit codes

- Formatted errors (via `Orchestrator.report`) go to stderr, one per line;
  truth tables go to stdout.
- Exit codes: 0 on success, 1 when evaluation failed (including a file with
  no propositions), 2 for usage errors.
- I/O problems (for example an unreadable file) are reported by the CLI
  directly; they are not part of the `Error` model.
