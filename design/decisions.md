# Plogic - Decision log

One record per settled design decision. Each record states the decision and
its why and cost in a few lines; the design documents say how the decision is
carried out. Records are appended, never rewritten: a decision that changes
gets a new record that supersedes the old one. `Status` is `Accepted` or
`Superseded by ADR-nnn`. The log was condensed once, on request: the errata
that patched older records are folded into them, and every record states the
decision as it stands today.

## Index

| ID | Decision | Status |
| --- | --- | --- |
| 001 | Functional, static style instead of the book's classes | Accepted |
| 002 | Errors as values, with stage gates | Accepted |
| 003 | `TokenType` as a payload enum with one spelling map | Accepted |
| 004 | Line-oriented input: `Newline` ends a proposition | Accepted |
| 005 | Precedence ladder and associativity | Accepted |
| 006 | Batch parse with panic-mode synchronization | Accepted |
| 007 | Simplification by oriented rewrites to a fixed point | Accepted |
| 008 | Canonicalization (flatten + sort) inside the Simplifier | Accepted |
| 009 | One normalized display form | Accepted |
| 010 | Four literal spellings; short words reserved | Accepted |
| 011 | Size limits: 4096-row cap, 1024-row file redirect | Accepted |
| 012 | Spanish UI, messages and flags | Accepted |
| 013 | CLI surface: bare argument, `-p`, `-a`, `-i`, `-s` | Accepted |
| 014 | I/O errors live outside the `Error` model | Accepted |
| 015 | Two printers: `ASTPrinter` and `InfixPrinter` | Accepted |
| 016 | 1-based positions, columns counted in characters | Accepted |
| 017 | Default tabulates the original; `--tabla-simplificada` tabulates the simplified | Superseded by ADR-046 |
| 018 | Erase `Grouping` before simplification | Accepted |
| 019 | Stage results carry `List<Error>`; helpers carry one error | Accepted |
| 020 | Immutable `TruthTable` with `with_display` | Accepted |
| 021 | Interactive modifiers and always-on classification | Accepted |
| 022 | Strict CLI usage, I/O exit 1, redirect on every non-interactive run | Accepted |
| 023 | Simplification lowers weight, not necessarily connective count | Accepted |
| 024 | Dash-prefixed arguments are options; `--` ends option parsing; trim blank propositions | Accepted |
| 025 | Parser rule failures carry the `ParseState` | Accepted |
| 026 | `table_simplified` is ignored when `simplify` is false | Superseded by ADR-046 |
| 027 | Fixed precedence for usage errors | Accepted |
| 028 | `--` is an optional trailing separator | Accepted |
| 029 | Blank lines separate the presented sections | Accepted |
| 030 | Help and version flags | Accepted |
| 031 | Repeated flags reuse the existing usage messages | Accepted |
| 032 | Compact usage line and generated help screen | Accepted |
| 033 | The parser handles help and version, in argument order | Accepted |
| 034 | Usage errors print the message and the full help screen | Accepted |
| 035 | Errors are bright red on a terminal | Accepted |
| 036 | Token vocabulary details settled in implementation | Accepted |
| 037 | The error type displays as its report line | Accepted |
| 038 | The scanner owns the source; a lone CR is invalid | Accepted |
| 039 | An underscore continues an identifier but never starts one | Accepted |
| 040 | AST details: public fields, equality without positions, grouping in the printers | Accepted |
| 041 | Parser details: owned tokens, kind predicates and a total cursor | Accepted |
| 042 | Simplifier details: rule order, chain matching and expansion order | Accepted |
| 043 | Interpreter and truth-table details: borrowed expression, public fields | Accepted |
| 044 | Orchestrator arguments are owned | Accepted |
| 045 | CLI layout details: free functions, alignment, numbering and separators | Accepted |
| 046 | Simplified tables by default; `-s` prints only the simplified formula | Accepted |
| 047 | The Simplifier returns its applied steps, shown with `-m` | Accepted |
| 048 | A graphical window opens on a bare `plogic`, built with egui/eframe | Accepted |
| 049 | GUI modes are snapshotted; tables are only built below ten variables | Accepted |
| 050 | The parser explains that `v` is not an operator in operator position | Accepted |
| 051 | The GUI scrolls the whole window as one column | Accepted |
| 052 | Reserved words get role-specific parser diagnostics | Accepted |
| 053 | File-mode headings carry the proposition's source line | Accepted |
| 054 | The interactive session edits lines with rustyline | Accepted |
| 055 | The GUI toolbar has a delete button | Accepted |

