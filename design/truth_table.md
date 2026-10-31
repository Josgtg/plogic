# Plogic - TruthTable design

Role: the result for one proposition: every truth assignment and the
proposition's value under it. The Interpreter creates it; the UI presents
it. Nothing in it can fail.

## Contents

- The ordered variable names — one column per distinct variable, in order
  of first appearance.
- The rows — one per truth assignment (2^n), each holding the variables'
  values (in column order) and the proposition's value.
- The display text of the evaluated proposition, labelling the result
  column: the normalized form (see below), in the operand order the user
  wrote.
- Optionally, present only when the Simplifier ran: the display text of
  the simplified formula, in canonical operand order.

There is a single display form. It is infix, with the connectives `¬`,
`∧`, `∨`, `→`, `↔`, the literals `V` and `F`, and parentheses only where
precedence requires (see tokens.md). How the user spelled the proposition
(`y`, `no`, `0`, `->`) never shows. The label keeps the order and
grouping the user wrote; the simplified formula comes out in the canonical
order of the Simplifier (see simplifier.md). Both texts are produced by
`InfixPrinter` (see ui.md).

## Layout

One column per variable, then the result column; one row per assignment,
in binary counting order with the last variable varying fastest. For
`p -> q` this is exactly the product description's example, whose result
column is labelled `p → q`:

| p | q | p → q |
| --- | --- | --- |
| F | F | V |
| F | V | V |
| V | F | F |
| V | V | V |

## Classification

`classify(): Classification` derives the class of the proposition from its
result column:

- all values true → `Tautology`
- all values false → `Contradiction`
- anything else → `Contingency`

The classification is exact: it reads the column, not the simplified
formula. Whether it is shown to the user is decided at presentation time.
