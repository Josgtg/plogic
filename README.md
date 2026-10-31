# plogic

A propositional logic evaluator: it reads propositions, simplifies them, and builds a
truth table for each one.

> **Status: design phase.** The language, the pipeline and the CLI are fully designed
> in [`design/`](design/), but the Rust implementation is still a stub (`src/main.rs`
> prints `Hello, world!`). Everything below documents the intended behavior.

## Build and install

Requires a Rust toolchain with edition 2024 support.

```sh
cargo build --release
./target/release/plogic 'p -> q'
```

## Usage

Three modes, chosen by flag. A bare argument is evaluated as a single proposition.

| Flag | Mode |
| --- | --- |
| *(none)* | single proposition, from the argument |
| `-p`, `--proposicion <proposition>` | single proposition |
| `-a`, `--archivo <path>` | one proposition per line |
| `-i`, `--interactivo` | interactive session |

Modifiers:

- `--no-simplificar` — skip simplification.
- `-s`, `--salida <path>` — write the output to a file instead of the console.

`-s` cannot be combined with `--interactivo`. With no argument at all, more than one
mode flag, an unknown flag, or `-s` with `--interactivo`, plogic prints a usage message
and exits with code 2.

```sh
plogic 'p -> q'
plogic --proposicion '(p o q) y no r'
plogic --archivo proposiciones.txt --no-simplificar
plogic --archivo proposiciones.txt --salida resultados.txt
plogic --interactivo
```

In interactive mode each line is evaluated on its own and presented immediately; a bad
line reports its errors without ending the session, and `exit`, `salir` or Ctrl-D ends
it.

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
- **Identifiers** are ASCII letters followed by letters or digits (`p`, `q1`, `Premisa`),
  case-sensitive. The words used as spellings (`no`, `y`, `o`, `entonces`, `sisolosi`,
  `V`, `F`) are reserved and cannot be identifiers.
- **Parentheses** group a subexpression.

Precedence, lowest to highest: biconditional, implication, disjunction, conjunction,
negation, primary. Implication is right-associative, so `p -> q -> r` means
`p -> (q -> r)`; every other binary connective is left-associative.

A proposition ends at the end of its line: multi-line propositions are not supported.

## Output

Every formula is displayed in one normalized form: `¬`, `∧`, `∨`, `→`, `↔`, the literals
`V` and `F`, and only the parentheses precedence requires. The order the proposition was
written in is kept.

`plogic 'p -> q'` prints the simplified formula and the classification (both omitted
under `--no-simplificar`), then the table:

```
Simplificada: q ∨ ¬p

p  q  | p → q
F  F  | V
F  V  | V
V  F  | F
V  V  | V

Esta proposición es una contingencia.
```

- Rows follow binary counting order, the last variable varying fastest, so the first row
  assigns `F` to every variable.
- One column per distinct variable, in order of first appearance.
- The simplified formula comes out in canonical order (a variable before a negation,
  `F` before `V`); the result column keeps the order and grouping the proposition was
  written in.
- Classification is `tautología` when the result column is all `V`, `contradicción` when
  it is all `F`, `contingencia` otherwise.

Truth tables go to stdout and errors to stderr. Output moves to a file in two cases:
with `-s`, or automatically to `plogic-output.txt` when any table of the run has more
than 1024 rows (the user is notified). A table of more than 4096 rows is refused: 13 or
more variables is an error, so at most 12 variables are accepted.

## Errors

Errors are reported in Spanish, one per line, with the position of the offending
character or token:

```
línea 1, columna 3: carácter inválido '%'
línea 1, columna 4: se esperaba una expresión
línea 1, columna 3: se esperaba el fin de la proposición
```

A run with errors produces no truth tables, but reports as many errors as it can find.
An input file with no propositions is an error too (`el archivo no contiene
proposiciones`).

Exit codes: `0` success, `1` evaluation failed or the file had no propositions, `2`
usage error.

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
