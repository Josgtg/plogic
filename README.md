# plogic

A propositional logic evaluator: it reads propositions, simplifies them, and builds a
truth table for each one.

## Use the application

You can download the latest release [here](https://github.com/Josgtg/plogic/releases/latest).

### Build from source

You can also build from the source. Requires a Rust toolchain with edition 2024 support.

Build:

```sh
cargo build --release
```

Run:

```sh
./target/release/plogic 'p -> q'
```

To put the binary on your `PATH`:

```sh
cargo install --path .
```

## Usage

Running `plogic` with no arguments at all opens the graphical window. With any
argument or flag, the command-line modes below apply.

Three modes, chosen by flag. A bare argument is evaluated as a single proposition.

| Flag | Mode |
| --- | --- |
| *(none)* | single proposition, from the argument |
| `-p`, `--proposicion <proposition>` | single proposition |
| `-a`, `--archivo <path>` | one proposition per line |
| `-i`, `--interactivo` | interactive session |

Modifiers:

- `--no-simplificar` — skip simplification and omit the `Simplificada:` line; the table
  evaluates the original proposition.
- `-s`, `--simplificar` — simplify and print only the `Simplificada:` formula, with no
  truth table and no classification. It cannot be combined with `--no-simplificar`.
- `-m`, `--mostrar-proceso` — print the ordered list of laws the Simplifier applied,
  under a `Proceso:` heading. It cannot be combined with `--no-simplificar`.
- `-o`, `--salida <path>` — write the output to a file instead of the console.

`-h` / `--help` prints the help screen on stdout and exits 0: the usage line
`uso: plogic [opciones] [proposición]` followed by every argument, generated from the
argument definitions. `-V` / `--version` prints exactly `plogic 1.0.0` and exits 0. The
parser prints them when it reaches them, so `plogic --help -x` shows the help but
`plogic -x --help` is an unknown-flag error, and `plogic -- --help` evaluates the
proposition `--help`.

`-o` cannot be combined with `--interactivo`; `-s` and `-m` cannot be combined with
`--no-simplificar`. With no proposition at all (a modifier-only invocation), more than
one mode flag, an unknown flag, a missing flag value, an extra positional argument, or a
single proposition that is empty after trimming, plogic prints the reason followed by the
full help screen and exits with code 2. The two conflicts above are usage errors too.

Every argument that begins with `-` is a flag, except `--`, so `plogic ->` is an
unknown-flag error. `--` ends flag parsing and is never a flag value: a following argument
is taken as the bare proposition even if it begins with `-`, only one positional is
allowed, and a trailing `--` is harmless when a proposition was already given
(`plogic -p p --`). A single proposition from the bare argument or from `-p` is trimmed
before the emptiness check; surrounding spaces are otherwise left for the scanner to
ignore.

```sh
plogic 'p -> q'
plogic --proposicion '(p o q) y no r'
plogic --archivo proposiciones.txt --no-simplificar
plogic --archivo proposiciones.txt --simplificar --salida resultados.txt
plogic --interactivo
plogic --help
```

In interactive mode each line is evaluated on its own and presented immediately; a bad
line reports its errors without ending the session, and `exit`, `salir` or Ctrl-D ends
it. On a terminal the arrows move the caret and Up/Down recall the lines typed in that
session; Ctrl-C cancels the current line without ending the session.
`--no-simplificar` and `--simplificar` apply to every line, but using them together
is a usage error.

## Graphical window

`plogic` with no arguments opens a dark, monospace window that evaluates one proposition
at a time. The whole window scrolls as one column: its centered components — the
simplified (or original) formula, the optional process list, a real truth table and the
classification — flow directly in the main window, not in a result box. The collapsed
help lists the accepted connective and literal spellings in one column set.
`Simplificar`, `Mostrar tabla` and `Mostrar proceso` are all read when `Evaluar` is
pressed: with `Mostrar tabla` off the table is not processed, and with it on the table is
built only for fewer than ten variables, so the classification appears only with a
table. Editing the formula does not
replace the previous result: only `Evaluar` does. Files, interactive mode and `-o`
remain CLI-only. If the window cannot open, plogic prints `no se pudo iniciar la
interfaz gráfica` on stderr and exits 1.

## The language

A proposition is built from variables, literals, connectives and parentheses. Every
connective can be written as a symbol or as a Spanish word:

| Connective | Spellings | Arity |
| --- | --- | --- |
| Negation | `!`, `¬`, `no` | unary |
| Conjunction | `&`, `∧`, `y` | binary |
| Disjunction | `\|`, `∨`, `o` | binary |
| Implication | `->`, `→`, `entonces` | binary |
| Biconditional | `<->`, `↔`, `sisolosi` | binary |

- **Literals** are written `0`, `1`, `F` or `V`; `0` and `F` mean false, `1` and `V`
  mean true.
- **Identifiers** are an ASCII letter followed by ASCII letters, digits or underscores
  (`p`, `q1`, `p_1`, `Premisa`), case-sensitive; an underscore cannot start one. The
  words used as spellings (`no`, `y`, `o`, `entonces`, `sisolosi`, `V`, `F`) are
  reserved and cannot be identifiers.
- **Parentheses** group a subexpression.

Precedence, lowest to highest: biconditional, implication, disjunction, conjunction,
negation, primary. Implication is right-associative, so `p -> q -> r` means
`p -> (q -> r)`; every other binary connective is left-associative.

A proposition ends at the end of its line: multi-line propositions are not supported.

## Output

Every formula is displayed in one normalized form: `¬`, `∧`, `∨`, `→`, `↔`, the literals
`V` and `F`, and only the parentheses precedence or associativity requires. By default
the result-column label is the simplified formula in canonical order, and the
`Simplificada:` line repeats it. With `--no-simplificar` the label keeps the order and
grouping the proposition was written in, with redundant parentheses removed, and the
`Simplificada:` line is omitted.

By default the table evaluates the simplified formula: its variables, rows, size check
and label all come from it. With `--no-simplificar` the table evaluates the original
proposition instead. The classification is printed in every case. With
`-s` / `--simplificar` only the simplified formula is printed, with no table. With
`-m` / `--mostrar-proceso`, a `Proceso:` block lists the laws the Simplifier applied, in
order, each with the formula it produced; it is omitted when no law applied.

`plogic 'p -> q'` prints:

```
Simplificada: q ∨ ¬p

Tabla de verdad:
q  p  | q ∨ ¬p
F  F  | V
F  V  | F
V  F  | V
V  V  | V

Clasificación: Esta proposición es una contingencia.
```

- Rows follow binary counting order, the last variable varying fastest, so the first row
  assigns `F` to every variable.
- One column per distinct variable, in order of first appearance in the evaluated
  expression.
- The simplified formula comes out in canonical order: constants first (`F` before `V`),
  then variables alphabetically, then negated variables, then `&` chains, then `|`
  chains. Under `--no-simplificar` the `Simplificada:` line is omitted.
- Classification is `tautología` when the result column is all `V`, `contradicción` when
  it is all `F`, `contingencia` otherwise.

In a terminal the `V` and `F` cells of the truth table are colored green and red
respectively, and the message of an error is bright red. Nothing else is colored — not
the formulas, the label, the classification or the help screen — and output written to a
file is plain text.

Truth tables go to stdout and errors to stderr. Output moves to a file in two cases:
with `-o`, or automatically to `plogic-output.txt` when any table of a non-interactive
run has more than 1024 rows (the user is notified on stderr). `-o` wins over the
automatic file name. Interactive sessions always print to the console, and
`--simplificar` never redirects. A table of more
than 4096 rows is refused: 13 or more variables in the evaluated expression is an error,
so at most 12 variables are accepted.

## Errors

Errors are reported in Spanish, one per line, with a position when the error has one. A
scan failure looks like this:

```
$ plogic 'p % q'
línea 1, columna 3: carácter inválido '%'
```

A parse failure looks like this:

```
$ plogic 'p q'
línea 1, columna 3: se esperaba el fin de la proposición
```

A mistyped or misplaced operator explains the accepted spelling:

```
$ plogic 'p v q'
línea 1, columna 3: 'v' no es un operador; usa '|', '∨' u 'o' para la disyunción
$ plogic 'p no q'
línea 1, columna 3: la negación ('!', '¬' u 'no') va antes de la expresión, no entre dos proposiciones
```

A failing stage reports the errors it found; later stages do not run, so a run with
errors produces no truth tables. An input file with no propositions is an error too
(`el archivo no contiene proposiciones`).

Exit codes: `0` success or help/version, `1` on any failing run (scan, parse,
evaluation, an empty input file, or an I/O failure), `2` usage error.

## Design

The implementation follows the pipeline designed in [`design/`](design/):

| Document | Contents |
| --- | --- |
| [`product_description.md`](design/product_description.md) | the product and the language |
| [`tokens.md`](design/tokens.md), [`scanner.md`](design/scanner.md), [`parser.md`](design/parser.md) | source text to AST |
| [`simplifier.md`](design/simplifier.md), [`interpreter.md`](design/interpreter.md), [`truth_table.md`](design/truth_table.md) | AST to truth table |
| [`orchestrator.md`](design/orchestrator.md), [`ui.md`](design/ui.md), [`error_handling.md`](design/error_handling.md) | pipeline driver, CLI, errors |
| [`decisions.md`](design/decisions.md), [`testing.md`](design/testing.md) | why the design is what it is, and how it will be verified |

The project follows the algorithms of
[Crafting Interpreters](https://craftinginterpreters.com/) by Robert Nystrom, in a
functional style: immutable data, errors as values, static entry points.

## Tech stack

- Rust 2024.
- [`clap`](https://docs.rs/clap) parses the command line and renders the help screen,
  using its `derive` feature. plogic supplies the Spanish words and the usage-error
  messages, and owns the usage-error precedence described above.
- [`thiserror`](https://docs.rs/thiserror) derives `Display` and `Error` for the typed
  error enum. There is no anyhow: the core returns `Result<T, List<Error>>`, and the
  CLI handles file I/O failures itself.
- [`proptest`](https://docs.rs/proptest) generates expressions for the property tests
  in [`design/testing.md`](design/testing.md).
- [`colored`](https://docs.rs/colored) colors the truth-table cells (`V` green, `F` red)
  and the message of an error (bright red on a terminal). Formulas, labels,
  classification, the help screen and output written to a file are not colored.
