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
- Simplifying a proposition.
- Producing the truth table of each proposition.
- Writing the output to a file — on request, or automatically when a truth
  table is long.
- Spanish UI.

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
where precedence requires them, so the example above needs none: it parses
exactly as it reads. When grouping does matter they appear — `(p o q) y r`
displays as `(p ∨ q) ∧ r`, because without them `∧` would bind to `q` first.


In this project, a proposition is composed of literals, connectives and
identifiers:

- A **literal** (Boolean constant) is a truth value written directly, as
  one of the four spellings `0`, `1`, `F`, `V`. `0` and `F` mean false;
  `1` and `V` mean true.
- An **identifier** is a propositional variable: a name such as `p`, `q`, `s`
  or `t` that stands for an arbitrary truth value. The truth table assigns
  every combination of values to the variables that appear in the proposition.
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
**contingency**.

## Simplification

Before evaluation, Plogic by default simplifies the proposition (can be turned off) using the laws of
Boolean algebra, such as De Morgan's laws, double negation, idempotence,
absorption, identity (`p & V ≡ p`), domination (`p & F ≡ F`) and
distributivity. Simplification is an equivalence transformation: the reduced
formula has the same truth table as the original, but fewer connectives.

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
