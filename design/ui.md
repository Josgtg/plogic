# Plogic - UI / CLI design

Role: the impure UI. `CLI` implements the `UI` interface, which is unchanged:
`receive_file(path)`, `receive_proposition(proposition)`, `start_interactive()`
and `present(table)`. The GUI in [gui.md](gui.md) is a second impure UI
component that calls the Orchestrator directly. Both present their content in
Spanish.

## Modes — explicit flags

A bare `plogic` (no arguments at all) does not reach this CLI: it opens the
graphical window in [gui.md](gui.md). Every invocation with at least one
argument — a flag, a proposition, `-h`, `-V` or `--` — keeps the behavior
below. The CLI has the following flags. With no mode flag, one bare argument
is evaluated as a proposition (single proposition mode):

| Flag | Mode | Maps to |
| --- | --- | --- |
| `none` | single proposition (default) | `receive_proposition` |
| `-p`, `--proposicion <proposition>` | single proposition | `receive_proposition` |
| `-a`, `--archivo <path>` | one proposition per line | `receive_file` |
| `-i`, `--interactivo` | interactive session | `start_interactive` |

Modifier flags (they combine with `-p`, `-a`, the bare argument and, except
for `-o`, with `-i`):

- `--no-simplificar` disables the Simplifier: the table is built from the
  original proposition and no `Simplificada:` line is shown.
- `-s`, `--simplificar` simplifies and prints only the simplified formula:
  no truth table and no classification. It cannot be combined with
  `--no-simplificar`.
- `-m`, `--mostrar-proceso` adds the `Proceso:` list: the applied laws in
  order, each with the formula it produced. It combines with the default table
  and with `-s`; it cannot be combined with `--no-simplificar`, since no
  simplification exists to trace.
- `-o <path>` (long form `--salida <path>`) writes the output to the file
  in `<path>` rather than to the console.

By default propositions are simplified and the table is built from the
simplified expression: its variables, rows, size check and result-column
label all come from the simplified form, and the `Simplificada:` line
repeats that label. With `--no-simplificar` the table is built from the
original proposition. The classification is shown in both cases.

The mode, modifier and output flag names are Spanish. `--help` and `--version`
keep their conventional English names. With no mode and no proposition (a
modifier-only invocation), more than one mode flag, an unknown
flag, a missing flag value, an extra positional argument, or a single
proposition that is empty after trimming, the CLI prints a usage message and
exits with code 2.
`--simplificar` together with `--no-simplificar`, `--mostrar-proceso`
together with `--no-simplificar`, and `--salida`
together with `--interactivo`, are also usage errors: there is nothing to
simplify, nothing to trace, and a session produces many tables with no single
file for them.

A flag may appear only once. A repeated mode flag (`-p`, `-a`, `-i`) is
`solo se puede usar un modo a la vez`; a repeated `-s`, `-m`,
`--no-simplificar` or `--salida` is `argumento inesperado: '<flag>'`, with the
flag as the user typed it (ADR-031).

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

## Graphical window

Running `plogic` with no arguments opens the window in [gui.md](gui.md). It
evaluates one proposition at a time. The whole window is one vertical scroll
area: the centered components — the formula, the optional process, the optional
table and the classification — flow in the main column, with no separate
result pane. `Simplificar`, `Mostrar tabla` and `Mostrar proceso` are all read
once, when `Evaluar` is pressed; changing one afterwards only affects the next
evaluation. `Mostrar
tabla` off means the table is not processed at all; when it is on, the table is
built only for fewer than ten variables, and the classification appears only
with a table. Editing, inserting or clearing the formula does not replace the
stored result either; only `Evaluar` does. The `Ayuda` list is collapsed by
default and is one column set when opened. The CLI modes `-a`, `-i` and `-o`
are not available in the GUI.

## Help and version

`-h` / `--help` prints this help screen on stdout and exits 0:

```text
plogic: evalúa proposiciones de lógica proposicional

uso: plogic [opciones] [proposición]

argumentos:
  [proposición]  evalúa una proposición (entre comillas)

opciones:
  -p, --proposicion <texto>  evalúa una proposición (entre comillas)
  -a, --archivo <archivo>    evalúa una proposición por línea
  -i, --interactivo          inicia una sesión interactiva
      --no-simplificar       no simplifica la proposición
  -s, --simplificar          muestra solo la proposición simplificada
  -m, --mostrar-proceso      muestra el proceso de simplificación
  -o, --salida <archivo>     escribe la salida en <archivo>
  -h, --help                 muestra esta ayuda
  -V, --version              muestra la versión
```

