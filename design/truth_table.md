# Plogic - TruthTable design

Role: the result for one proposition: every truth assignment and the
proposition's value under it. The Interpreter creates the table with its
variables and rows; the Orchestrator adds the display text through
`with_display`; the UI presents it. Nothing in it can fail.

## Contents

- The ordered variable names — one column per distinct variable of the
  evaluated expression (the original by default, the simplified one with
  `--tabla-simplificada`), in order of first appearance.
- The rows — one per truth assignment (2^n), each holding the variables'
  values (in column order) and the value of the evaluated expression.
- The display text of the evaluated expression, labelling the result
  column: the normalized form (see below). It keeps the operand order the
  user wrote when the table is the original, and the canonical order when
  the table is the simplified formula.
- Optionally, present only when the Simplifier ran: the display text of
  the simplified formula, in canonical operand order. With
  `--tabla-simplificada` it repeats the result-column label.

`with_display(label: str, simplified: str?): TruthTable` returns a new table
with the two display fields set; the Interpreter's table starts with them
empty. There are no setters (ADR-020). The variables, rows, label and
simplified text are public plain data, like the AST's fields (ADR-043), so the
UI can read them without accessors.

There is a single display form. It is infix, with the connectives `¬`,
`∧`, `∨`, `→`, `↔`, the literals `V` and `F`, and parentheses only where
precedence or associativity requires (see tokens.md). How the user spelled
the proposition (`y`, `no`, `0`, `->`) never shows. The label of the original
proposition keeps the order and grouping the user wrote; the simplified
formula comes out in the canonical order of the Simplifier (see
simplifier.md). With `--tabla-simplificada` the label is the simplified
formula, so it uses canonical order too. Both texts are produced by
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
formula. It is always shown, in every mode and with or without
simplification (ADR-021). A table always has at least one row, since
`interpret` builds 2^0 = 1 for a constant; an empty table, constructible only
by hand, is a contingency (ADR-043).
