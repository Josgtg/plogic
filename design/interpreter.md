# Plogic - Interpreter design

Role: evaluates a proposition under truth assignments and builds its truth
table (components diagram, steps 8–9). Pure; the only failure case is the
size limit below.

## Evaluation

`Interpreter.interpret(expression: Expression): Result<TruthTable, List<Error>>`
walks the AST recursively with an environment mapping variable names to truth
values. The Orchestrator chooses which expression it receives: the original by
default, or the simplified one with `--tabla-simplificada` (ADR-017). The
Interpreter itself does not know which one it is evaluating.

It evaluates each node as follows:

- `Literal(value)` yields `value`.
- `Identifier(name)` looks the name up in the environment. Every variable of
  the expression is in the environment by construction — the environment is
  built from the expression itself.
- `Unary` and `Binary` combine their operands with the connective's truth
  function, i.e. the "true when" column of the product description's
  connective table (negation, conjunction, disjunction, implication,
  biconditional).
- `Grouping` simply recurses.

## Building the table

1. Collect the distinct variables of the expression it received, in order of
   first appearance (an in-order walk of the AST). This is the column order,
   and with `--tabla-simplificada` it is the simplified expression's variable
   order, not the original's.
2. Enumerate all 2^n truth assignments in binary counting order: the last
   variable varies fastest. For `p`, `q` the rows are `F F`, `F V`, `V F`,
   `V V` — exactly the order of the product description's example table.
3. Evaluate the expression under each assignment and store the result.

The result is one `TruthTable` per proposition (see
truth_table.md): 2^n rows, one column per variable plus the result
column. The Interpreter creates the table with its variables and rows; the
Orchestrator later adds the display text through `TruthTable.with_display`
(ADR-020).

## Limits

A truth table of more than 4096 rows (13 or more variables) is refused
before any row is enumerated: `interpret` returns
`Err([EvalError("resultado muy largo: más de 4096 filas")])` (see
error_handling.md). With `n` distinct variables, the check is `n > 12`
(equivalently `2^n > 4096`, written so the count cannot overflow), so at most
12 variables are accepted. Because the variables come from the expression the
Interpreter receives, the default checks the original proposition and
`--tabla-simplificada` checks the simplified one (ADR-017).
