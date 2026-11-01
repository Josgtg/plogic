# Plogic - Parser design

Role: second pipeline stage. The parser turns the token list into the
abstract syntax tree: one `Expression` per proposition (batch parse). It is
a recursive-descent parser, one method per precedence level, following
Crafting Interpreters ch. 6. It runs only when scanning succeeded.

## Entry point and state

- `Parser.parse(tokens: List<Token>): Result<List<Expression>, List<Error>>`
  — static entry point; the parser is never constructed. It takes the token
  list by value: `ParseState` owns it, as the scanner's state owns the source,
  so no helper carries a lifetime (ADR-038, ADR-041).
- The immutable `ParseState { tokens, current }` is threaded through every
  rule; each rule returns
  `Result<(Expression, ParseState), (Error, ParseState)>` and `ParseState` is
  returned anew. On failure the second component is the state at the failing
  token, so `parse` can call `synchronize` from exactly there. The stage entry
  point collects rule errors into the `List<Error>` it returns.
- `check` and `match` take a predicate over the token kind rather than a
  `TokenType` value, because `Identifier` and `Literal` carry payloads;
  `match` returns the matched token, so no `previous` helper is needed.
- The cursor is total: past the end of the list `peek` returns an `Eof` and
  `is_at_end` is true, so a malformed list cannot make a rule index out of
  bounds. `parse` never consumes `Eof`; a proposition consumes its terminating
  `Newline`, and a blank line is skipped by the script loop (ADR-041).

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
operator families collapse to single-operator levels.

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

The node fields are public and equality is structural over the node shape and
the operators' kinds, not their positions: the Simplifier compares subformulas
and its canonical order must not depend on where an operator was typed
(ADR-040). `ASTPrinter` prints a `Grouping` as `(group …)`, so the verification
output shows the tree exactly as parsed, and `InfixPrinter` prints a grouping
transparently, keeping only the parentheses precedence or associativity
requires. In particular `(p | q) & r` keeps them, `(p)` loses them, and a
right-nested `∧` or `∨` loses them too (`p & (q & r)` prints `p ∧ q ∧ r`).

Example: `!p & q` becomes
`Binary(Unary(Not, Identifier("p")), Identifier("q"), And)`, printed by
`ASTPrinter` as `(∧ (¬ p) q)` and by `InfixPrinter` as `¬p ∧ q`.

## Script loop and error recovery

`parse` is the book's statement loop: one proposition per iteration.

- Consecutive `Newline` tokens are empty propositions and are skipped; a
  proposition may also end at `Eof`, so a single proposition needs no
  trailing newline.
- On a parse error the parser records the `ParseError`, then
  `synchronize`: a pure `ParseState -> ParseState` step that skips tokens
  until just after the next `Newline`, or until `Eof`. It starts from the
  state carried by the failing rule, which points at the failing token.
  Parsing then continues with the next proposition, so as many errors as
  possible are reported.
- At the end of the loop the stage result is `Err(errors)` — the partial
  expression list is discarded — or `Ok(expressions)`. This is a stage
  gate: on `Err` nothing is simplified or evaluated.

Typical messages: `se esperaba una expresión`,
`se esperaba ')' después de la expresión`, `se esperaba el fin de la
proposición` (a proposition did not end at `Newline` or `Eof`).

`v` scans as a variable name, not as a connective, so `p v q` leaves `v`
where an operator belongs. At that point — a leftover token after a complete
expression, or a `v` before a required `)` — the parser replaces the generic
message with `'v' no es un operador; usa '|', '∨' u 'o' para la disyunción`
(ADR-050). Any other leftover token keeps the message that says what was
expected, and `v` in operand position (`p & v`) stays a legal variable.