## ADR-001 — Functional, static style instead of the book's classes

Status: Accepted

Every stage is a static entry point over immutable values: `Scanner.scan`,
`Parser.parse`, `Simplifier.simplify`, `Interpreter.interpret`,
`Orchestrator.evaluate`. Intermediate state (`ScanState`, `ParseState`) is
threaded through helpers and returned anew; no stage is ever constructed.
Plogic follows the book's algorithms, not its mutable-instance state model.
Cost: helpers pass state explicitly, so signatures are longer.

## ADR-002 — Errors as values, with stage gates

Status: Accepted

User mistakes are normal input, not exceptions. Every fallible stage returns
`Result`, recovers maximally inside itself, and discards its partial value if
it recorded any error; between stages the pipeline stops at the first failing
stage, so one `Err` list holds errors of exactly one kind. Deliberate cost: a
run with any error produces no truth tables, as the product promises.

## ADR-003 — `TokenType` as a payload enum with one spelling map

Status: Accepted

`TokenType` is a Rust-style enum with payloads (`Literal(bool)`,
`Identifier(str)`), and `TokenType.map(lexeme): Result<TokenType, str>` is the
only place where spellings are defined; an unknown word becomes an identifier.
One table to keep in sync with the product description, and free mixing of
spellings (`p o q`, `p ∨ q`, `p | q` are the same tokens).

## ADR-004 — Line-oriented input: `Newline` ends a proposition

Status: Accepted

The scanner emits `Newline` instead of swallowing line breaks; a proposition
ends at `Newline` or `Eof`, and blank lines are skipped. One proposition per
line is a hard rule of the language, which keeps file mode and error positions
simple and rules out wrapping long propositions. Multi-line propositions are
out of scope.

## ADR-005 — Precedence ladder and associativity

Status: Accepted

Five levels, lowest to highest: biconditional → implication → disjunction →
conjunction → negation → primary, one parser method per level. Implication is
right-associative (`p → q → r` is `p → (q → r)`); every other binary level is
left-associative. The ladder matches the connective family grouping and the
book's ladder depth; mixed chains still need parentheses to be readable, which
the display makes explicit.

## ADR-006 — Batch parse with panic-mode synchronization

Status: Accepted

The parser owns the statement loop and returns one expression per proposition.
On a parse error it records the `ParseError`, synchronizes to just after the
next `Newline` (a pure `ParseState → ParseState`), and keeps parsing, so every
bad proposition in a file is reported in one run. Cost: the stage gate of
ADR-002 — no tables when any line fails.

## ADR-007 — Simplification by oriented rewrites to a fixed point

Status: Accepted

Each law is fixed to one direction and applied as a local, bottom-up rewrite,
re-canonicalizing until a fixed point; termination is argued with a weight
`W` that every reduction rule strictly lowers. Laws that grow the formula
(distributivity read left-to-right) run only in an optional expansion phase,
and a candidate is kept only if `W` strictly drops. No normal form is
guaranteed, so the classification comes from the truth table, not from the
reduced formula.

## ADR-008 — Canonicalization (flatten + sort) inside the Simplifier

Status: Accepted

Before applying laws, `&` and `|` chains are flattened into one operand list
and sorted in a total, structural order, so "do these operands match?" is a
plain equality check. The lists live inside the Simplifier only; the AST keeps
binary nodes and a chain is rebuilt as left-nested `Binary` nodes on return.
Cost: a sort on every rewrite, and a canonical order that must be total.

## ADR-009 — One normalized display form

Status: Accepted

Every formula is displayed through `InfixPrinter` in one normalized form:
`¬`, `∧`, `∨`, `→`, `↔`, literals `V` / `F`, operand order as written, and
only the parentheses precedence and associativity require — the left child of
`→` at equal precedence is parenthesized, so `(p -> q) -> r` keeps its
parentheses while `p -> (q -> r)` does not. The formal symbols are accepted as
input too; design documents keep writing `!`, `&`, `|`, `->`, `<->` because
they are easier to type. No lexeme plumbing; a non-UTF-8 terminal shows
mojibake, by design.

