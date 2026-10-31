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
