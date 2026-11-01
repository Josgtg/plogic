# Plogic - Decision log

One record per settled design decision. A record states what was decided,
what it replaced, and what it costs. The design documents say how each
decision is carried out; this file says why, and what the alternatives were.

Records are appended, never rewritten: a decision that changes gets a new
record that supersedes the old one. `Status` is `Accepted` or `Superseded by
ADR-nnn`.

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

## ADR-001 — Functional, static style instead of the book's classes

Status: Accepted

Context: Crafting Interpreters builds a `Scanner` instance and a `Parser`
instance that mutate their own fields and report through a static
`Lox.error` sink. Plogic follows the book's algorithms, not its state model.

Decision: Every stage is a static entry point over immutable values:
`Scanner.scan(source)`, `Parser.parse(tokens)`, `Simplifier.simplify(expr)`,
`Interpreter.interpret(expr)`, `Orchestrator.evaluate(source, simplify)`.
Intermediate state (`ScanState`, `ParseState`) is threaded through helpers and
returned anew; nothing is mutated and nothing is constructed.

Alternatives: the book's mutable instances; a trait-object pipeline of stages.

Consequences: no shared mutable state anywhere, stages are trivially
testable one at a time, and the diagrams show no constructors. The cost is
that helpers must pass state explicitly, which makes signatures longer.

## ADR-002 — Errors as values, with stage gates

Status: Accepted

Context: User mistakes are normal input. The book throws for scan/parse
errors and keeps a mutable `hadError` flag.

Decision: Errors are values. Every fallible operation returns `Result<T>`;
failure is `Err(List<Error>)` and never an empty value or a flag. Recovery is
maximal inside a stage (the scanner skips a bad character and continues, the
parser synchronizes to the next line and continues), and a stage that recorded
any error discards its partial value. Between stages the pipeline stops at the
first failing stage, so one `Err` list holds errors of exactly one kind.

Alternatives: a mutable error sink as in the book; collecting errors from
every stage into one list; partial results alongside errors.

Consequences: a run with any error produces no truth tables — one bad line in
a long file yields nothing (deliberate: the product description promises
exactly this). All errors of the failing stage are still reported.

## ADR-003 — `TokenType` as a payload enum with one spelling map

Status: Accepted

Context: Connectives can be written as symbols or as Spanish words, and
literals in four spellings. The set of spellings has to live in exactly one
place or the scanner and the documentation will drift.

Decision: `TokenType` is a Rust-style enum with payloads (`Literal(bool)`,
`Identifier(str)`), and `TokenType.map(lexeme): Result<TokenType>` is the only
place where spellings are defined. The scanner asks `map` for every symbol and
word; a word it does not know becomes an identifier, anything else is a
`ScanError`. The token does not keep its lexeme.

Alternatives: a lexeme table in the scanner; a separate keyword table; keeping
the source spelling on every token for display.

Consequences: one table to keep in sync with the product description, and free
mixing of spellings (`p o q`, `p ∨ q`, `p | q` are the same tokens). The token
stays small because the display is normalized (ADR-009).

## ADR-004 — Line-oriented input: `Newline` ends a proposition

Status: Accepted

Context: A script holds several propositions, and the syntax has no statement
terminator of its own.

Decision: The scanner emits `Newline` instead of swallowing line breaks, and a
proposition ends at `Newline` or `Eof`. Blank lines are empty propositions and
are skipped, so a single proposition needs no trailing newline. Multi-line
propositions are out of scope.

Alternatives: a `;` terminator; treating the whole input as one proposition;
implicit concatenation of lines.

Consequences: one proposition per line is a hard rule of the language, which
makes file mode and error positions simple, and rules out wrapping long
propositions.

## ADR-005 — Precedence ladder and associativity

Status: Accepted

Context: `p | q & s -> t` must parse the way a logician reads it, and
unparenthesized chains must not silently change meaning.

Decision: Five levels, lowest to highest: biconditional → implication →
disjunction → conjunction → negation → primary, one parser method per level:
`biconditional → implication ( ↔ implication )*`, `implication → disjunction
( → implication )?`, `disjunction → conjunction ( ∨ conjunction )*`,
`conjunction → unary ( ∧ unary )*`, `unary → ¬ unary | primary`. Implication is
right-associative (`p → q → r` is `p → (q → r)`); every other binary level is
left-associative.