## ADR-010 — Four literal spellings; short words reserved

Status: Accepted

Literals are written `0`, `1`, `F` or `V`; display always uses `V` / `F`. The
words that spell connectives (`no`, `y`, `o`, `entonces`, `sisolosi`) and the
literals (`V`, `F`) are reserved and can never be identifiers. Asymmetrically,
lowercase `v` and `f` are valid variables; accepted because one-letter names
that clash with reserved words are confusing anyway.

## ADR-011 — Size limits: 4096-row cap, 1024-row file redirect

Status: Accepted

Two separate limits. More than 4096 rows is refused before any row is built
(`Err([EvalError("resultado muy largo: más de 4096 filas")])`), so the
expression chosen for the table has at most 12 variables. More than 1024 rows
in any table of a non-interactive run (ADR-022 widened the earlier batch-only
scope) sends the run's output to `plogic-output.txt` with a notification. The
two numbers are different on purpose — 1024 is where reading becomes
impractical, 4096 is where the run refuses — but they are easy to confuse.

## ADR-012 — Spanish UI, messages and flags

Status: Accepted

Everything the user reads is Spanish: prompts, table headers, classification
lines, error messages and flag names. Messages are lower-case and say what was
expected; the labels `Simplificada:` and `Esta proposición...` keep their
capital initial (ADR-021). Implementation strings are part of the product, and
a second language is out of scope.

## ADR-013 — CLI surface: bare argument, `-p`, `-a`, `-i`, `-s`

Status: Accepted

`-p` / `--proposicion` (or a bare argument with no mode flag) for one
proposition, `-a` / `--archivo` for a file with one proposition per line,
`-i` / `--interactivo` for a session; modifiers `--no-simplificar` and the
output flag. No argument beyond modifiers, more than one mode flag, an unknown
flag, or the output flag with `-i` is a usage error (exit 2). A bare argument
is convenient but ambiguous with a mistyped flag (ADR-024 settles that). The
flag surface evolved after this record: `--salida` moved to `-o` and `-s`
became `--simplificar` (ADR-046), and `-m` / `--mostrar-proceso` was added
(ADR-047).

## ADR-014 — I/O errors live outside the `Error` model

Status: Accepted

The `Error` enum covers the language only (`ScanError`, `ParseError`,
`EvalError`); unreadable files and unwritable output paths belong to the CLI,
which reports them directly with exit 1. An input file with no propositions is
reported as `el archivo no contiene proposiciones`. Cost: the CLI owns two
reporting paths and must keep their wording consistent.

## ADR-015 — Two printers: `ASTPrinter` and `InfixPrinter`

Status: Accepted

`ASTPrinter` renders parenthesized prefix notation as a verification tool;
`InfixPrinter` renders the normalized infix form (ADR-009) and is the only
printer for user-facing formulas. Two traversals to keep in step, in exchange
for readable output and a debugging aid whose parentheses never depend on
precedence.

## ADR-016 — 1-based positions, columns counted in characters

Status: Accepted

`line` and `col` are 1-based and point at the first character of the token;
`col` counts characters from the last line break, so a UTF-8 symbol or a tab
is one position. The column is derived from the source and the token's start
offset — `ScanState` stores no column counter. Reported positions match an
editor gutter; a tab makes the reported column differ from the visual one.

## ADR-017 — Default tabulates the original; `--tabla-simplificada` tabulates the simplified

Status: Superseded by ADR-046

The original decision tabulated the original expression by default, so every
variable the user wrote appeared in first-appearance order, and offered
`--tabla-simplificada` for the simplified table. ADR-046 inverted the default,
removed the flag, and made `--no-simplificar` the escape hatch.

## ADR-018 — Erase `Grouping` before simplification

Status: Accepted

`Simplifier.simplify` strips every `Grouping` recursively before
canonicalization or reduction: the weight, canonical order and law matching
are defined on the other variants only. Parentheses survive in the original
label and in `--no-simplificar` output; the simplified formula relies on
`InfixPrinter`'s precedence-aware parentheses.

## ADR-019 — Stage results carry `List<Error>`; helpers carry one error

Status: Accepted

