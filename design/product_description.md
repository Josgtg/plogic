# Plogic - Product Description

Plogic is a propositional logic evaluator. It reads propositions written in a
small formal language, simplifies them, and evaluates them, producing a truth
table for each one. Internally it tokenizes the source, parses it into an
abstract syntax tree (AST), simplifies the tree, and evaluates the result for
every possible truth assignment.

This project follows the book [Crafting Interpreters](https://craftinginterpreters.com/)
by Robert Nystrom, implementing a functional programming style rather than pure OOP.

## In scope

- Evaluating a single proposition.
- Evaluating a file, with one proposition per line.
- Interactive mode: write a proposition and get its truth table immediately.
- A graphical window, opened by running `plogic` with no arguments, that
  evaluates one proposition at a time.
- Simplifying a proposition.
- Producing the truth table of each proposition, optionally from its
  simplified form.
- Writing the output to a file — on request, or automatically when a truth
  table is long.
- Spanish UI.

## Graphical window

Running `plogic` with no arguments opens a dark, monospace window for one
proposition at a time. The whole window scrolls as one column: its centered
components — the simplified (or original) formula, the optional process list, a
real truth table and the classification — flow directly in the main window, not
in a result box. The collapsed help lists the accepted connectives and
literals. The `Simplificar`, `Mostrar tabla` and `Mostrar proceso` controls are
all read when `Evaluar` is pressed: with `Mostrar tabla` off the table is not
processed, and with it on the table is built only for fewer than ten
variables; the classification appears only with a table. The toolbar inserts
connectives, parentheses and variables. Editing the formula does not replace
the previous result: only `Evaluar` does. Any argument keeps the
command-line interface.

## What is a proposition?

A proposition is a statement that is either true or false, written here as a
well-formed formula of propositional logic. A formula is built from
propositional variables, literals, logical connectives, and parentheses.

An example of a proposition is the following:

`((p o (q y s)) entonces t) sisolosi (q o 0)`

which is the same as:

`((p | (q & s)) -> t) <-> (q | 0)`

which can be read as _"p or q and s implies t if and only if q or false"_.

Plogic displays it as:

`p ∨ q ∧ s → t ↔ q ∨ F`

Whatever the spelling, the display is normalized: formal symbols, `V` / `F`
literals and the operand order the user wrote. Parentheses are kept only
where precedence or associativity requires them, so the example above needs
none: it parses exactly as it reads. When grouping does matter they appear —
`(p o q) y r` displays as `(p ∨ q) ∧ r`, because without them `∧` would bind
to `q` first; `(p -> q) -> r` keeps its parentheses, while `p -> (q -> r)`
prints as `p → q → r`, because implication is right-associative. The result
column is labelled with the expression being tabulated: the simplified
formula in canonical order by default, or the original proposition with
`--no-simplificar`; the `Simplificada:` line shows that same canonical
formula (see simplifier.md).

In this project, a proposition is composed of literals, connectives and
identifiers:

- A **literal** (Boolean constant) is a truth value written directly, as
  one of the four spellings `0`, `1`, `F`, `V`. `0` and `F` mean false;
  `1` and `V` mean true.
- An **identifier** is a propositional variable: a name such as `p`, `q`, `s`
  or `t` that stands for an arbitrary truth value. An identifier starts with
  an ASCII letter (`a`–`z`, `A`–`Z`) and continues with ASCII letters,
  digits or underscores; an underscore never starts one, so `_p` is invalid.
  Names are case-sensitive, so `P` and `p` are different variables.
  The words used as connectives (`no`, `y`, `o`, `entonces`, `sisolosi`) and
  the literals `V` and `F` are reserved and cannot be identifiers. The truth
  table assigns every combination of values to the variables that appear in
  the expression being tabulated.
- A **connective** (operator) combines one or more propositions into a larger
  one. Each connective can be written as a symbol or as a word. The supported
  connectives are:

  | Connective | Symbols | Word | Arity | True when |
  | --- | --- | --- | --- | --- |
  | Negation | `!`, `¬` | `no` | unary | its operand is false |
  | Conjunction | `&`, `∧` | `y` | binary | both operands are true |
  | Disjunction | `\|`, `∨` | `o` | binary | at least one operand is true |
  | Implication | `->`, `→` | `entonces` | binary | the antecedent is false or the consequent is true |
  | Biconditional | `<->`, `↔` | `sisolosi` | binary | both operands have the same truth value |

- **Parentheses** `(` and `)` group a subexpression and override the default
  precedence.

## Precedence and associativity

Connectives are listed from lowest to highest precedence:

1. Biconditional `<->` / `↔` / `sisolosi` — left-associative
2. Implication `->` / `→` / `entonces` — right-associative
3. Disjunction `|` / `∨` / `o` — left-associative
4. Conjunction `&` / `∧` / `y` — left-associative
5. Negation `!` / `¬` / `no` — unary, binds tightest
6. Parentheses, literals and identifiers

Because implication is right-associative, `p -> q -> r` means
`p -> (q -> r)`. Biconditional and the other binary connectives are
left-associative, so `p <-> q <-> r` means `(p <-> q) <-> r`. Parentheses can
always be used to make the grouping explicit.

## Truth tables

For each proposition, Plogic collects the distinct variables, enumerates every
truth assignment (2^n rows for n variables), and evaluates the proposition
under each assignment. The table has one column per variable and one column
for the proposition itself. For example, `p -> q` produces:

| p | q | p → q |
| --- | --- | --- |
| F | F | V |
| F | V | V |
| V | F | F |
| V | V | V |

A proposition whose column contains only `V` is a **tautology**; one whose
column contains only `F` is a **contradiction**; anything else is a
**contingency**. Plogic always reports the classification.

The variables, rows and 4096-row limit come from the expression being
tabulated. By default that is the simplified expression, so the columns can
shrink (a formula that simplifies to `V` or `F` has no variable columns at
all); with `--no-simplificar` the original proposition is tabulated instead.

## Simplification

Before evaluation, Plogic by default simplifies the proposition (can be turned
off with `--no-simplificar`) using the laws of Boolean algebra, such as De
Morgan's laws, double negation, idempotence, absorption, identity
(`p & V ≡ p`), domination (`p & F ≡ F`) and distributivity. Simplification is
an equivalence transformation: the reduced formula has the same truth table
as the original and no greater weight, strictly lower whenever a rewrite
applies, though not necessarily fewer connectives (De Morgan and the
biconditional expansion can add some). By default the truth table is built
from the reduced formula and the formula is shown as a `Simplificada:` line;
`--no-simplificar` builds the table from the original and omits that line.
`-s` / `--simplificar` simplifies and prints only the reduced formula, with
no table. `-m` / `--mostrar-proceso` adds the ordered list of laws that were
applied, each with the formula it produced, so the derivation can be followed
step by step.

## Errors

A mistake in a proposition is reported in Spanish, one error per line, with
its position:

`línea 2, columna 7: se esperaba ')' después de la expresión`

Plogic reports as many errors as it can find in one run, but a run with
errors produces no truth tables. A file that contains no propositions at all
is reported as an error too: `el archivo no contiene proposiciones`.

## Out of scope

- Multi-line propositions: a proposition ends at the end of its line.
- Other connectives, such as XOR, NAND and NOR.
- First-order logic: no quantifiers, predicates or functions.
- Values other than truth values: no numbers, strings or arithmetic.
- More languages in UI.
