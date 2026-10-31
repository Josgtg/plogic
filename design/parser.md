# Plogic - Parser design

Role: second pipeline stage. The parser turns the token list into the
abstract syntax tree: one `Expression` per proposition (batch parse). It is
a recursive-descent parser, one method per precedence level, following
Crafting Interpreters ch. 6. It runs only when scanning succeeded.

## Entry point and state

- `Parser.parse(tokens: List<Token>): Result<List<Expression>>` — static
  entry point; the parser is never constructed.
- The immutable `ParseState { tokens, current }` is threaded through every
  rule; each rule returns `Result<(Expression, ParseState)>` (one `Error`
  per rule-level result) and `ParseState` is returned anew.

## Grammar

The grammar follows the product description (five connectives):

```
script         -> proposition* ;
proposition    -> expression ( Newline | Eof ) ;
expression     -> biconditional ;
biconditional  -> implication ( Biconditional implication )* ;
implication    -> disjunction ( Implication implication )? ;
disjunction    -> conjunction ( Or conjunction )* ;
conjunction    -> unary ( And unary )* ;
unary          -> Not unary | primary ;
primary        -> Literal | Identifier | "(" expression ")" ;
```

One method per level: `expression`, `biconditional`, `implication`,
`disjunction`, `conjunction`, `unary`, `primary`, plus `proposition` for
the statement level. With one connective per precedence level, the book's
operator families collapse to single-operator levels. The class diagrams
still show the `Xor` / `Nand` / `Nor` families; those are pending removal.

## Precedence and associativity

Lowest to highest: biconditional, implication, disjunction, conjunction,
negation, primary — exactly the ladder in the product description. For
example, `p | q & s -> t <-> q | 0` parses as
`((p | (q & s)) -> t) <-> (q | 0)`.

- Implication is right-associative: the rule recurses into itself, so
  `p -> q -> r` means `p -> (q -> r)`.
- Every other binary level is left-associative (a loop), so
  `p <-> q <-> r` means `(p <-> q) <-> r`.

## The AST

Nodes are the `Expression` hierarchy: `Binary(left, right, operator:
Token)`, `Unary(operator, expression)`, `Grouping(expression)`,
`Literal(value: bool)` and `Identifier(name: str)`. `Expression` itself
stays abstract and method-free; the parser creates nodes, `ASTPrinter`
renders them in parenthesized prefix notation for verification, and
`InfixPrinter` renders them in the normalized infix form for display (see
truth_table.md).

Example: `!p & q` becomes `Binary(Unary(Not, Identifier("p")), And,
Identifier("q"))`, printed by `ASTPrinter` as `(∧ (¬ p) q)` and by
`InfixPrinter` as `¬p ∧ q`.

## Script loop and error recovery

`parse` is the book's statement loop: one proposition per iteration.

- Consecutive `Newline` tokens are empty propositions and are skipped; a
  proposition may also end at `Eof`, so a single proposition needs no
  trailing newline.
- On a parse error the parser records the `ParseError`, then
  `synchronize`: a pure `ParseState -> ParseState` step that skips tokens
  until just after the next `Newline`, or until `Eof`. Parsing then
  continues with the next proposition, so as many errors as possible are
  reported.
- At the end of the loop the stage result is `Err(errors)` — the partial
  expression list is discarded — or `Ok(expressions)`. This is a stage
  gate: on `Err` nothing is simplified or evaluated.

Typical messages: `se esperaba una expresión`,
`se esperaba ')' después de la expresión`, `se esperaba el fin de la
proposición` (a proposition did not end at `Newline` or `Eof`).