Public stage entry points return `Result<T, List<Error>>` with a non-empty
list; internal helpers return `Result<T, Error>`, the scanner's main case.
Parser rules are the exception: they return `Result<T, (Error, ParseState)>`
so recovery resumes from the failing token (ADR-025), and `TokenType.map`
returns `Result<TokenType, str>` because it has no position. The stage wraps a
helper error into a one-element list before returning.

## ADR-020 — Immutable `TruthTable` with `with_display`

Status: Accepted

The Interpreter creates the `TruthTable` with variables and rows and empty
display fields; `with_display(label, simplified)` returns a new table with
them set, called once by the Orchestrator. No setters, no mutation, so the
table is immutable end to end.

## ADR-021 — Interactive modifiers and always-on classification

Status: Accepted

`--no-simplificar` (and, before ADR-046 removed it, `--tabla-simplificada`)
apply in interactive mode, and conflicts between them are usage errors there
too. The classification (`tautología`, `contradicción`, `contingencia`) is
always printed, from the table's result column, in every mode — it never
depends on whether a simplified line was shown.

## ADR-022 — Strict CLI usage, I/O exit 1, redirect on every non-interactive run

Status: Accepted

The CLI is strict: a modifier-only invocation, a missing flag value, an extra
positional, more than one mode, conflicting flags, or an empty `-p ''` is a
usage error — one message, exit 2 (the precedence is ADR-027; repeated flags
reuse the same messages, ADR-031). The output flag writes only on success.
Reading or writing a file fails with exit 1 and a Spanish message; the `Error`
model is never used for I/O. The automatic `plogic-output.txt` redirect
applies to any non-interactive run when any table has more than 1024 rows; the
explicit `-o` wins; interactive sessions never redirect. The exact strings
live in `ui.md`.

## ADR-023 — Simplification lowers weight, not necessarily connective count

Status: Accepted

The guarantee is the same truth table and a weight that is never greater —
strictly lower whenever a rewrite applies, equal at the fixed point. The
simplified formula need not have fewer connectives (De Morgan and the
biconditional expansion add connectives while lowering `W`);
`--no-simplificar` keeps the original untouched. Documents and tests assert
equivalence and decreasing weight, not a smaller connective count.

## ADR-024 — Dash-prefixed arguments are options; `--` ends option parsing; trim blank propositions

Status: Accepted

Every argument that begins with `-` is an option unless it appears after `--`,
which ends option parsing and passes the next argument through as the bare
proposition (`plogic ->` is `opción desconocida: '->'`; `plogic -- '->'`
evaluates `->`). A single proposition, bare or from `-p`, is trimmed, and if
nothing remains the CLI reports `no se proporcionó ninguna proposición`
(exit 2); the scanner still receives the original text. No valid proposition
begins with `-`, so strictness costs nothing in practice. Refined by ADR-028.

## ADR-025 — Parser rule failures carry the `ParseState`

Status: Accepted

Every parser rule returns `Result<(Expression, ParseState), (Error,
ParseState)>` (and `consume` likewise), with the state at the failing token on
failure. `parse` records the error and synchronizes from that exact state, so
recovery never resumes several tokens behind. Cost: heavier rule signatures,
and every failure path must return the state.

## ADR-026 — `table_simplified` is ignored when `simplify` is false

Status: Superseded by ADR-046

The original decision made `evaluate` total over its flag combinations by
ignoring `table_simplified` when `simplify` was false. ADR-046 removed the
parameter: `evaluate(source, simplify)` always tabulates the chosen tree.

## ADR-027 — Fixed precedence for usage errors

Status: Accepted

When several usage errors apply at once, the CLI reports exactly one message,
the first that applies in this order: unknown flag, missing flag value, more
than one mode, extra positional argument, no proposition, then the modifier
conflicts — `--simplificar` with `--no-simplificar`, `--mostrar-proceso` with
`--no-simplificar`, `-o` with `--interactivo` (ADR-046 renamed the flags and
settled this set). Exit code is always 2. A new usage check must be placed in
the order and the order documented.

## ADR-028 — `--` is an optional trailing separator

Status: Accepted

`--` is always a separator, never an option value. It may be trailing when a
proposition already exists (`plogic -p p --` is valid); `plogic --` alone is a
usage error, and `plogic -p -- p` fails with `falta el valor de '-p'`. Two
positionals after `--` are an extra-positional error. A proposition that is
literally `--` cannot be passed; the language has no such token.