The screen is generated from the argument definitions: each argument
contributes its value name, its one-line description and the section it
belongs to, so a new argument appears in the help without a second list to
maintain. The usage line is the compact `uso: plogic [opciones]
[proposición]`, and every description is lower-case, like the error messages.

`-V` / `--version` prints exactly `plogic 1.0.0` on stdout and exits 0. Neither
flag is a mode. Both are handled by the command-line parser, which prints them
when it reaches them: an error raised by an earlier argument wins, so
`plogic -x --help` and `plogic -p --help` are usage errors, while
`plogic --help -x` prints the help and `plogic -V --help` prints the version.
`--` still ends option parsing, so `plogic -- --help` evaluates the proposition
`--help` (ADR-033).

## File mode

`receive_file(path)` is where file mode starts: the CLI reads the file (it
is one of the impure UI components) and passes the text to `Orchestrator.evaluate` (or
`Orchestrator.simplify` with `--simplificar`), so the Orchestrator stays
pure. A file with no propositions — empty, or only blank lines — is not
evaluated: the CLI prints `el archivo no contiene proposiciones` on stderr
and exits with code 1.

A file run presents its tables (or its simplified formulas with
`--simplificar`) in input order, numbered: each block starts with
`Proposición n: <proposición>` — the number and the proposition exactly as the
file line holds it, trimmed (ADR-053) — then a blank line, then the normal
presentation, and one blank line separates consecutive blocks. Numbering
counts only the propositions actually built, so blank lines in the file do not
advance the counter; single-proposition mode has no heading (ADR-045).

## Output destination

Truth tables go to stdout and errors to stderr. The output is written to a
file instead of the console in two cases (the UI does the writing — the
Orchestrator stays pure):

- `-o` / `--salida` writes the output to the given path; the explicit
  flag wins over the automatic file name. The file is written only when
  there is output: a failing run writes nothing and leaves an existing file
  at `<path>` untouched. A write failure prints
  `no se pudo escribir el archivo '<path>': <detalle>` on stderr and exits
  1.
- When one of a run's truth tables has more than 2^10 rows, the output goes
  to `plogic-output.txt` and the user is notified on stderr with
  `la salida se escribió en plogic-output.txt`. This applies to every
  non-interactive run, single proposition or file, and only to runs that
  build tables: `--simplificar` never redirects. (A table of more than
  4096 rows never reaches presentation: the Interpreter refuses it with an
  `EvalError`, see interpreter.md.) In an interactive session every table
  is printed to the console and no redirect happens.

## Presentation

`present(table: TruthTable)` prints, in this order:

1. when simplification ran (the default): the simplified formula in
   canonical form, e.g. `Simplificada: p ∧ ¬q`. It repeats the table's
   label.
2. with `-m`/`--mostrar-proceso`, and only when at least one law applied: the
   `Proceso:` block, one numbered line per step, e.g.
   `1. 13a. Condicional: q ∨ ¬p`. The label is the law number and Spanish name
   from the Simplifier's `Rule`; the formula is the canonical result of that
   step. Steps appear in the order the Simplifier followed them.
3. the truth table — the variable columns and the result column, aligned,
   with `V` and `F` as values, following the layout in truth_table.md; the
   result column is labelled with the evaluated proposition in normalized
   form. In these cells only, `V` is green and `F` is red when the output is
   a terminal; the rest of the presentation is uncolored, and output written
   to a file is plain. By default that is the simplified formula in
   canonical order; with `--no-simplificar` it is the original proposition,
   keeping the order and grouping the user wrote (redundant parentheses
   removed).
4. always: a classification line, e.g.
   `Esta proposición es una tautología.`

When the user passed `--no-simplificar`, the simplified formula is not
printed (item 1); the table and the classification are still printed.
A blank line separates the printed items: after the simplified line, around
the process block, after the table, and before the classification. With
`--no-simplificar` only the last separator remains, between the table and the
classification.