Alternatives: C-style precedence (`!` `&` `|` `->` all with the book's ladder);
right-associative biconditional; fully parenthesized input only.

Consequences: the ladder matches the connective family grouping and the book's
ladder depth, so the parser stays one-method-per-level. Mixed chains still need
parentheses to be readable, which the display makes explicit.

## ADR-006 — Batch parse with panic-mode synchronization

Status: Accepted

Context: A file holds many propositions, and reporting only the first error
would make fixing a file a sequence of runs.

Decision: The parser owns the statement loop (`parse(tokens):
Result<List<Expression>>`, one expression per proposition). On a parse error it
records the `ParseError`, runs `synchronize` — a pure `ParseState → ParseState`
that skips to just after the next `Newline` or to `Eof` — and keeps parsing.

Alternatives: stop at the first error; a separate statement parser per line;
error recovery at expression level only.

Consequences: every bad proposition in a file is reported in one run, at the
cost of a whole-run stage gate (ADR-002): no tables are printed when any line
fails.

## ADR-007 — Simplification by oriented rewrites to a fixed point

Status: Accepted

Context: Simplification must be an equivalence transformation — same truth
table, fewer connectives — and must terminate without an arbitrary iteration
cap.

Decision: Each law is fixed to one direction and applied as a local rewrite,
bottom-up, re-canonicalizing and re-simplifying after each rewrite until a
fixed point. Termination is argued with a weight `W` that every reduction rule
strictly lowers. Laws that make the formula bigger (distributivity read
left-to-right) are tried only in an optional expansion phase, and a candidate
is kept only if `W` strictly drops.

Alternatives: a fixed normal form (CNF/ANF) computed in one pass; a rewrite
engine with an iteration cap; no simplification.

Consequences: no normal form is guaranteed, so the simplified formula is one of
many equivalents — the classification therefore comes from the truth table,
not from the reduced formula. The expansion phase costs extra scans but cannot
loop.

## ADR-008 — Canonicalization (flatten + sort) inside the Simplifier

Status: Accepted

Context: `q & (p & r)` and `r & (q & p)` are the same proposition by the
associative and commutative laws, but as trees they differ, so "do these
operands match?" — the question idempotence, absorption and factoring all ask —
is not a plain equality check.

Decision: Before applying laws, `&` and `|` chains are flattened into one
operand list and sorted in a total, structural order (constants, variables,
negations, `&` chains, `|` chains, conditionals last), compared by kind first
and then within the kind. The lists live inside the Simplifier only: the AST
keeps binary nodes, and a chain is rebuilt as left-nested `Binary` nodes on
return.

Alternatives: matching modulo associativity and commutativity during each rule;
a canonical form in the AST itself; no sorting (order-dependent matching).

Consequences: matching becomes structural equality and output order becomes
deterministic. The cost is a sort on every rewrite, and a canonical order that
has to be defined precisely enough to be total.

## ADR-009 — One normalized display form

Status: Accepted

Context: The user may write `!p and not q ∧ V` and must see one consistent
formula back. Keeping the source spelling would preserve intent but make output
depend on how the user typed.

Decision: Every formula is displayed through `InfixPrinter` in one normalized
form: `¬`, `∧`, `∨`, `→`, `↔`, literals `V` / `F`, operand order as written and
only the parentheses precedence requires. The formal symbols are also accepted
as input. The simplified formula is shown in the Simplifier's canonical order.
Design documents keep writing `!`, `&`, `|`, `->`, `<->` because they are
easier to type.

Alternatives: display exactly as written (needs the lexeme on every token);
display with C-style symbols; display only the simplified formula.

Consequences: no lexeme plumbing; the truth-table label and the `Simplificada:`
line can differ in operand order, which is expected. Non-UTF-8 terminals show
mojibake — no ASCII fallback exists.

## ADR-010 — Four literal spellings; short words reserved

Status: Accepted

Context: The examples write "false" as `0`, and the product description also
allows `V` / `F`.

Decision: Literals are written `0`, `1`, `F` or `V`; displays always use `V` /
`F`. The words that spell connectives (`no`, `y`, `o`, `entonces`, `sisolosi`)
and literals (`V`, `F`) are reserved and can never be identifiers.

Alternatives: only `V` / `F`; only `0` / `1`; Spanish words `verdadero` /
`falso`.

Consequences: single-letter identifiers `y` and `o` (and `V`, `F`) are
impossible — and, asymmetrically, lowercase `v` and `f` are valid variables.
Accepted because one-letter variable names that clash with reserved words are
confusing anyway.

## ADR-011 — Size limits: 4096-row cap, 1024-row file redirect

Status: Accepted

Context: A truth table has 2^n rows. Twelve variables is 4096 rows; thirteen is
8192, and a long run would flood the console.

Decision: Two separate limits. More than 4096 rows is refused before any row is
built (`Err(EvalError("resultado muy largo: más de 4096 filas"))`), so at most
12 variables are accepted and the run fails like any other stage. More than
1024 rows (2^10) in any table of a batch run sends the run's output to
`plogic-output.txt`, written by the UI, with a notification.

Alternatives: one single limit; no cap and streamed output; a configurable
limit.

Consequences: the error is size-related, so `EvalError` carries no position and
is the only error after parsing. The two numbers are different on purpose —
1024 is where reading becomes impractical, 4096 is where the run refuses — but
they are easy to confuse.

## ADR-012 — Spanish UI, messages and flags

Status: Accepted

Context: The tool is for Spanish-speaking users, including students.

Decision: Everything the user reads is Spanish: prompts, table headers,
classification lines, error messages and flag names (`-p`, `-a`, `-i`, `-s`,
`--proposicion`, `--archivo`, `--interactivo`, `--salida`,
`--no-simplificar`). Message style is lower-case,
saying what was expected.

Alternatives: English UI only; bilingual output; an i18n layer.

Consequences: implementation strings are part of the product and must be
treated as user-facing text, not debug output. A second language is out of
scope.

## ADR-013 — CLI surface: bare argument, `-p`, `-a`, `-i`, `-s`

Status: Accepted

Context: Three ways to feed propositions plus two modifiers, and the flags were
initially inconsistent (`--eval` in one language, `--archivo` in another).

Decision: `-p` / `--proposicion <proposition>` (and a bare argument with no
flag) for one proposition, `-a` / `--archivo <path>` for a file with one
proposition per line, `-i` / `--interactivo` for a session. Modifiers:
`--no-simplificar` and `-s` / `--salida <path>`. No argument at all, more than
one mode flag, an unknown flag, or `-s` with `-i` is a usage error (exit 2).

Alternatives: subcommands (`plogic eval`, `plogic file`); `--eval` in English;
positional file path detection.

Consequences: a bare argument is convenient but ambiguous with a mistyped flag
(`plogic -x` is a usage error, `plogic -\>` would be a proposition). The
combination `-s` + `-i` is rejected because a session has many tables and no
single file.

## ADR-014 — I/O errors live outside the `Error` model

Status: Accepted

Context: Unreadable files, unwritable output paths and terminal problems are
not proposition mistakes and have no line or column.

Decision: The `Error` enum covers the language only (`ScanError`, `ParseError`,
`EvalError`). I/O problems belong to the CLI, which reports them directly. An
input file with no propositions (empty, or only blank lines) is reported as
`el archivo no contiene proposiciones` with exit 1.

Alternatives: an `IoError` variant; mapping I/O failures onto `ScanError` at
line 1.

Consequences: the pure part of the pipeline never formats I/O text, but the CLI
owns two reporting paths (formatted errors via `Orchestrator.report`, I/O
messages directly) and must keep their wording consistent.

## ADR-015 — Two printers: `ASTPrinter` and `InfixPrinter`

Status: Accepted

Context: The book's AST printer produces prefix s-expressions for verification;
users need the formula back in infix.

Decision: `ASTPrinter` renders parenthesized prefix notation and is a
verification tool; `InfixPrinter` renders the normalized infix form (ADR-009)
and is the only printer used for user-facing formulas. Both are static,
stateless visitors over `Expression`.

Alternatives: one printer with a mode flag; printing the tokens instead of the
AST; no prefix printer.

Consequences: two traversals to keep in step, in exchange for a readable output
and a debugging aid whose parentheses never depend on precedence.

## ADR-016 — 1-based positions, columns counted in characters

Status: Accepted

Context: Token positions feed every diagnostic (`línea L, columna C: …`). The
book tracks only `line`, starting at 1, and has no column at all; scanners in
other tools often use 0-based offsets.

Decision: `line` and `col` are 1-based and point at the first character of the
token. `col` is the number of characters between the last line break and the
token's start: a UTF-8 symbol (`¬`, `∧`, `∨`, `→`, `↔`) counts as one position,
and so does a tab. The column is derived from the source and the token's start
offset; `ScanState` stores no column counter.

Alternatives: 0-based columns (programmer convention, off by one against every
editor); byte offsets (wrong for the formal symbols); a column counter in the
state (bookkeeping the design avoids).

Consequences: reported positions match what the user sees in an editor gutter.
The scanner must advance character by character, which it already does to
recognize the formal symbols, and a tab makes the reported column differ from
the visual one.

## ADR-017 — Default tabulates the original; `--tabla-simplificada` tabulates the simplified

Status: Accepted

Context: The Simplifier sorts operands and can remove variables. Building the
truth table from the simplified expression by default would reorder the
columns, drop variables that the user wrote, and check the 4096-row limit
against the wrong variable set. The product promises one column per distinct
variable in order of first appearance. At the same time, the smaller table of
the simplified formula is useful on its own.

Decision: By default the Interpreter receives the original expression.
Variables, rows, the 4096-row check and the result-column label all come from
it; when simplification ran, its formula is also shown in a `Simplificada:`
line. With `--tabla-simplificada` (long-only, no short alias), the Interpreter
receives the simplified expression: variables, rows, the check and the label
come from it, the `Simplificada:` line still prints and repeats the label, and
the classification is still printed. `--tabla-simplificada` combined with
`--no-simplificar` is a usage error in every mode.

Alternatives: always tabulate the simplified expression (wrong columns for the
proposition the user wrote); pass the original variable list to the Interpreter
(couples it to a display decision and still loses variables with the flag); show
two tables (more output than asked for).

Consequences: default output always shows every variable the user wrote, in
first-appearance order. The flag can shrink the table to the simplified
variables, including the zero-variable constant case, and can make a
13-variable original acceptable when its simplified form has at most 12
variables. The Interpreter stays unaware of original versus simplified: it
evaluates the expression it receives.

## ADR-018 — Erase `Grouping` before simplification

Status: Accepted

Context: The parser keeps `Grouping` nodes so the AST and `InfixPrinter` can
preserve the parentheses the user wrote. The Simplifier's weight, canonical
order and law matching were defined on the other variants only; with `Grouping`
present, `W` and the termination argument were undefined.

Decision: `Simplifier.simplify` first strips every `Grouping`, recursively,
before canonicalization or reduction. The rest of the Simplifier never sees a
`Grouping`, and its output contains none. The parser, the AST and both printers
keep `Grouping` for the original expression and its label.

Alternatives: define `W(Grouping(p)) = W(p)` and make every traversal see
through it; give `Grouping` a canonical rank; remove `Grouping` from the AST.

Consequences: simplification is total again and the weight table stays small.
Parentheses survive in the label and in `--no-simplificar` output; the
simplified formula relies on `InfixPrinter`'s precedence-aware parentheses.
Nested grouping is erased in one pass.

## ADR-019 — Stage results carry `List<Error>`; helpers carry one error

Status: Accepted

Context: The documents disagreed on the error payload: parser rules return one
`Error`, the Interpreter and tests wrote `Err(EvalError(...))`, and AGENTS and
ADR-002 said every fallible operation carries `List<Error>`.

Decision: Public stage entry points (`Scanner.scan`, `Parser.parse`,
`Interpreter.interpret`, `Orchestrator.evaluate`) return
`Result<T, List<Error>>`; `Err` always carries a non-empty list from the stage
that failed. Internal helpers return `Result<T, Error>`; parser rules are the
main case. `TokenType.map` returns `Result<TokenType, str>` because it has no
position: the scanner turns the message into `ScanError(line, col, message)` at
the token start. A stage wraps a helper error into a one-element list before
returning. `Simplifier.simplify` stays total and returns `Expression`.

Alternatives: lists everywhere (noise inside rules); one error everywhere
(stage gates lose multi-error reporting); two named aliases.

Consequences: `Err` at a stage boundary is always a list, including evaluation
failures written as `Err([EvalError(...)])`. The scanner and parser can still
recover internally and collect several errors before returning. Tests assert
one representation per boundary.

## ADR-020 — Immutable `TruthTable` with `with_display`

Status: Accepted

Context: The Interpreter creates the table with variables and rows, while the
Orchestrator knows the original and simplified display text. The documents said
the Orchestrator "fills" the display text, which implied mutation and had no
factory.

Decision: The Interpreter creates a `TruthTable` with variables and rows; the
display fields start empty. `TruthTable.with_display(label: str, simplified:
str?): TruthTable` returns a new table with the display fields set. The
Orchestrator calls it once per table. There are no setters and no mutation.

Alternatives: the Interpreter receives the display strings (couples it to the
CLI); the Orchestrator constructs the table (duplicates the Interpreter's
invariants); mutable fields (violates purity).

Consequences: the table is immutable end to end. `Interpreter ..> TruthTable :
<<create>>` and `Orchestrator ..> TruthTable : with_display` remain accurate.

## ADR-021 — Interactive modifiers and always-on classification

Status: Accepted

Context: `ui.md` excluded `-i` from the modifier flags while ADR-013 and the
README allowed `--no-simplificar -i`. The classification line was suppressed
under `--no-simplificar` without a recorded decision, and ADR-012's lower-case
rule did not say whether labels such as `Simplificada:` were included.

Decision: `--no-simplificar` and `--tabla-simplificada` each apply in
interactive mode; using them together is a usage error there too. The
classification (`tautología`, `contradicción`, `contingencia`) is always
printed, from the table's result column, in every mode and with or without
simplification. The labels `Simplificada:` and `Esta proposición...` keep
their capital initial; the lower-case rule of ADR-012 applies to error messages
and prompts.

Alternatives: reject modifiers in interactive mode; suppress the
classification with `--no-simplificar`; lower-case the labels.

Consequences: interactive behavior matches single-proposition and file mode,
and the classification is independent of whether a simplified line was shown.

## ADR-022 — Strict CLI usage, I/O failures exit 1, redirect on every non-interactive run

Status: Accepted

Context: The CLI left several edge cases and Spanish strings unspecified, the
README described the automatic redirect as applying to any run while `ui.md`
called it batch-only, and no exit code was assigned to I/O failures.

Decision: The CLI is strict. No arguments, a modifier-only invocation, a
missing flag value, an extra positional argument, more than one mode, `-s` with
`-i`, `--tabla-simplificada` with `--no-simplificar`, and an empty `-p ''` are
usage errors: a usage message on stderr and exit code 2. `-s` writes only on
success. Reading or writing a file fails with exit code 1 and a Spanish
message; the `Error` model is not used for I/O. The automatic
`plogic-output.txt` redirect applies to any non-interactive run (single
proposition or file) when any table has more than 1024 rows; `-s` wins;
interactive sessions never redirect.

The exact strings are:

- usage: `uso: plogic [<proposición> | -p <proposición> | -a <archivo> | -i] [--no-simplificar | --tabla-simplificada] [-s <archivo>]`
- `falta el valor de '<flag>'`
- `opción desconocida: '<flag>'`
- `solo se puede usar un modo a la vez`
- `argumento inesperado: '<arg>'`
- `'-s' no se puede usar con '--interactivo'`
- `'--tabla-simplificada' no se puede usar con '--no-simplificar'`
- `no se proporcionó ninguna proposición`
- `no se pudo leer el archivo '<path>': <detalle>`
- `no se pudo escribir el archivo '<path>': <detalle>`
- `el archivo no contiene proposiciones`
- redirect notification: `la salida se escribió en plogic-output.txt` (stderr)

Alternatives: permissive parsing; exit 0 on I/O failure; redirect only in file
mode; only the explicit `-s` destination.

Consequences: every CLI edge case has one outcome and one message. `-s` with
`--tabla-simplificada` and `-s` with `--no-simplificar` remain valid, and `-s`
wins over the automatic file name.

## ADR-023 — Simplification lowers weight, not necessarily connective count

Status: Accepted

Context: `product_description.md` and ADR-007's context said the reduced
formula has fewer connectives, but the law table includes De Morgan and the
biconditional expansion, which add connectives while lowering `W`.

Decision: The guarantee is the same truth table and a strictly lower weight
whenever a rewrite applies. The simplified formula need not have fewer
connectives. `--no-simplificar` keeps the original expression untouched.

Alternatives: require a connective-count decrease (blocks useful laws); count
connectives in `W` (gives the wrong ordering).

Consequences: documents and tests assert equivalence and decreasing weight,
not a smaller connective count.

## ADR-024 — Dash-prefixed arguments, `--`, and blank propositions

Status: Accepted

Context: Strict unknown-flag handling and bare propositions can conflict when
an argument begins with `-`. A single proposition can also arrive empty or
whitespace-only from a shell variable, and it has to be distinguished from a
genuine proposition that happens to contain spaces.

Decision: Every argument that begins with `-` is an option, unless it appears
after `--`. `--` ends option parsing: the next argument is the single bare
proposition even if it begins with `-`, and at most one positional argument is
allowed after it. Thus `plogic ->` is `opción desconocida: '->'` and exits 2,
while `plogic -- '->'` passes `->` to the scanner. A single proposition, bare
or from `-p`, is trimmed; if nothing remains, the CLI prints
`no se proporcionó ninguna proposición` and exits 2. Only the emptiness check
uses the trimmed value; the scanner receives the original text. A
whitespace-only line in interactive mode is still a harmless blank line.

Alternatives: treat an unknown dash-prefixed argument as a bare proposition
(breaks option parsing); require `--` before every proposition (needless);
accept a whitespace-only proposition as an empty success (inconsistent with
the empty string).

Consequences: the CLI follows the option-handling convention users expect:
`--` is the escape hatch for a leading dash, and unknown options fail before
scanning. No valid proposition begins with `-` (negation is `!`, `¬` or `no`),
so strictness costs nothing in practice.

## ADR-025 — Parser rule failures carry the `ParseState`

Status: Accepted

Context: After a parse error, `parse` must call `synchronize` from the exact
failure point. A rule that returns only an `Error` leaves the caller with the
state from before the rule ran, which may be several tokens behind.

Decision: Every parser rule returns
`Result<(Expression, ParseState), (Error, ParseState)>`; `consume` returns
`Result<(Token, ParseState), (Error, ParseState)>`. On failure the second
component is the immutable state at the failing token. `parse` records the
error, calls `synchronize` on that state, and continues with the next
proposition. The public stage result stays `Result<List<Expression>,
List<Error>>` (ADR-019). The `error(token, message)` helper still builds the
`ParseError`; the rule pairs it with the state it already holds.

Alternatives: synchronize from the state at the start of the proposition
(simpler, but discards tokens already consumed inside the rule); store an
absolute token index in the error (duplicates `current`); mutate a parser
instance (rejected by ADR-001).

Consequences: Rule signatures are heavier, and every failure path must return
the state. Recovery resumes exactly where parsing failed, so error positions
and the following proposition match the source.

## ADR-026 — `table_simplified` is ignored when `simplify` is false

Status: Accepted

Context: `Orchestrator.evaluate(source, simplify, table_simplified)` is public
and pure, but the CLI rejects `--tabla-simplificada` with `--no-simplificar`.
A direct caller can still pass `simplify = false, table_simplified = true`, and
the design does not define what happens; step 8 would look for a simplified
expression that step 6 never produced.

Decision: When `simplify` is false, `table_simplified` has no effect: the
original expression is tabulated and no simplified text is produced.
`evaluate` is total over its four argument combinations. The CLI still rejects
the combination as a usage error, so users never reach it.

Alternatives: return `Err([EvalError(...)])` (overloads the size-limit error
with a usage problem); treat `table_simplified` as implying simplification
(the caller's `simplify = false` is silently overridden); make the state
unrepresentable with a two-variant type (more API churn for one internal
caller).

Consequences: Direct callers get the documented default instead of a panic or
an ambiguous result. Tests assert that `simplify = false, table_simplified =
true` produces the same table as `simplify = false, table_simplified = false`.

## ADR-027 — Fixed precedence for usage errors

Status: Accepted

Context: Several usage errors can apply at once, for example
`-i -s out.txt --tabla-simplificada --no-simplificar`. ADR-022 requires one
outcome and one message per CLI edge case but does not say which message wins.

Decision: The CLI reports exactly one usage message, the first that applies in
this order: unknown flag, missing flag value, more than one mode, extra
positional argument, no proposition, `--tabla-simplificada` with
`--no-simplificar`, `-s` with `--interactivo`. The exit code is always 2.

Alternatives: report the first problem encountered while scanning arguments
left to right (the winner depends on argument order); report every usage error
(more informative, but contradicts the one-message presentation and makes
tests brittle).

Consequences: The behavior is deterministic and testable with a table of
argument vectors. A new usage check must be placed in the order and the order
documented.

## ADR-028 — `--` is an optional trailing separator

Status: Accepted

Context: ADR-024 makes `--` end option parsing and allows at most one
positional after it, but does not say whether a trailing `--` is an error or
whether `--` can be consumed as a flag value.

Decision: `--` is always a separator, never an option value. It may be
trailing when a proposition already exists: `plogic -p p --` is valid.
`plogic --` alone has no proposition and is a usage error. `plogic -p -- p`
fails with `falta el valor de '-p'`, because `--` is not the value of `-p`.
Two positionals after `--` are an extra-positional usage error.

Alternatives: require a positional after `--` (rejects a harmless trailing
separator); allow `--` only before a bare proposition (inconsistent with
ordinary option parsing).

Consequences: The rule matches conventional option parsing and gives one
deterministic reading. A proposition that is literally `--` cannot be passed;
the language has no such token, so nothing is lost.

## ADR-029 — Blank lines separate the presented sections

Status: Accepted

Context: The README example prints a blank line after the `Simplificada:`
line, after the table and before the classification, but ui.md did not define
separators, so two implementations could both claim to follow the design.

Decision: `present` prints one blank line between the printed items: after
the simplified line, after the table, and before the classification. With
`--no-simplificar` only the separator between the table and the classification
remains.

Alternatives: no blank lines (denser, but the classification runs into the
table); a blank line only before the classification (the simplified line sits
directly above the table).

Consequences: Output is stable for golden tests and matches the README.
Presentation code emits separators explicitly, including when the simplified
line is absent.

## ADR-030 — Help and version flags

Status: Accepted

Context: The CLI parser offers `-h`/`--help` and `-V`/`--version` by default,
but the flag surface in ADR-013 and the exact usage line in ADR-022 did not
include them. Leaving the defaults on would add undocumented behavior; turning
them off would remove the help every CLI user expects.

Decision: The CLI supports `-h`/`--help` and `-V`/`--version`. Both write to
stdout and exit 0. They are not modes and do not combine with a proposition.
When either appears as an option (before `--`), it is handled before any usage
validation; if both appear, help wins. `--help` prints the Spanish help block
defined in `ui.md`; `--version` prints exactly `plogic 0.1.0`, taken from the
Cargo package name and version. The usage line gains
`[-h | --help] [-V | --version]`.

Alternatives: disable both and keep the strict surface (no discoverable help);
help only (no version output); keep the parser's defaults (English text,
conflicting with ADR-012 and the exact-string rule).

Consequences: `plogic --help --bad` exits 0 instead of reporting an unknown
flag, and an invalid invocation with `--help` shows help rather than the error.
Tests assert the exact help and version output and the precedence over usage
errors.

## ADR-031 — Repeated flags reuse the existing usage messages

Status: Accepted

Context: ADR-022 and ADR-027 give every usage error one message and one place
in the precedence, but neither says what a flag passed twice does
(`plogic -p p -p q`, `-i -i`, `-s a -s b`). The command-line parser rejects a
repeated flag as a conflict, so the outcome has to be defined rather than
inherited.

Decision: A repeated flag is a usage error with exit code 2 and no new
strings. Repeating a mode flag (`-p`, `-a`, `-i`) reports
`solo se puede usar un modo a la vez`; repeating `-s`, `--no-simplificar` or
`--tabla-simplificada` reports `argumento inesperado: '<flag>'`, with the flag
as the user typed it. Each repetition takes the place of the message it reuses
in the ADR-027 order: a repeated mode flag counts as more than one mode, and
any other repetition counts as an extra positional argument.

Alternatives: a new message such as `'<flag>' no se puede usar más de una vez`
(clearer, but adds a string plus its precedence and its tests); letting the
last occurrence win (permissive, contradicts ADR-022).

Consequences: The strict CLI keeps one message per case and one documented
order. A repeated mode flag is indistinguishable from two different modes,
which is acceptable because both mean the same thing: more than one mode.

## ADR-032 — Compact usage line and generated help screen

Status: Accepted

Context: ADR-022 fixed a usage line that enumerated every mode and modifier,
and ADR-030 documented a hand-written help block. Both repeated the argument
definitions: adding or renaming a flag meant editing several documents, and the
enumerated line was long enough to wrap on a normal terminal. The command-line
parser already knows the arguments and can render them.

Decision: The usage line is the compact
`uso: plogic [opciones] [proposición]`. The help screen is generated from the
argument definitions, one line per argument, grouped in `argumentos:` (the
positional) and `opciones:` (everything else, the help and version flags last).
Its fixed words stay Spanish and every description is lower-case, like the
error messages. Error messages remain plogic's own exact Spanish strings with
the ADR-022/ADR-027/ADR-031 rules; the parser's error text is never shown.

Alternatives: keep the enumerated usage line (long, and repeated in every
document); adopt the parser's English help and error text (breaks ADR-012);
keep the hand-written help block (a second list to keep in sync).

Consequences: A new argument appears in the help as soon as it is declared, and
the usage line fits on one line. ADR-022's usage string is replaced by the
compact one; the rest of ADR-022, and ADR-027/ADR-031, still hold. ADR-030's
help block and its precedence rule are superseded by ADR-032 and ADR-033.

## ADR-033 — The parser handles help and version, in argument order

Status: Accepted

Context: ADR-030 made plogic scan the arguments itself so that `-h`/`--help`
and `-V`/`--version` win over every other argument and every usage error, and
then print a hand-written help block. ADR-032 moved the help screen to the
command-line parser, which also knows the two flags and can print them: it
does so when it reaches them, in the order they appear. Keeping ADR-030's
precedence therefore means a second pass over the tokens just to pre-empt an
error the parser has not raised yet.

Decision: `-h`/`--help` and `-V`/`--version` are the parser's. It prints them
to stdout and exits 0, and it does so when it reaches them: an error raised by
an earlier argument wins, so `plogic -x --help` and `plogic -p --help` are
usage errors (exit 2), while `plogic --help -x`, `plogic -hV` and
`plogic --help --version` print the help. `plogic -V --help` prints the
version. `--` still ends option parsing, so `plogic -- --help` evaluates the
proposition `--help`. The parsing entry point returns the validated run only;
the informational flags never reach the caller.

Alternatives: keep the pre-scan (ADR-030's rule, at the cost of a hand-written
pass over the tokens that does not exist for any other flag); let the parser
handle them but keep the pre-scan for help only (one more rule to document).

Consequences: One flag mechanism, no duplicate help block, and the behavior
matches every other command-line tool that uses the same parser. The precedence
in ADR-030 no longer applies to the two informational flags; ADR-022/ADR-024/
ADR-027/ADR-028/ADR-031 hold for every usage error. Tests assert the fixed
order with a table of argument vectors.

## ADR-034 — Usage errors print the message and the full help screen

Status: Accepted

Context: ADR-022/ADR-027 gave every usage error the usage line plus one
message. Since ADR-032 the help screen is generated and complete, so a user who
misses an option had to run `--help` separately; the one-line usage string in an
error repeats a fragment of the screen without saying what the options are.

Decision: A usage error prints, on stderr and in this order, the specific
message, a blank line, and the full help screen from `ui.md`, then exits 2. The
screen is the same one `-h`/`--help` prints, generated by the parser from the
argument definitions; there is no second copy. The message stays first so the
cause is read before the reference.

Alternatives: keep the usage line plus the message (short, but no discoverable
options); print the help first and the message last (the cause ends up at the
bottom of a screen); send the help to stdout and the message to stderr (one
error split across two streams, and a redirect keeps half of it).

Consequences: Usage errors grow to a full screen, which is acceptable because
they end the run. Tests assert the exact layout: message, blank line, help
screen, exit 2.

## ADR-035 — Errors are bright red on a terminal

Status: Accepted

Context: Coloring was scoped to the `V` and `F` cells of the truth table, and
the presentation rule said errors stay plain. An error that ends the run, and
in a file mode the run of many propositions, is the one line the user has to
find among the tables; plain text gives it no more weight than a formula.

Decision: The message of an error is painted bright red. For a usage error that
is the cause line; the blank line and the help screen below it stay plain, so
the reference reads normally. The paint is a presentation detail of the CLI:
it applies only when the stream the error goes to (`stderr`) is a terminal, and
never when `NO_COLOR` is set; a redirected error is plain text, and so is
everything written to a file. The same treatment applies to the scan, parse and
evaluation errors the Orchestrator reports, so every error looks alike.

Alternatives: color the whole help screen with the message (loud, and the option
list becomes harder to read); color always (escape codes in logs and shell
captures); leave errors plain (the design's earlier rule, but the cause is
visually indistinguishable from the output around it).

Consequences: `colored` now covers errors as well as the table cells, and the
choice depends on the destination stream rather than on the program. Tests
assert the layout with and without the escapes; the terminal check itself is
not automated.

## ADR-036 — Token vocabulary details settled in implementation

Status: Accepted

Context: `tokens.md` fixes the vocabulary, the spellings and the `map` contract
but leaves three details open: who composes the text of `map`'s `Err(message)`
("the message says what was expected"), what `to_string()` renders for the two
structural kinds, and how a token exposes its three fields.

Decision: `TokenType.map` composes the message from the lexeme's shape — a run
of digits gives `literal inválido '<lexeme>'`, anything else gives
`carácter inválido '<lexeme>'` — and the scanner records it unchanged. The
display spelling is `Display` on `TokenType`, with `Token` delegating to its
kind; `Newline` and `Eof` have no spelling and render as the empty string.
`Token` is plain data with public `kind`, `line` and `col` (`kind` because
`type` is a Rust keyword).

Alternatives: `map` returning a neutral message that the scanner rewrites into
the literal/character one (two places would know about lexeme shapes); Spanish
words such as `nueva línea` for the structural kinds (text the design never
promises); private fields with accessors (boilerplate for a record every stage
reads).

Consequences: the scanner has no message table of its own, the tokens that can
never appear in a formula print as nothing, and every later stage reads a token
by field. Tests assert the exact `map` messages and the display of every kind.

## ADR-037 — The error type displays as its report line

Status: Accepted

Context: Two documents described the same user-facing line from different
places: `error_handling.md` gave `Orchestrator.report` the format
`línea {line}, columna {col}: {message}`, while `AGENTS.md` said `thiserror`
derives `Display` for the typed `Error` enum. Left alone, the format would live
twice and could drift.

Decision: `Error` derives `thiserror::Error`. The two positioned variants print
`línea {line}, columna {col}: {message}`; `EvalError` prints its message alone.
That is exactly the report line, so `Orchestrator.report(error)` returns
`error.to_string()` and the Spanish wording lives in one place. The variant
names keep the `Error` suffix that clippy dislikes (`ScanError`, `ParseError`,
`EvalError`): each names the stage that produced it, and the allow is local to
the enum.

Alternatives: `Display` rendering the bare message and `report` composing the
prefix (two places know the format); no `Display` until the orchestrator lands
(`thiserror` unused, but the enum's text would still be composed later).

Consequences: the CLI prints `report`, which is the enum's `Display`; tests
assert the exact line for a positioned error and for a size error, and the
prefix is never duplicated.

## ADR-038 — The scanner owns the source; a lone CR is invalid

Status: Accepted

Context: `scanner.md` writes `ScanState { source, … }` and
`Scanner.scan(source)` without deciding whether the state borrows the text, and
in Rust that choice shows in every signature: a borrowed source puts a lifetime
parameter on the state and on each helper. Nothing after scanning needs the raw
text — the table is labelled from the parsed expression and each error carries
its own message — and the scanner is pure, so it cannot keep the text either.
The document also defined a line break as "LF, or CRLF counted as one" and said
nothing about a CR on its own.

Decision: `ScanState` owns the source and `Scanner.scan` takes it by value; the
state and every helper carry no lifetime, and the text is dropped with the
state. A carriage return not followed by LF is not a line break: it is a
`carácter inválido` error, so only LF and CRLF end a line. Alongside them, the
helpers follow idiomatic Rust without changing the algorithm: `match` becomes
`match_char` (a Rust keyword), `peek` and `peek_next` return an optional
character instead of the book's `\0` sentinel, `advance` returns the new state,
and `add_token` takes the kind. A `Newline` token carries the line it ends.

Alternatives: borrow the source (no copy, lifetimes on the state and helpers);
take `&str` and clone inside (ownership without the lifetime, one copy per
scan); take the string by value (chosen: no copy, no lifetime, and the caller
has nothing else to do with the text). For a lone CR: treat it as a break to
tolerate old Mac files, or reject it so the only line breaks are the ones
`product_description.md` defines (chosen).

Consequences: `scan` consumes its argument, so the orchestrator passes the text
along instead of lending it; tests call `scan(source.to_string())`. A file with
CR line endings fails on its first line rather than being silently accepted.

## ADR-039 — An underscore continues an identifier but never starts one

Status: Accepted

Context: `product_description.md`, `tokens.md` and `scanner.md` defined an
identifier as an ASCII letter followed by ASCII letters or digits, so a name
such as `p_1` was a `carácter inválido '_'` error and only `p1` could be
written. Longer propositions are easier to read when a name can group its
parts, and `_` is the conventional separator for that.

Decision: an underscore is part of an identifier but cannot start one. A word
begins at an ASCII letter and continues over ASCII letters, digits and
underscores, so `p_1`, `a_b` and `p__q` are identifiers while `_p` and a bare
`_` stay `carácter inválido '_'` errors. The reserved spellings are unaffected:
`V`/`F`/`no`/`y`/`o`/`entonces`/`sisolosi` are still matched whole by `map`,
and no spelling contains an underscore.

Alternatives: allow `_` as the first character too (`_p` and `_` become
variables); restrict it to a single separator between letter/digit runs;
reject underscores and keep `p1` as the only form. Allowing it to start a name
would make `_` a one-character variable and has no payoff for readability.

Consequences: the scanner's word predicate is `is_ascii_alphanumeric() || c ==
'_'`; the start-of-word condition is unchanged, so the character-dispatch
match still treats `_` as an invalid character. The error-message rule of
ADR-036 is untouched — `_` in a non-word position is still
`carácter inválido '_'`.

## ADR-040 — AST details: public fields, equality without positions, grouping in the printers

Status: Accepted

Context: ADR-015 fixes two printers over an `Expression` tree whose nodes
carry the operator `Token` (for its spelling), and `parser.md` shows the
nodes. Three details were left open. First, whether the node fields are
reachable from other modules, as the token fields are (ADR-036). Second, what
equality means for a tree: the token carries `line` and `col`, but the
Simplifier compares operand lists to decide whether two subformulas are the
same (idempotence, absorption, factoring) and its canonical order is
structural, so a position in the comparison would be a bug waiting to happen.
Third, how `ASTPrinter` renders a `Grouping`, which `InfixPrinter` prints
transparently.

Decision: The node fields are public, like the token fields, so the parser and
the stages pattern-match and build nodes without constructors. `Expression`
derives `Debug` and `Clone` and implements `PartialEq`/`Eq` by hand: it
compares the shape and each operator's `TokenType`, never the operator's
position. `ASTPrinter` prints a `Grouping` as `(group …)`, as the book's
printer does, so the verification output shows exactly the tree the parser
built, redundant parentheses included. `InfixPrinter` keeps its
associativity-aware rule for `→` and `↔` and also drops parentheses for a
right-nested `∧` or `∨` (`p ∧ (q ∧ r)` prints `p ∧ q ∧ r`), because those
operators are associative and the parentheses are not needed to read the
formula; the design keeps `p ↔ (q ↔ r)` explicit.

Alternatives: private fields with constructors or accessors (more code for a
tree that is pure data); derive `PartialEq` including positions (simpler, but
positions would leak into the Simplifier's structural equality); print a
grouping transparently in `ASTPrinter` (shorter, but two different trees print
identically); keep the right-nested `∧`/`∨` parentheses (uniform rule, but the
display disagrees with the simplifier's flattened output).

Consequences: structural comparisons in the Simplifier are sound; tests can
compare hand-built trees against parsed ones without reproducing positions;
`ASTPrinter` is a faithful tree dump; the infix form of `p ∧ (q ∧ r)` loses a
parenthesis the user wrote, which is harmless because the meaning is the same.

## ADR-041 — Parser details: owned tokens, kind predicates and a total cursor

Status: Accepted

Context: `parser.md` fixes the grammar, the rule results and the recovery
strategy, and ADR-025 fixes where a failing rule stops. Four implementation
details were still open. First, `ParseState` carries a token list; the scanner
owns its source (ADR-038), so the question is whether the parser copies the
tokens or borrows them. Second, the diagram gives `check`/`match` a
`TokenType` parameter, but `Identifier` and `Literal` carry payloads, so a
value to compare against does not exist. Third, whether `parse` consumes the
final `Eof` and how it behaves if the list has none. Fourth, whether the
missing-expression report points at the `Eof` just past the end (it does:
`p &` is `columna 4`).

Decision: `ParseState` owns `Vec<Token>`, so no helper carries a lifetime, and
`Parser::parse` takes the list by value. `check` and `match` take a predicate
over `&TokenType` (`matches!(kind, TokenType::Newline)`), which is total over
the payload variants. `match` returns the matched token and the state after it,
so the rules never need the book's `previous`. The cursor is total: `peek`
falls back to an `Eof` token at line 1, column 1 when the list runs out, and
`is_at_end` is true at `Eof` or past the end, so `parse` cannot read out of
bounds even if it is handed a hand-built list; a scanner-produced list is
unchanged because it always ends with `Eof`. `parse` does not consume `Eof`,
and `proposition` consumes the terminating `Newline`. A blank line is skipped
by the script loop. Every error is reported at the token under the cursor when
the rule fails.

Alternatives: borrow the tokens (`&[Token]` and a lifetime through every rule,
for no gain, since the scanner produces an owned list); compare tokens against
a sentinel `TokenType` value (invents a literal or an identifier and would
match the wrong kind); consume `Eof` (then the end of the script is observed
once and the loop needs a separate flag); panic on a malformed list (the
scanner's output is always well formed, but errors are values, never panics).

Consequences: the parser is as pure and lifetime-free as the scanner; the
diagram lists predicate helpers instead of `TokenType` arguments; tests can
call `parse` with any token list without risking a panic.

## ADR-042 — Simplifier details: rule order, chain matching and expansion order

Status: Accepted

Context: `simplifier.md` fixes the laws, the direction each is used in, the
canonical order and the shape of the expansion loop, but leaves the mechanics
open in four places that change the output. Several laws can match the same
node, a two-operand law has to look inside chains longer than two operands, the
expansion loop needs a deterministic traversal, and the new nodes the rules
create need an operator token the source never spelled.

Decision: The reduction rules are tried in the order of the law table —
idempotence, identity, domination, complement, absorption, redundancy,
conditional and biconditional elimination, then factoring (law 4 read right to
left). Absorption and redundancy match every ordered pair of operands of the
flattened chain, so `p | q | (p & s)` absorbs to `p | q`. Each rewrite is a
single step: a bottom-up, left-to-right scan applies the first match, the
result is canonicalized, and the scan restarts from the root until no rule
matches. The expansion phase visits nodes pre-order (a node before its
children, left operand first) and, at each node, tries law 4a before 4b and
the matching operands left to right; the first strict improvement wins and the
phase restarts from it. A distribution over a chain with other operands
distributes the rest of the chain too (`(p | s) | (q & r)` gives
`(p | s | q) & (p | s | r)`). Nodes the rules create use synthetic `!`, `&`
and `|` tokens at line 0, column 0; expression equality ignores positions
(ADR-040), so they are indistinguishable from parsed ones. `simplify` takes
`&Expression` (the orchestrator keeps the original to tabulate it), and the
weight saturates instead of overflowing, so a pathologically nested negation
cannot panic in a debug build.

Alternatives: try the "biggest" law first (no reason grounded in the document,
and it makes the output harder to predict); apply absorption and redundancy
only to an exact two-operand node (contradicts `simplifier.md`); visit the
expansion tree post-order (deterministic too, but the result is less
predictable from the text); distribute only when the outer chain has exactly
two operands (leaves many expansions untried); panic on the weight overflow
(errors are values; a panic over a legal, if silly, input is not).

Consequences: the output is deterministic and reproducible from the document,
and every rewrite is a single, testable step. The price is a canonicalization
per step, which is acceptable because the trees are small, and a weight that
saturates in contrived inputs, where the program then stops simplifying
further instead of misbehaving.

## ADR-043 — Interpreter and truth-table details: borrowed expression, public fields

Status: Accepted

Context: `interpreter.md` and `truth_table.md` fix the evaluation, the row
order, the size limit and the display contract, but leave four implementation
details open. `interpret(expression: Expression)` shows the tree by value even
though the Interpreter only reads it and the Orchestrator needs the same tree
for the result-column label. The `TruthTable` diagram marks the fields private
(`-variables`, `-rows`, `-label`, `-simplified`, and `Row`'s `-values`,
`-result`), while the UI lives in another module and must read them. The
`classify` pseudocode says "all true", "all false", "anything else", which is
ambiguous for the zero-row table that public fields make constructible even
though `interpret` always builds at least one row. Finally, an identifier
lookup has to do something when the name is missing, which the constructed
environment makes unreachable but the type system does not.

Decision: `Interpreter.interpret` takes `&Expression`, like
`Simplifier.simplify` (ADR-042), so the Orchestrator keeps one tree for both the
table and the label without cloning. `TruthTable` and `Row` expose their fields
as public plain data, as `Expression` does (ADR-040); the only methods stay
`with_display` and `classify`, and the diagrams mark the fields public. An empty
table — only constructible by hand — classifies as `Contingency`, so the two
"all" cases are never vacuously true at once. `evaluate` looks the name up in
the assignment map and falls back to false instead of indexing, keeping the
walk total and panic-free even on a malformed tree.

Alternatives: take the expression by value to match the pseudocode literally
(forces a clone in the Orchestrator); keep the fields private and add a getter
per field (encapsulation of data that the UI must read wholesale); classify an
empty table as a tautology by vacuous truth (nonsensical to show a user);
`unwrap` the lookup (a panic over an impossible input is still a panic).

Consequences: the Interpreter reads the tree without owning it, the UI can
present a table without an accessor layer, and no public operation panics.
Hand-built empty tables get a harmless, documented class rather than an
arbitrary one. The two field-visibility decisions are choices of this stage
and apply to `TruthTable`/`Row` only.

## ADR-044 — Orchestrator arguments are owned

Status: Accepted

Context: `orchestrator.md` gives `evaluate(source: str, simplify: bool,
table_simplified: bool)` and `report(error: Error): str`, which do not say how
Rust should pass the two owned values. The scanner takes its source by value
and stores it in `ScanState` (ADR-038), so borrowing the text in `evaluate`
would add a copy on every call. The UI prints each error from the list the
failed stage returned, one at a time.

Decision: Both entry points take their owned argument by value:
`evaluate(source: String, ...)` consumes the source and `report(error: Error)`
consumes the error. The UI moves the source it read into `evaluate` and each
error out of the list into `report`, so neither call copies the payload.

Alternatives: borrow both (`&str`/`&Error`) to match the expressions the other
stages take — rejected because the scanner immediately needs an owned copy of
the source and the error list is consumed by the caller anyway; borrow only the
source; borrow only the error.

Consequences: the signatures are the pseudocode's with Rust types, no call site
clones its payload, and `report` stays a thin `Display` wrapper. The asymmetry
with `interpret(&Expression)` and `simplify(&Expression)` is deliberate: those
stages read a tree the Orchestrator still needs, while the Orchestrator
consumes text and errors nobody reads again.

## ADR-045 — CLI layout details: free functions, alignment, numbering and separators

Status: Accepted

Context: `ui.md` fixes the modes, the flags, the presentation order and the
redirect rules, but leaves the mechanics of the last stage open in five places
that the user sees. Whether the CLI is a struct or a set of functions is not a
design question for the document. The table is "aligned" without saying how
(a column width, cell padding, separators, trailing spaces). File mode shows
several tables in sequence without saying how they are separated or numbered.
Interactive mode says the prompt comes again without saying where the blank
lines around a table or an error go. End-of-input and `exit` differ in what the
terminal shows.

Decision: The CLI is a set of free functions — `cli::run(config) -> i32`
dispatching on `Input`, private helpers for the file, batch and interactive
paths, and a thin `main` that only translates the exit code. In the table,
every cell is left-aligned to the width of the widest text of its column,
header included; columns are separated by two spaces, the result column is
introduced by `| `, and the last column is not padded, so no line has trailing
spaces. A table with no variables has no `|` separator: the label is the only
header and the rows hold the result. File mode numbers the tables it builds:
each block starts with `Proposición n:` (1-based, counting only built tables,
so blank lines do not advance the counter), then a blank line, then the normal
presentation, and blocks are separated by one blank line. Single-proposition
mode has no heading. In interactive mode, a presentation or an error is
followed by a blank line before the next prompt; a blank input line adds no
output of its own. End-of-input prints one newline after the pending `> `,
while `exit` and `salir` add none. Table cells are painted by temporarily
overriding `colored`'s terminal detection, since `colored` decides from stdout
and the cells may be written to stdout while the error color depends on stderr.

Alternatives: a `Cli` struct with methods (more ceremony for state that does
not outlive one call); right-aligning cells or padding every column (trailing
spaces and a table that looks ragged in a file); separating file-mode tables
with a blank line but no heading (the user loses track of which proposition is
which); numbering in every mode (redundant for one proposition, noisy in the
REPL); printing a newline on `exit` too (the shell prompt already starts on a
fresh line).

Consequences: the output is deterministic and diff-stable, file batches are
self-describing, and the REPL leaves a clean line at Ctrl-D. The layout rules
live in `ui.md` and are exercised through the binary; the CLI-only decisions
(no trailing spaces, numbering only in file mode) cost nothing to change later
because they are presentation, not data.

## ADR-046 — Simplified tables by default; `-s` prints only the simplified formula

Status: Accepted

Context: The product wants the simplified formula and the table of that
simplified formula to be the default, with `--no-simplificar` as the way to
tabulate the original. That inverts ADR-017 (original by default,
`--tabla-simplificada` for the simplified table) and makes
`--tabla-simplificada` redundant. A new `-s`/`--simplificar` mode was also
asked for: print only the simplified formula, with no table. The short `-s`
was already `--salida` (ADR-013), and the surface has not been published, so
it can change.

Decision: The default simplifies and builds the table from the simplified
expression: variables, rows, the 4096-row check and the result-column label
all come from it, and the `Simplificada:` line repeats that label.
`--no-simplificar` disables the Simplifier, tabulates the original and omits
the line. `-s`/`--simplificar` simplifies and prints only
`Simplificada: <formula>`, with no table and no classification; it works in
every mode, composes with `-o` and with file numbering, never triggers the
automatic redirect, and conflicts with `--no-simplificar`
(`'--simplificar' no se puede usar con '--no-simplificar'`). The output flag
moves to `-o`/`--salida`; its conflict message becomes
`'-o' no se puede usar con '--interactivo'`. `--tabla-simplificada` is
removed. `Orchestrator.evaluate` becomes `evaluate(source, simplify)` and
always tabulates the chosen tree; a new `Orchestrator.simplify(source)`
returns the normalized formulas and builds no tables, so the size limit does
not apply to `--simplificar`.

Alternatives: keep the original table by default and add only the new flag
(contradicts the request); keep both `-s` meanings (impossible); print the
table with `--simplificar` and let the flag only affect the table (not what
was asked); keep `--tabla-simplificada` as a deprecated alias (a redundant
flag with no user).

Consequences: the default output now shows the small, canonical table, and
`--no-simplificar` is the escape hatch that preserves the user's own formula
and variable order. The CLI surface breaks once, before publication. The
formula-only mode is the cheapest path through the pipeline (no interpreter,
no table, no size limit) and is available in the interactive session too.

## Errata

- ADR-001 lists `Orchestrator.evaluate(source, simplify)`; the signature gained
  `table_simplified` in ADR-017 and the stage result carries `List<Error>` per
  ADR-019. Its "nothing is constructed" means no stage is constructed; AST nodes
  and `TruthTable` are immutable values.
- ADR-001's "the diagrams show no constructors" means no stage constructors;
  `<<create>>` marks creation of immutable AST, `Error` and `TruthTable` values.
- ADR-002's "every fallible operation returns `Result<T>`; failure is
  `Err(List<Error>)`" is refined by ADR-019: stage entry points carry lists,
  internal helpers carry one `Error`. Parser rules are the exception: they carry
  the `ParseState` with the error (ADR-025), and `TokenType.map` carries a
  message string.
- ADR-019's "parser rules are the main case" of one-error helpers is superseded
  by ADR-025: parser rules now return `Result<T, (Error, ParseState)>` so
  synchronization resumes from the failing token. Stage results still carry
  `List<Error>`.
- ADR-007's context says the reduced formula has "fewer connectives"; ADR-023
  clarifies that the guarantee is a lower weight, not a lower connective count.
- ADR-009's context writes `!p and not q ∧ V`; `and` and `not` are not accepted
  spellings. The intended example is `!p & ¬q ∧ V`.
- ADR-009's "only the parentheses precedence requires" also covers
  associativity: the left child of `->` at equal precedence is parenthesized,
  so `(p -> q) -> r` keeps its parentheses while `p -> (q -> r)` does not.
- ADR-011's "at most 12 variables are accepted" applies to the expression chosen
  for the table; ADR-017 lets a 13-variable original be tabulated when
  `--tabla-simplificada` selects a simplified form with at most 12 variables.
- ADR-003 and ADR-006 write `Result<T>` as shorthand for `Result<T, List<Error>>`
  (ADR-019); `TokenType.map` is the exception, `Result<TokenType, str>`.
- ADR-011 writes `Err(EvalError(...))` as shorthand; stage results carry a
  list, so the form is `Err([EvalError(...)])` (ADR-019). Its "batch run" scope
  for the 1024-row redirect is widened by ADR-022 to every non-interactive run.
- ADR-012's lower-case rule covers error messages and prompts; the labels
  `Simplificada:` and `Esta proposición...` keep their capital initial
  (ADR-021).
- ADR-012 and ADR-013 list the flags without `--tabla-simplificada`; the flag
  surface was extended by ADR-017, and its conflicts and strict usage rules are
  in ADR-022. ADR-030 adds `-h`/`--help` and `-V`/`--version`, whose names stay
  English; the Spanish-flag rule covers modes, modifiers and output. ADR-022's
  usage line is replaced by the compact
  `uso: plogic [opciones] [proposición]` (ADR-032), which also carries the two
  informational flags.
- ADR-013's "bare argument with no flag" means no mode flag; modifiers may
  accompany it. Its `plogic ->` example is superseded by ADR-024: every
  dash-prefixed argument is an option, so that command is an unknown-flag error
  unless it appears after `--`.
- ADR-023's "strictly lower weight" applies whenever a rewrite fires; at a fixed
  point with no applicable rewrite the weight is equal, so the guarantee is
  "never greater, strictly lower when a rewrite applies".
- ADR-022's one-message rule for usage errors gains the fixed precedence of
  ADR-027: unknown flag, missing value, more than one mode, extra positional,
  no proposition, the two conflicts.
- ADR-024's "at most one positional after `--`" is refined by ADR-028: `--` may
  be trailing, it is never a flag value, and `plogic -p p --` is valid.