## ADR-029 — Blank lines separate the presented sections

Status: Accepted

`present` prints one blank line between the printed items: after the
simplified line, after the table, and before the classification; with
`--no-simplificar` only the table-classification separator remains. Output is
stable for golden tests and matches the README.

## ADR-030 — Help and version flags

Status: Accepted

The CLI supports `-h`/`--help` and `-V`/`--version`, both to stdout with exit
0; they are not modes and do not combine with a proposition. The generated
help screen and the in-order handling live in ADR-032 and ADR-033, which
superseded this record's hand-written help block and its pre-scan precedence.

## ADR-031 — Repeated flags reuse the existing usage messages

Status: Accepted

A repeated flag is a usage error with exit 2 and no new strings: a repeated
mode flag reports `solo se puede usar un modo a la vez` (it counts as more
than one mode), any other repetition reports `argumento inesperado: '<flag>'`
(it counts as an extra positional). Each repetition takes the place of the
message it reuses in the ADR-027 order.

## ADR-032 — Compact usage line and generated help screen

Status: Accepted

The usage line is the compact `uso: plogic [opciones] [proposición]`, and the
help screen is generated from the argument definitions, grouped in
`argumentos:` and `opciones:` with the help and version flags last; its fixed
words stay Spanish and lower-case. A new argument appears in the help as soon
as it is declared. Error messages remain plogic's own exact Spanish strings;
the parser's error text is never shown.

## ADR-033 — The parser handles help and version, in argument order

Status: Accepted

`-h`/`--help` and `-V`/`--version` are the argument parser's: it prints them
to stdout and exits 0 when it reaches them, so an error raised by an earlier
argument wins (`plogic -x --help` is a usage error; `plogic --help -x` prints
the help; `plogic -- --help` evaluates the proposition `--help`). One flag
mechanism, no duplicate help block, no pre-scan.

## ADR-034 — Usage errors print the message and the full help screen

Status: Accepted

A usage error prints, on stderr and in this order, the specific message, a
blank line, and the full help screen — the same one `-h`/`--help` prints, so
there is no second copy — then exits 2. The message stays first so the cause
is read before the reference. Cost: usage errors grow to a full screen,
acceptable because they end the run.

## ADR-035 — Errors are bright red on a terminal

Status: Accepted

The message of an error is painted bright red, but only when the stream it
goes to (stderr) is a terminal and `NO_COLOR` is not set; a redirected error
is plain text, and so is everything written to a file. For a usage error only
the cause line is painted; the help screen below stays plain. The same
treatment applies to scan, parse and evaluation errors, so every error looks
alike.

## ADR-036 — Token vocabulary details settled in implementation

Status: Accepted

`TokenType.map` composes its error message from the lexeme's shape — digits
give `literal inválido '<lexeme>'`, anything else `carácter inválido
'<lexeme>'` — and the scanner records it unchanged. The display spelling is
`Display` on `TokenType`, with `Token` delegating to its kind; `Newline` and
`Eof` render as the empty string. `Token` is plain data with public fields
(`kind` because `type` is a Rust keyword).

## ADR-037 — The error type displays as its report line

Status: Accepted

`Error` derives `thiserror::Error`, and the two positioned variants print
`línea {line}, columna {col}: {message}` — exactly the report line, so
`Orchestrator.report(error)` is `error.to_string()` and the Spanish wording
lives in one place. `EvalError` prints its message alone. The variant names
keep the `Error` suffix that clippy dislikes; the allow is local to the enum.

## ADR-038 — The scanner owns the source; a lone CR is invalid

Status: Accepted

`ScanState` owns the source and `Scanner.scan` takes it by value, so no helper
carries a lifetime and the text is dropped with the state — nothing after
scanning needs it. A carriage return not followed by LF is not a line break:
it is a `carácter inválido` error, so only LF and CRLF end a line. Helpers
follow idiomatic Rust (`match_char`, optional `peek`, `advance` returning the
new state). A CR-only file fails on its first line rather than being silently
accepted.

## ADR-039 — An underscore continues an identifier but never starts one

Status: Accepted

A word begins at an ASCII letter and continues over ASCII letters, digits and
underscores, so `p_1`, `a_b` and `p__q` are identifiers while `_p` and a bare
`_` stay `carácter inválido '_'`. Reserved spellings are unaffected: none
contains an underscore.