With `--simplificar` there is no table and no classification: the
presentation is only the `Simplificada: <formula>` line and, with `-m`, its
`Proceso:` block, once per proposition, with the file-mode heading and block
separation when reading a file. When the Simplifier applied no law, the
process block is omitted entirely, even when `-m` was passed (ADR-047).

The table is left-aligned: every cell is padded to the width of the widest
text in its column, header included, with two spaces between columns and
`| ` before the result column. The last column is not padded, so no line ends
in spaces. A table with no variables has no `|` separator — the label is the
only header and each row holds just the result, e.g. `V` over `V` (ADR-045).
Formulas are shown in infix notation by `InfixPrinter` — a static printer
alongside `ASTPrinter` — and always in the one normalized form: `¬`, `∧`,
`∨`, `→`, `↔`, `V` / `F`, parentheses only where precedence or
associativity requires (see truth_table.md).

## Interactive mode

`start_interactive()` runs a read-eval-print session:

- The prompt `> ` accepts one proposition per line; each line is evaluated
  on its own (`evaluate` over that single line) and presented immediately.
- `--no-simplificar`, `--simplificar` and `--mostrar-proceso` each apply to
  every line; `--no-simplificar` with either of the other two is a usage
  error, as in the other modes.
- With `--simplificar`, each line prints its `Simplificada:` formula only;
  with `-m` it also prints that formula's `Proceso:` block.
- An error on one line prints its formatted errors and the session
  continues — a bad proposition does not end the session.
- A blank or whitespace-only line evaluates nothing and just shows a new
  prompt; an empty session is not an error. A presentation or an error is
  followed by a blank line before the next prompt (ADR-045).
- On a terminal the line is read with the rustyline editor (ADR-054): the
  left/right arrows move the caret, Up and Down walk the lines typed in this
  session (blank lines are not recorded), and the usual editing keys work.
  Ctrl-C cancels the current line and shows a fresh prompt; the session is
  sticky by design, so a cancelled line never ends it.
- When the input is a pipe or a redirected file, the session reads plain lines
  instead, so the prompt and the output stay byte-identical to the earlier
  behavior.
- `exit`, `salir`, or end-of-input (Ctrl-D) ends the session. End-of-input
  prints one newline after the pending `> `; the two words add none, since
  the user already pressed Enter (ADR-045).
- Interactive output always goes to the console; the automatic
  `plogic-output.txt` redirect never applies.

## Errors and exit codes

- Formatted errors (via `Orchestrator.report`) go to stderr, one per line;
  truth tables go to stdout.
- An error's message is bright red when stderr is a terminal and `NO_COLOR`
  is not set; a redirected error is plain, and the help screen below a usage
  error is never colored (ADR-035).
- Usage errors print, on stderr, the specific message, a blank line and the
  full help screen above, then exit 2 (ADR-034). The specific messages are
  `falta el valor de '<flag>'`, `opción desconocida: '<flag>'`, `solo se
  puede usar un modo a la vez`, `argumento inesperado: '<arg>'`,
  `'-o' no se puede usar con '--interactivo'`,
  `'--simplificar' no se puede usar con '--no-simplificar'`,
  `'--mostrar-proceso' no se puede usar con '--no-simplificar'` and
  `no se proporcionó ninguna proposición`. Help and version are the parser's
  and are printed when it reaches them, so an earlier argument's error wins
  (ADR-033). When several usage errors apply, the CLI reports one
  message, the first in this order: unknown flag, missing flag value, more than
  one mode, extra positional, no proposition,
  `--simplificar` with `--no-simplificar`, `--mostrar-proceso` with
  `--no-simplificar`, `-o` with `--interactivo` (ADR-027, ADR-047). A repeated
  flag counts at the place of the message it reuses (ADR-031).
- Exit codes: 0 on success or on help/version, 1 on any failing run (scan,
  parse, evaluation, a file with no propositions, or an I/O failure), 2 for
  usage errors.
- I/O problems are reported by the CLI directly; they are not part of the
  `Error` model. A read failure prints
  `no se pudo leer el archivo '<path>': <detalle>` and exits 1; an empty
  file prints `el archivo no contiene proposiciones` and exits 1.
