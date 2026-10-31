# Plogic - Interpreter design

Role: evaluates a proposition under truth assignments and builds its truth
table (components diagram, steps 8–9). Pure; the only failure case is the
size limit below.

## Evaluation

`Interpreter.interpret(expression: Expression): Result<TruthTable>` walks the AST
recursively with an environment mapping variable names to truth values:

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

1. Collect the distinct variables of the expression, in order of first
   appearance (an in-order walk of the AST).
2. Enumerate all 2^n truth assignments in binary counting order: the last
   variable varies fastest. For `p`, `q` the rows are `F F`, `F V`, `V F`,
   `V V` — exactly the order of the product description's example table.
3. Evaluate the expression under each assignment and store the result.

The result is one `TruthTable` per proposition (see
truth_table.md): 2^n rows, one column per variable plus the result
column.

## Limits

A truth table of more than 4096 rows (13 or more variables) is refused
before any row is enumerated: `interpret` returns
`Err(EvalError("resultado muy largo: más de 4096 filas"))` (see
error_handling.md). The check is `2^n > 4096`, so at most 12 variables are
accepted.