## ADR-040 — AST details: public fields, equality without positions, grouping in the printers

Status: Accepted

`Expression` node fields are public and nodes are built by pattern, not
constructors; `PartialEq`/`Eq` compare shape and operator kinds, never
positions, so the Simplifier's structural comparisons are sound and tests can
compare hand-built trees. `ASTPrinter` prints a `Grouping` as `(group …)`, a
faithful tree dump; `InfixPrinter` drops parentheses for a right-nested `∧` or
`∨` because they are associative, and keeps `p ↔ (q ↔ r)` explicit.

## ADR-041 — Parser details: owned tokens, kind predicates and a total cursor

Status: Accepted

`ParseState` owns `Vec<Token>`, so no helper carries a lifetime. `check` and
`match` take a predicate over `&TokenType` (total over the payload variants),
and `match` returns the matched token so the rules never need the book's
`previous`. The cursor is total: `peek` falls back to an `Eof` token past the
end, so a malformed list cannot read out of bounds or panic — errors are
values, never panics. `parse` does not consume `Eof`; `proposition` consumes
the terminating `Newline`, and a blank line is skipped by the script loop.

## ADR-042 — Simplifier details: rule order, chain matching and expansion order

Status: Accepted

Reduction rules are tried in the law-table order; absorption and redundancy
match every ordered pair of a flattened chain (`p | q | (p & s)` absorbs to
`p | q`). Each rewrite is a single step in a bottom-up, left-to-right scan
that restarts from the root after canonicalizing; the expansion phase visits
pre-order, tries law 4a before 4b, and distributes the rest of a chain too.
Nodes the rules create use synthetic `!`, `&`, `|` tokens at line 0, column 0,
indistinguishable from parsed ones because equality ignores positions.
`simplify` takes `&Expression`, and the weight saturates instead of
overflowing, so a pathological input stops simplifying instead of panicking.

## ADR-043 — Interpreter and truth-table details: borrowed expression, public fields

Status: Accepted

`Interpreter.interpret` takes `&Expression`, like `Simplifier.simplify`, so
the Orchestrator keeps one tree for both the table and the label without
cloning. `TruthTable` and `Row` expose their fields as public plain data; the
only methods stay `with_display` and `classify`. A hand-built empty table
classifies as `Contingency`, so the two "all" cases are never vacuously true
at once. Identifier lookup falls back to false instead of indexing, keeping
evaluation panic-free even on a malformed tree.

## ADR-044 — Orchestrator arguments are owned

Status: Accepted

`evaluate(source: String, ...)` and `report(error: Error)` take their owned
arguments by value, and the UI moves the source and each error in, so no call
site copies its payload. The asymmetry with `interpret(&Expression)` is
deliberate: those stages read a tree the Orchestrator still needs, while the
Orchestrator consumes text and errors nobody reads again.

## ADR-045 — CLI layout details: free functions, alignment, numbering and separators

Status: Accepted

The CLI is a set of free functions — `cli::run(config) -> i32` dispatching on
`Input`, private helpers per mode, a thin `main` for the exit code. Table
cells are left-aligned to the widest text of their column, separated by two
spaces, with no trailing spaces. File mode numbers built tables with
`Proposición n:` blocks separated by one blank line; single-proposition mode
has no heading. In interactive mode a presentation or error is followed by a
blank line before the next prompt; end-of-input prints one newline after the
pending `> `, while `exit` and `salir` add none. Table cells are painted by
temporarily overriding `colored`'s terminal detection, since cells may go to
stdout while error color depends on stderr.

## ADR-046 — Simplified tables by default; `-s` prints only the simplified formula

Status: Accepted

The default simplifies and builds the table from the simplified expression;
`--no-simplificar` disables the Simplifier and tabulates the original. The
formula-only mode is `-s` / `--simplificar`: print `Simplificada: <formula>`,
no table, no classification, no size limit, in every mode. The output flag
moved to `-o` / `--salida`, `--tabla-simplificada` was removed,
`Orchestrator.evaluate` became `evaluate(source, simplify)`, and a new
`Orchestrator.simplify(source)` builds no tables. The CLI surface breaks once,
before publication.

## ADR-047 — The Simplifier returns its applied steps, shown with `-m`

Status: Accepted

`Simplifier::simplify(&Expression)` returns
`Simplification { expression, steps }`, where each `Step { rule, expression }`
names one applied law (`10a. De Morgan`) and holds the canonical formula it
produced; grouping erasure, canonicalization and discarded expansion
candidates are not steps. `Orchestrator::evaluate` returns
`Vec<Evaluation { table, simplification: Option<Simplification> }>` and
`Orchestrator::simplify` returns `Vec<Simplification>` — the trace is data,
and only the CLI turns it into text. The flag is `-m` / `--mostrar-proceso`,
which conflicts with `--no-simplificar`. The `Proceso:` block follows
`Simplificada:` with one numbered line per step and is omitted entirely when
no law applied.

## ADR-048 — A graphical window opens on a bare `plogic`, built with egui/eframe

Status: Accepted

Running `plogic` with no arguments opens a native window built with
egui/eframe 0.37; any argument, flag included, keeps the CLI exactly as it is
(`main` intercepts the bare run, and `args::parse([])` still returns
`NoProposition`). The GUI evaluates one proposition at a time: help, three
controls (`Simplificar`, `Mostrar tabla`, `Mostrar proceso`), the formula
field, the insertion toolbar, `Evaluar`, and then the result components in the
same flow — no fixed output box. Only `Evaluar` (or Enter) replaces the stored
result; editing and the display controls do not. The toolbar inserts
`¬ ∧ ∨ → ↔ ( )` at the cursor and a variable button inserts the next unused
letter from `p` to `z`, skipping `y` (reserved; `o` is before `p` and is not a
candidate, and `p` is returned when every usable letter is taken). Dark,
monospace, fixed to the mockup's theme; if it cannot open, plogic prints
`no se pudo iniciar la interfaz gráfica` and exits 1. Cost: eframe adds a
large dependency tree, and the window cannot be exercised headlessly — state
transitions are tested directly, layout is checked by eye. The layout was
refined by ADR-049 and ADR-051.

## ADR-049 — GUI modes are snapshotted; tables are only built below ten variables

Status: Accepted

The GUI's three controls are read once, when `Evaluar` (or Enter) is pressed;
changing any control afterwards only affects the next evaluation, and editing
the formula never replaces the stored result. The GUI calls
`Orchestrator.assess(source, simplify, max_variables)`, which returns
`Assessment { formula, variables, simplification, table: Option }`: with
`Mostrar tabla` on it passes `Some(10)`, so a table is built only when the
chosen expression has fewer than ten distinct variables (otherwise a muted
note shows the variable count); with it off, `None` and no table is built at
all. The classification appears only when a table was built. The CLI keeps
`evaluate` and its 12-variable reach unchanged. This record's result-only
scroll layout was replaced by ADR-051.

## ADR-050 — The parser explains that `v` is not an operator in operator position

Status: Accepted

`v` is a common mistyping for the disjunction but scans as a variable name
(ADR-003), so when the parser finds an identifier named `v` where an operator
belongs it reports `'v' no es un operador; usa '|', '∨' u 'o' para la
disyunción` at that token. `v` in operand position stays a legal variable, and
every other leftover token keeps the generic message. The special case lives
in the parser's `operator_error`, not in the scanner or the vocabulary.

## ADR-051 — The GUI scrolls the whole window as one column

Status: Accepted

One centered column of bounded width inside a single vertical scroll area
wrapping the whole content — header, input section and result components — so
the layout behaves like one document. The result components flow directly in
the main column, and the truth table keeps its horizontal scroll while growing
vertically with the window scroll. Cost: the controls scroll out of view with
the results, so `Evaluar` is not always visible while reading a long table.

## ADR-052 — Reserved words get role-specific parser diagnostics

Status: Accepted

The reserved spellings (`no`, `y`, `o`, `entonces`, `sisolosi`, `V`, `F`) fail
differently from symbols, but after scanning `y` and `&` are the same kind, so
`Token` gains a `reserved_word: bool` set by the scanner — diagnostic
metadata, not part of the language or the normalized display. The parser uses
it to report a word connective in operand position as `… no puede usarse como
variable`, a word connective with no right operand as `… necesita una
expresión a su derecha`, `no` between propositions as the prefix negation, and
`V` / `F` in operator position as values. The message names every spelling of
the kind; symbol spellings keep the generic messages. The reserved words
themselves are unchanged (ADR-010).

## ADR-053 — File-mode headings carry the proposition's source line

Status: Accepted

File mode numbered its blocks with a bare `Proposición n:`, so a reader of a
long batch could not see which proposition each table belongs to without
counting lines back to the file.

Decision: the heading is `Proposición n: <proposición>`, with the proposition
exactly as the file line holds it, trimmed. The Orchestrator splits the
non-blank lines out of the source before the scanner consumes it (ADR-038
drops the text with its state) and carries the line in its results:
`Evaluation.proposition` for the table run and a new orchestrator-owned
`Formula { proposition, simplification }` for `--simplificar` — the Simplifier
never sees the source, so `Simplification` stays untouched. The pairing is
positional and lives next to the parser: a line with any non-blank character
always yields exactly one proposition and a blank line yields none (ADR-004).

Alternatives: show the normalized original formula (reuses printers, but is
not what the user wrote and needs the same plumbing with `--no-simplificar`);
pair lines and results in the CLI (duplicates the blank-line rule away from
the parser); keep the raw text in the parser (violates ADR-038's ownership).

Consequences: `Evaluation` and the new `Formula` each grow one `String`, and
every file-mode block names its proposition. Numbering and the blank-line
counter are unchanged, and headings never appear outside file mode. The
positional pairing is pinned by orchestrator tests.

## ADR-054 — The interactive session edits lines with rustyline

Status: Accepted

The session read whole lines from stdin, so the terminal's own line discipline
was the only editor: no caret movement, no history, and a mistyped proposition
had to be retyped. Everything else a line editor does — the Emacs keys, Ctrl-C,
Ctrl-D — comes from raw-terminal handling that plogic does not implement.

Decision: on a terminal the session reads its lines with `rustyline` 19,
built with `default-features = false`: `DefaultEditor` draws the `> ` prompt,
the arrows move the caret, Up/Down walk the session's history and Ctrl-D ends
the session. History is `MemHistory`, in memory for the session only; nothing
is written to disk, and blank lines are not recorded. Ctrl-C cancels the line
and shows a fresh prompt — the session stays sticky, so only `exit`, `salir`
and Ctrl-D end it. When stdin is not a terminal (a pipe, a redirect, the test
harness), the plain reader runs instead: rustyline's own non-tty fallback
writes no prompt, which would silently change the output every session test
asserts.

Alternatives: `reedline` (better unicode and paste handling, heavier tree for
what this needs); a hand-rolled raw mode over termios and escape sequences
(weeks of terminal edge cases); accepting rustyline's non-tty fallback (loses
the prompt on stdout and breaks the piped tests); file-based history
(persistent state the request did not ask for).

Consequences: `rustyline` joins the dependency list, adding eight small leaf
crates and no `home` or `radix_trie` because the default features are off. The
session splits into `run_edited`, `run_piped` and the shared `session_line`, so
the piped behavior is exactly the earlier one and its tests are untouched. The
terminal path cannot run in the test harness; it is checked on a
pseudo-terminal, and `design/testing.md` records what that check covers.

## ADR-055 — The GUI toolbar has a delete button

Status: Accepted

The toolbar could insert every symbol and the next variable, but a mistyped
character had to be removed with the keyboard, which means finding and
selecting it first. The field does not expose a delete key of its own, and the
caret is already tracked for insertion (ADR-052's caret retention).

Decision: a `borrar` button sits between `variable` and `Limpiar` and deletes
before the remembered caret. With a selection it deletes the selected text
whole; otherwise it deletes the character just before the caret, counted in
characters so a formal symbol is one deletion. A caret at the start deletes
nothing, and with no known caret it deletes the last character — the mirror of
the insert's append. The caret lands where the deletion started and keeps the
focus. Insertion and deletion share `cursor_bounds` and `set_caret`, so both
read the caret and restore it the same way.

Alternatives: a `⌫` glyph as the label (the mockup's font fallback does not
cover it reliably); deleting at the end regardless of the caret (ignores the
caret the insert buttons respect); deleting only the last character, never a
selection (inconsistent with the insert buttons, which replace selections).

Consequences: the GUI tests cover the deletion cases directly through the pure
helpers, since the window cannot run headlessly; the layout is checked by eye.
`Limpiar` keeps its meaning: it clears the whole field, not one character.
